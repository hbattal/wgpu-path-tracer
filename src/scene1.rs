use guillotiere::{AtlasAllocator, size2};
use include_bytes_aligned::include_bytes_aligned;
use wgpu::util::DeviceExt;

use crate::bvh::*;
use crate::material::PBRMaterialGPU;
use crate::object::*;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Scene {}

impl Scene {
    pub fn fast_model(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> (
        &'static [u8],
        &'static [u8],
        &'static [u8],
        &'static [u8],
        wgpu::BindGroupLayout,
        wgpu::BindGroup,
    ) {
        let bytes_linear = include_bytes_aligned!(16, "../precomp/bvh");
        let bytes_spheres = include_bytes_aligned!(16, "../precomp/spheres");
        let bytes_triangle = include_bytes_aligned!(16, "../precomp/triangles");
        let bytes_mats = include_bytes_aligned!(16, "../precomp/mats");

        let (layout, group) = Scene::pre_images(&device, &queue);

        (
            bytes_linear,
            bytes_spheres,
            bytes_triangle,
            bytes_mats,
            layout,
            group,
        )
    }

    pub fn test_model(device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut world: Vec<Rc<dyn Hittable>> = Vec::new();
        let mut linear: Vec<BvhGPU> = Vec::new();

        let mut world_spheres: Vec<SphereGPU> = Vec::new();
        let mut world_triangle: Vec<TriangleGPU> = Vec::new();
        let mut world_mats: Vec<PBRMaterialGPU> = Vec::new();

        let bytes = include_bytes!("../models/p6.glb");

        Scene::load_gltf(bytes, &mut world, &mut world_triangle, &mut world_mats);

        world.push(Rc::new(Sphere::new(
            glam::vec3(0.0, -4000., -1.0),
            4000.0,
            world_spheres.len() as u32,
        )));

        world_spheres.push(SphereGPU::new(
            glam::vec3(0.0, -4000., -1.0),
            4000.0,
            world_mats.len() as u32,
        ));

        //ground mat
        world_mats.push(PBRMaterialGPU::new(
            glam::Vec4::from_array([0.2, 0.2, 0.2, 1.0]),
            -1,
            -1,
            1.0,
            0.1,
            glam::Vec3::from_array([0.0, 0.0, 0.0]).extend(1.0),
            -1,
            0.0,
            -1,
            -1,
            -1,
        ));

        world.push(Rc::new(Sphere::new(
            glam::vec3(0.0, 600., -1.0),
            400.0,
            world_spheres.len() as u32,
        )));

        world_spheres.push(SphereGPU::new(
            glam::vec3(0.0, 600., -1.0),
            400.0,
            world_mats.len() as u32,
        ));

        world.push(Rc::new(Sphere::new(
            glam::vec3(0.0, 600., -1200.0),
            400.0,
            world_spheres.len() as u32,
        )));

        world_spheres.push(SphereGPU::new(
            glam::vec3(0.0, 600., -1200.0),
            400.0,
            world_mats.len() as u32,
        ));

        //ground mat
        world_mats.push(PBRMaterialGPU::new(
            glam::Vec4::from_array([0.0, 0.0, 0.0, 1.0]),
            -1,
            -1,
            0.0,
            0.0,
            glam::Vec3::from_array([4.0, 4.0, 4.0]).extend(1.0),
            -1,
            0.0,
            -1,
            -1,
            -1,
        ));

        let root_bvh = BvhNode::new(world);
        root_bvh.shader_format(&mut linear);

        println!("ended");

        let _ = std::fs::write("bvh", bytemuck::cast_slice(&linear));
        let _ = std::fs::write("spheres", bytemuck::cast_slice(&world_spheres));
        let _ = std::fs::write("triangles", bytemuck::cast_slice(&world_triangle));
        let _ = std::fs::write("mats", bytemuck::cast_slice(&world_mats));
    }

    //currently no error handling at all
    //https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html

    //GLTF TODO:

    //samplers
    //textures in web, binding array is not supported
    //error handling
    //proper image formats

    //resources (bevy is good)
    //https://github.com/bevyengine/bevy/blob/e8b3598ff5e5ec40e8ba84edd5750a1c0e4d4e59/crates/bevy_image/src/image_texture_conversion.rs#L9
    //https://github.com/bevyengine/bevy/blob/main/crates/bevy_gltf/src/loader/gltf_ext/mod.rs#L46

    fn load_gltf(
        bytes: &[u8],
        world: &mut Vec<Rc<dyn Hittable>>,
        world_triangle: &mut Vec<TriangleGPU>,
        world_mats: &mut Vec<PBRMaterialGPU>,
    ) {
        let (document, buffers, images) = gltf::import_slice(bytes).unwrap();

        println!("{:?}", document.extensions_used());
        println!("{:?}", document.extensions_required());

        let s = glam::Mat4::from_scale(glam::Vec3::splat(10.));
        let t = glam::Mat4::from_translation(glam::Vec3::new(0., 6.2, 0.));

        let init = t * s; //idk why some models do this

        for scene in document.scenes() {
            for node in scene.nodes() {
                Scene::node(node, init, &buffers, world, world_triangle);
            }
        }

        let mut linear = HashMap::new();

        for mat in document.materials() {
            let pbr = mat.pbr_metallic_roughness();

            let color_factor = pbr.base_color_factor();
            let metal_factor = pbr.metallic_factor();
            let rough_factor = pbr.roughness_factor();
            let emiss_factor = glam::Vec3::from_array(mat.emissive_factor())
                * mat.emissive_strength().unwrap_or(1.0);

            //println!("{:?}, and {:?}", mat.alpha_mode(), mat.alpha_cutoff());

            let mut color = -1;
            let mut color_sampler = -1;

            if let Some(info) = pbr.base_color_texture() {
                color = info.texture().source().index() as i32;

                color_sampler = info
                    .texture()
                    .sampler()
                    .index()
                    .map_or(-1, |ind| ind as i32);
            }

            if color != -1 {
                linear.insert(color, true);
            }

            let mut metal_rough = -1;
            let mut metal_rough_sampler = -1;

            if let Some(info) = pbr.metallic_roughness_texture() {
                metal_rough = info.texture().source().index() as i32;

                metal_rough_sampler = info
                    .texture()
                    .sampler()
                    .index()
                    .map_or(-1, |ind| ind as i32);
            }

            if metal_rough != -1 {
                linear.insert(metal_rough, false);
            }

            let mut emiss = -1;
            let mut emiss_sampler = -1;

            if let Some(info) = mat.emissive_texture() {
                emiss = info.texture().source().index() as i32;

                emiss_sampler = info
                    .texture()
                    .sampler()
                    .index()
                    .map_or(-1, |ind| ind as i32);
            }
            if emiss != -1 {
                linear.insert(emiss, true);
            }

            let ior = mat.ior().map_or(1.5, |ior| ior);

            world_mats.push(PBRMaterialGPU::new(
                glam::Vec4::from_array(color_factor),
                color,
                metal_rough,
                metal_factor,
                rough_factor,
                emiss_factor.extend(1.0),
                emiss,
                ior,
                color_sampler,
                metal_rough_sampler,
                emiss_sampler,
            ));
        }

        let mut atlas = AtlasAllocator::new(size2(8192, 8192));
        let mut bounds: Vec<Bounds> = Vec::new();
        let mut image_data = Vec::new();
        let mut offsets = Vec::new();

        for (_ind, image) in images.iter().enumerate() {
            //do formating for 3 value images?

            //println!("{} and {}", image.width, image.height);

            let a = atlas
                .allocate(size2(image.width as i32, image.height as i32))
                .unwrap();

            println!("{:?}", a);

            //println!("{:?}", image.format);

            bounds.push(Bounds {
                x: a.rectangle.min.x as u32,
                y: a.rectangle.min.y as u32,
                w: a.rectangle.width() as u32,
                h: a.rectangle.height() as u32,
            });

            let mut data = Vec::new();

            //I dont understand why we have r8? if i use  wgpu::TextureFormat::R8Unorm then the gpu needs to do something different or I fix it here? wtf?
            match image.format {
                gltf::image::Format::R8 => {
                    for &i in &image.pixels {
                        data.extend_from_slice(&[i, i, i, 255]);
                    }
                }

                gltf::image::Format::R8G8B8 => {
                    for i in image.pixels.chunks(3) {
                        data.extend_from_slice(&[i[0], i[1], i[2], 255]);
                    }
                }

                //https://docs.rs/image/latest/src/image/color.rs.html#740
                gltf::image::Format::R8G8 => {
                    for i in image.pixels.chunks(2) {
                        data.extend_from_slice(&[i[0], i[0], i[0], i[1]]);
                    }
                }

                gltf::image::Format::R8G8B8A8 => {
                    data = image.pixels.clone();
                }
                _ => todo!(),
            }

            offsets.push(image_data.len() as u32); //might
            image_data.extend_from_slice(&data);

            // let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            // views.push(view);
        }

        let _ = std::fs::write("bounds", bytemuck::cast_slice(&bounds));
        let _ = std::fs::write("images", bytemuck::cast_slice(&image_data));
        let _ = std::fs::write("offsets", bytemuck::cast_slice(&offsets));
    }

    fn node(
        node: gltf::Node,
        parent: glam::Mat4,
        buffers: &Vec<gltf::buffer::Data>,
        world: &mut Vec<Rc<dyn Hittable>>,
        world_triangle: &mut Vec<TriangleGPU>,
    ) {
        let matr = parent * glam::Mat4::from_cols_array_2d(&node.transform().matrix());
        let matr_norm = matr.inverse().transpose();

        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let mut vertices: Vec<Vertex> = Vec::new();
                let mat = primitive.material().index().unwrap() as u32;

                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let pos: Vec<_> = reader.read_positions().unwrap().collect();
                let normal: Vec<_> = reader.read_normals().unwrap().collect();
                let uv: Vec<_> = reader.read_tex_coords(0).unwrap().into_f32().collect();

                for ind in 0..pos.len() {
                    vertices.push(Vertex {
                        pos: matr.transform_point3(glam::Vec3::from_array(pos[ind])),
                        normal: matr_norm
                            .transform_vector3(glam::Vec3::from_array(normal[ind]))
                            .normalize(), //TEST
                        uv: glam::Vec2::from_array(uv[ind]),
                    });
                }

                let indices: Vec<u32> = reader.read_indices().unwrap().into_u32().collect(); //why do i need to specify type?

                for ind in indices.chunks(3) {
                    let v0 = ind[0] as usize;
                    let v1 = ind[1] as usize;
                    let v2 = ind[2] as usize;

                    //println!("{} and {}", vertices.len(), v1);

                    world.push(Rc::new(Triangle::new(
                        vertices[v0].pos.extend(1.0),
                        vertices[v1].pos.extend(1.0),
                        vertices[v2].pos.extend(1.0),
                        world_triangle.len() as u32,
                    )));

                    world_triangle.push(TriangleGPU::new(
                        vertices[v0].pos.extend(1.0),
                        vertices[v1].pos.extend(1.0),
                        vertices[v2].pos.extend(1.0),
                        vertices[v0].normal.extend(1.0),
                        vertices[v1].normal.extend(1.0),
                        vertices[v2].normal.extend(1.0),
                        vertices[v0].uv,
                        vertices[v1].uv,
                        vertices[v2].uv,
                        mat,
                    ));
                }
            }
        }

