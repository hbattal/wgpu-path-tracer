use crate::aabb::*;
use crate::bvh::BvhGPU;
use crate::interval::*;

pub trait Hittable {
    fn bounding_box(&self) -> AABB;
    fn shader_format(&self, linear: &mut Vec<BvhGPU>);
    fn bvh_index(&self) -> i32;
    fn centroid(&self) -> glam::Vec3 {
        glam::vec3(0.0, 0.0, 0.0)
    }
}

pub struct Sphere {
    index: u32,

    bbox: AABB,
}

impl Sphere {
    pub fn new(center: glam::Vec3, radius: f32, index: u32) -> Sphere {
        let rvec = glam::vec3(radius, radius, radius);

        Sphere {
            index,
            bbox: AABB::new_point(center - rvec, center + rvec),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereGPU {
    pub center: glam::Vec3,
    pub rad: f32,
    pub mat: u32,
    pub _pad2: [u32; 3],
}

impl SphereGPU {
    pub fn new(center: glam::Vec3, radius: f32, mat: u32) -> SphereGPU {
        SphereGPU {
            center,
            rad: radius,
            mat,
            _pad2: [0, 0, 0],
        }
    }
}

impl Hittable for Sphere {
    fn bounding_box(&self) -> AABB {
        self.bbox
    }

    fn bvh_index(&self) -> i32 {
        -1
    }

    fn shader_format(&self, linear: &mut Vec<BvhGPU>) {
        linear.push(BvhGPU {
            right: self.index as i32, //should be location in sphere storage buffer?
            typ: 3,                   //for Sphere
            x: [self.bbox.x.min, self.bbox.x.max],
            y: [self.bbox.y.min, self.bbox.y.max],
            z: [self.bbox.z.min, self.bbox.z.max],
        });
    }

    fn centroid(&self) -> glam::Vec3 {
        glam::vec3(
            (self.bbox.x.min + self.bbox.x.max) * 0.5,
            (self.bbox.y.min + self.bbox.y.max) * 0.5,
            (self.bbox.z.min + self.bbox.z.max) * 0.5,
        )
    }
}
pub struct Triangle {
    a: glam::Vec3,
    b: glam::Vec3,
    c: glam::Vec3,
    index: u32,
    bbox: AABB,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TriangleGPU {
    a: glam::Vec4,
    b: glam::Vec4,
    c: glam::Vec4,
    norm0: glam::Vec4,
    norm1: glam::Vec4,
    norm2: glam::Vec4,
    uv0: glam::Vec2,
    uv1: glam::Vec2,
    uv2: glam::Vec2,

    mat: u32,
    _pad: u32,
}

impl TriangleGPU {
    pub fn new(
        a: glam::Vec4,
        b: glam::Vec4,
        c: glam::Vec4,
        norm0: glam::Vec4,
        norm1: glam::Vec4,
        norm2: glam::Vec4,
        uv0: glam::Vec2,
        uv1: glam::Vec2,
        uv2: glam::Vec2,
        mat: u32,
    ) -> TriangleGPU {
        TriangleGPU {
            a,
            b,
            c,
            norm0,
            norm1,
            norm2,
            uv0,
            uv1,
            uv2,
            mat,
            _pad: 0,
        }
    }
}

impl Triangle {
    pub fn new(a: glam::Vec4, b: glam::Vec4, c: glam::Vec4, index: u32) -> Triangle {
        let x_interval: Interval =
            Interval::new(a[0].min(b[0]).min(c[0]), a[0].max(b[0]).max(c[0]));
        let y_interval: Interval =
            Interval::new(a[1].min(b[1]).min(c[1]), a[1].max(b[1]).max(c[1]));
        let z_interval: Interval =
            Interval::new(a[2].min(b[2]).min(c[2]), a[2].max(b[2]).max(c[2]));

        let bbox = AABB::new(x_interval, y_interval, z_interval);

        Triangle {
            a: a.truncate(),
            b: b.truncate(),
            c: c.truncate(),
            index,
            bbox: bbox,
        }
    }
}

impl Hittable for Triangle {
    fn bounding_box(&self) -> AABB {
        self.bbox
    }

    fn bvh_index(&self) -> i32 {
        -1
    }

    fn shader_format(&self, linear: &mut Vec<BvhGPU>) {
        linear.push(BvhGPU {
            right: self.index as i32,
            typ: 2, //for Triangle
            x: [self.bbox.x.min, self.bbox.x.max],
            y: [self.bbox.y.min, self.bbox.y.max],
            z: [self.bbox.z.min, self.bbox.z.max],
        });
    }

    fn centroid(&self) -> glam::Vec3 {
        (self.a + self.b + self.c) * 0.333333
    }
}
