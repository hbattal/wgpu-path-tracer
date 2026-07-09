use rand::random_range;

use crate::bvh::*;
use crate::material::MaterialGPU;
use crate::object::*;
use std::io::Cursor;
use std::rc::Rc;

pub struct Scene {}

impl Scene {
    pub fn test() -> (Vec<BvhGPU>, Vec<SphereGPU>, Vec<TriangleGPU>) {
        let mut world: Vec<Rc<dyn Hittable>> = Vec::new();
        let mut linear: Vec<BvhGPU> = Vec::new();

        let mut world_spheres: Vec<SphereGPU> = Vec::new();
        let mut world_triangle: Vec<TriangleGPU> = Vec::new();
        //let mut world_mats: Vec<MaterialGPU> = Vec::new();

        world.push(Rc::new(Sphere::new(
            glam::vec3(0.0, -100.5, -1.0),
            100.0,
            world_spheres.len() as u32,
        )));

        world_spheres.push(SphereGPU::new(glam::vec3(0.0, -100.5, -1.0), 100.0, 1));

        for i in 0..400 {
            let loc = glam::vec3(
                random_range(-10.0..10.0),
                random_range(0.0..10.0),
                random_range(-10.0..10.0),
            );

            let rad = random_range(0.1..0.6);

            let loc2 = loc
                + glam::vec3(
                    random_range(-1.0..1.0),
                    random_range(-1.0..1.0),
                    random_range(-1.0..1.0),
                );
            let loc3 = loc2
                + glam::vec3(
                    random_range(-1.0..1.0),
                    random_range(-1.0..1.0),
                    random_range(-1.0..1.0),
                );

            world.push(Rc::new(Sphere::new(loc, rad, world_spheres.len() as u32)));
            world_spheres.push(SphereGPU::new(loc, rad, random_range(0..7)));

            world.push(Rc::new(Triangle::new(
                loc.extend(1.0),
                loc2.extend(1.0),
                loc3.extend(1.0),
                world_triangle.len() as u32,
            )));
            world_triangle.push(TriangleGPU::new(
                loc.extend(1.0),
                loc2.extend(1.0),
                loc3.extend(1.0),
                loc.extend(1.0),
                loc2.extend(1.0),
                loc3.extend(1.0),
                random_range(0..7),
            ));
        }

        let root_bvh = BvhNode::new(world);
        root_bvh.shader_format(&mut linear);

        (linear, world_spheres, world_triangle)
    }

    pub fn test_model() -> (
        Vec<BvhGPU>,
        Vec<SphereGPU>,
        Vec<TriangleGPU>,
        Vec<MaterialGPU>,
    ) {
        let mut world: Vec<Rc<dyn Hittable>> = Vec::new();
        let mut linear: Vec<BvhGPU> = Vec::new();

        let mut world_spheres: Vec<SphereGPU> = Vec::new();
        let mut world_triangle: Vec<TriangleGPU> = Vec::new();
        let mut world_mats: Vec<MaterialGPU> = Vec::new();

        world_mats.push(MaterialGPU::new(glam::vec3(0.7, 0.0, 0.5), 1.0, 0.5));
        world_mats.push(MaterialGPU::new(glam::vec3(0.5, 0.5, 0.9), 0.4, 1.0));
        world_mats.push(MaterialGPU::new(glam::vec3(0.7, 0.9, 0.2), 0.7, 0.1));
        world_mats.push(MaterialGPU::new(glam::vec3(0.3, 0.5, 1.), 0.5, 0.3));

        world_mats.push(MaterialGPU::new(glam::vec3(1., 0.5, 0.), 0.1, 1.0));
        world_mats.push(MaterialGPU::new(glam::vec3(0.4, 0.2, 0.1), 0.0, 0.7));
        world_mats.push(MaterialGPU::new(glam::vec3(1., 0.5, 0.), 1.0, 0.0));

        let bytes = include_bytes!("../obj/car.obj");
        let mut curs = Cursor::new(bytes);

        let model = tobj::load_obj_buf(&mut curs, &tobj::GPU_LOAD_OPTIONS, |_| {
            Ok(Default::default())
        });
        let (models, _materials) = model.expect("Failed to load OBJ file");

        world.push(Rc::new(Sphere::new(
            glam::vec3(0.0, -400., -1.0),
            400.0,
            world_spheres.len() as u32,
        )));
        world_spheres.push(SphereGPU::new(glam::vec3(0.0, -400., -1.0), 400.0, 1));

        for (_i, m) in models.iter().enumerate() {
            let mesh = &m.mesh;
            for idx in mesh.indices.chunks(3) {
                let v0 = idx[0] as usize;
                let v1 = idx[1] as usize;
                let v2 = idx[2] as usize;

                let a = glam::vec4(
                    mesh.positions[3 * v0],
                    mesh.positions[3 * v0 + 1],
                    mesh.positions[3 * v0 + 2],
                    1.0,
                );

                let b = glam::vec4(
                    mesh.positions[3 * v1],
                    mesh.positions[3 * v1 + 1],
                    mesh.positions[3 * v1 + 2],
                    1.0,
                );

                let c = glam::vec4(
                    mesh.positions[3 * v2],
                    mesh.positions[3 * v2 + 1],
                    mesh.positions[3 * v2 + 2],
                    1.0,
                );

                let norm0 = glam::vec3(
                    mesh.normals[3 * v0],
                    mesh.normals[3 * v0 + 1],
                    mesh.normals[3 * v0 + 2],
                );

                let norm1 = glam::vec3(
                    mesh.normals[3 * v1],
                    mesh.normals[3 * v1 + 1],
                    mesh.normals[3 * v1 + 2],
                );

                let norm2 = glam::vec3(
                    mesh.normals[3 * v2],
                    mesh.normals[3 * v2 + 1],
                    mesh.normals[3 * v2 + 2],
                );

                world.push(Rc::new(Triangle::new(a, b, c, world_triangle.len() as u32)));
                world_triangle.push(TriangleGPU::new(
                    a,
                    b,
                    c,
                    norm0.extend(1.),
                    norm1.extend(1.),
                    norm2.extend(1.),
                    5,
                ));
            }
        }

        let root_bvh = BvhNode::new(world);
        root_bvh.shader_format(&mut linear);

        (linear, world_spheres, world_triangle, world_mats)
    }
}