        for child in node.children() {
            Scene::node(child, matr, &buffers, world, world_triangle);
        }
    }

    //srgb is broken rn
    fn pre_images(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let bytes_bounds = include_bytes_aligned!(16, "../precomp/bounds");
        let bytes_offsets = include_bytes_aligned!(16, "../precomp/offsets");

        let bounds: &[Bounds] = bytemuck::cast_slice(bytes_bounds);
        let offsets: &[u32] = bytemuck::cast_slice(bytes_offsets);

        let images = include_bytes_aligned!(16, "../precomp/images");

        let size = wgpu::Extent3d {
            width: 8192,
            height: 8192,
            depth_or_array_layers: 1,
        };

        let tex: wgpu::Texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: None,
            view_formats: &[],
        });

        for i in 0..offsets.len() {
            let bound = bounds[i];

            let ofst = offsets[i] as usize;
            let nxt = if i == offsets.len() - 1 {
                images.len()
            } else {
                offsets[i + 1] as usize
            };

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: bound.x as u32,
                        y: bound.y as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &images[ofst..nxt],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * bound.w),
                    rows_per_image: Some(bound.h),
                },
                wgpu::Extent3d {
                    width: bound.w,
                    height: bound.h,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());

        let tex_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bounds_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sphere_buf"),
            contents: bytemuck::cast_slice(&bounds),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let tex_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &tex_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bounds_buffer.as_entire_binding(),
                },
            ],
        });

        (tex_bind_layout, tex_bind)
    }
}

pub struct Vertex {
    pos: glam::Vec3,
    normal: glam::Vec3,
    uv: glam::Vec2,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Bounds {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}
