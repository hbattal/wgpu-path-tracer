#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialGPU {
    color: glam::Vec3,
    roughness: f32,
    metallic: f32,
    _pad: [f32; 3],
}

impl MaterialGPU {
    pub fn new(color: glam::Vec3, roughness: f32, metallic: f32) -> MaterialGPU {
        MaterialGPU {
            color,
            roughness,
            metallic,
            _pad: [0.0, 0.0, 0.0],
        }
    }
}

/*struct Material {
    color: vec3f,
    roughness: f32,
    metallic: f32,
}*/
