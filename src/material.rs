// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct MaterialGPU {
//     color: glam::Vec3,
//     roughness: f32,
//     metallic: f32,
//     ior: f32,
//     _pad: [f32; 2],
//     emiss: glam::Vec3,
//     _pad2: f32,
// }

// impl MaterialGPU {
//     pub fn new(
//         color: glam::Vec3,
//         roughness: f32,
//         metallic: f32,
//         ior: f32,
//         emiss: glam::Vec3,
//     ) -> MaterialGPU {
//         MaterialGPU {
//             color,
//             roughness,
//             metallic,
//             ior,
//             _pad: [0.0, 0.0],
//             emiss,
//             _pad2: 0.0,
//         }
//     }
// }

////////////////////////////////////////////
/// V2 MATERIAL

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PBRMaterialGPU {
    color_factor: glam::Vec4,
    color: i32,

    metal_rough: i32,
    metal_factor: f32,
    rough_factor: f32,

    emiss_factor: glam::Vec4,
    emiss: i32,

    ior: f32,

    color_sampler: i32,
    metal_rough_sampler: i32,
    emiss_sampler: i32,

    _pad: [f32; 3],
}

impl PBRMaterialGPU {
    pub fn new(
        color_factor: glam::Vec4,
        color: i32,

        metal_rough: i32,
        metal_factor: f32,
        rough_factor: f32,

        emiss_factor: glam::Vec4,
        emiss: i32,

        ior: f32,

        color_sampler: i32,
        metal_rough_sampler: i32,
        emiss_sampler: i32,
    ) -> PBRMaterialGPU {
        PBRMaterialGPU {
            color_factor,
            color,

            metal_rough,
            metal_factor,
            rough_factor,

            emiss_factor,
            emiss,

            ior,
            color_sampler,
            metal_rough_sampler,
            emiss_sampler,

            _pad: [0.0, 0.0, 0.0],
        }
    }
}
