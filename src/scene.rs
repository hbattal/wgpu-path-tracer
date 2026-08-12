use crate::bvh::*;
use crate::material::PBRMaterialGPU;
use crate::object::*;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Scene {}

impl Scene {
    pub fn test_model(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> (
        Vec<BvhGPU>,
        Vec<SphereGPU>,
        Vec<TriangleGPU>,
        Vec<PBRMaterialGPU>,
        wgpu::BindGroupLayout,
        wgpu::BindGroup,
    ) {
        let mut world: Vec<Rc<dyn Hittable>> = Vec::new();
        let mut linear: Vec<BvhGPU> = Vec::new();

        let mut world_spheres: Vec<SphereGPU> = Vec::new();
        let mut world_triangle: Vec<TriangleGPU> = Vec::new();
        let mut world_mats: Vec<PBRMaterialGPU> = Vec::new();

        let (layout, group) = Scene::load_gltf(
            "models/p3.glb",
            &mut world,
            &mut world_triangle,
            &mut world_mats,
            &device,
            &queue,
        );

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
            0.12,
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

        (
            linear,
            world_spheres,
            world_triangle,
            world_mats,
            layout,
            group,
        )
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

    pub fn load_gltf(
        path: &str,
        world: &mut Vec<Rc<dyn Hittable>>,
        world_triangle: &mut Vec<TriangleGPU>,
        world_mats: &mut Vec<PBRMaterialGPU>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let (document, buffers, images) = gltf::import(path).unwrap();

        println!("{:?}", document.extensions_used());
        println!("{:?}", document.extensions_required());

        let init = glam::Mat4::from_scale(glam::Vec3::splat(1000.)); //idk why some models do this

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

        let mut views: Vec<wgpu::TextureView> = Vec::new();
        let mut samplers = Vec::new();

        for (ind, image) in images.iter().enumerate() {
            //do formating for 3 value images?

            //println!("{} and {}", image.width, image.height);

            let size = wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            };

            //println!("{:?}", image.format);

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

                gltf::image::Format::R8G8B8A8 => {
                    data = image.pixels.clone();
                }
                _ => todo!(),
            }

            let format = if let Some(true) = linear.get(&(ind as i32)) {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            };

            let tex = device.create_texture(&wgpu::TextureDescriptor {
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: None,
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * image.width),
                    rows_per_image: Some(image.height),
                },
                size,
            );

            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            views.push(view);
        }

        //umm im just noticing that mipmaps make no sense in the context of a path tracer
        for sampler in document.samplers() {
            use gltf::texture::MagFilter::{Linear, Nearest};
            use gltf::texture::MinFilter;
            use gltf::texture::WrappingMode::{ClampToEdge, MirroredRepeat, Repeat};

            let address_mode = |typ| match typ {
                ClampToEdge => wgpu::AddressMode::ClampToEdge,
                MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
                Repeat => wgpu::AddressMode::Repeat,
            };

            let address_mode_u = address_mode(sampler.wrap_s());
            let address_mode_v = address_mode(sampler.wrap_t());

            let mag_filter = match sampler.mag_filter() {
                Some(Nearest) => wgpu::FilterMode::Nearest,
                Some(Linear) => wgpu::FilterMode::Linear,
                None => wgpu::FilterMode::Linear,
            };

            let min_filter = match sampler.min_filter() {
                Some(MinFilter::Nearest) => wgpu::FilterMode::Nearest,
                Some(MinFilter::NearestMipmapLinear) => wgpu::FilterMode::Nearest,
                Some(MinFilter::NearestMipmapNearest) => wgpu::FilterMode::Nearest,
                _ => wgpu::FilterMode::Linear,
            };

            let smp = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u,
                address_mode_v,
                mag_filter,
                min_filter,
                ..Default::default()
            });

            samplers.push(smp);
        }

        let tex_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: std::num::NonZeroU32::new(views.len() as u32),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,

                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: std::num::NonZeroU32::new(samplers.len() as u32),
                },
            ],
        });

        let refs: Vec<_> = views.iter().collect();
        let refs_s: Vec<_> = samplers.iter().collect();

        let tex_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &tex_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&refs),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::SamplerArray(&refs_s),
                },
            ],
        });

        (tex_bind_layout, tex_bind)
    }

    pub fn node(
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
}

pub struct Vertex {
    pos: glam::Vec3,
    normal: glam::Vec3,
    uv: glam::Vec2,
}
