use winit::{
    event::MouseScrollDelta::{self, PixelDelta},
    keyboard::KeyCode,
};

//refactor this asap

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pos: glam::Vec4,
    forward: glam::Vec4,
    right: glam::Vec4,
    up: glam::Vec4,
    fov: f32,
    _pad: [f32; 3],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            pos: glam::vec4(0.0, 0.0, 0.0, 0.0),
            forward: glam::vec4(0.0, 0.0, 0.0, 0.0),
            right: glam::vec4(0.0, 0.0, 0.0, 0.0),
            up: glam::vec4(0.0, 0.0, 0.0, 0.0),
            fov: 45_f32.to_radians(),
            _pad: [0., 0., 0.],
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.pos = camera.pos.extend(1.0);
        self.forward = camera.front.extend(1.0);
        self.right = camera.rght.extend(1.0);
        self.up = camera.actual_up.extend(1.0);

        self.fov = camera.fov;
    }
}
pub struct Camera {
    pub pos: glam::Vec3,
    pub front: glam::Vec3,
    pub rght: glam::Vec3,
    pub actual_up: glam::Vec3,

    up: glam::Vec3,

    pub yaw: f32,
    pub pitch: f32,

    forward: f32,
    left: f32,
    right: f32,
    back: f32,

    rh: f32,
    rv: f32,

    old_pos: glam::Vec3,
    old_dir: glam::Vec3,

    pub fov: f32,
    old_fov: f32,
}

//i also need a orbit camera with panning

impl Camera {
    pub fn new(pos: glam::Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            pos,
            front: glam::vec3(0.0, 0.0, -1.0),
            rght: glam::Vec3::X,
            actual_up: glam::Vec3::Y,

            up: glam::Vec3::Y,

            yaw,
            pitch,

            forward: 0.0,
            left: 0.0,
            right: 0.0,
            back: 0.0,
            rh: 0.0,
            rv: 0.0,

            old_pos: pos,
            old_dir: glam::vec3(0.0, 0.0, -1.0),

            fov: 45.0_f32.to_radians(),
            old_fov: 45.0_f32.to_radians(),
        }
    }

    // pub fn matrix(&self) -> glam::Mat4 {
    //     glam::Mat4::look_at_rh(self.pos, self.pos + self.front, self.up)
    // }

    //handle_key makes sense as I assume it might fire more times than the frame?
    pub fn handle_key(&mut self, key: KeyCode, press: bool) {
        let amount = if press { 1.0 } else { 0.0 };

        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => self.forward = amount,

            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.back = amount;
            }

            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.left = amount;
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.right = amount;
            }
            _ => {}
        }
    }

    pub fn handle_dir(&mut self, dx: f64, dy: f64) {
        self.rh = dx as f32;
        self.rv = dy as f32;
    }

    pub fn handle_wheel(&mut self, delta: MouseScrollDelta) {
        //move below?
        match delta {
            PixelDelta(pos) => {
                self.fov -= pos.y as f32 * 0.01;
                self.fov = self.fov.clamp(20_f32.to_radians(), 120_f32.to_radians());
            }
            _ => {}
        }
    }

    pub fn update(&mut self, delta: f32) -> bool {
        let speed = 24.0 * delta; //needs time
        self.pos += speed * self.front * self.forward;
        self.pos -= speed * self.front * self.back;

        self.pos -= speed * glam::Vec3::normalize(self.rght) * self.left;
        self.pos += speed * glam::Vec3::normalize(self.rght) * self.right;
        //pos and rotation

        let sens = 0.002;

        self.yaw += sens * self.rh;
        self.pitch -= sens * self.rv;

        let mx = 89.0_f32.to_radians();

        if self.pitch > mx {
            self.pitch = mx;
        }
        if self.pitch < -mx {
            self.pitch = -mx;
        }

        //spherical coordinates
        let dir = glam::vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        );

        self.front = glam::Vec3::normalize(dir);
        self.rght = glam::Vec3::normalize(glam::Vec3::cross(self.front, self.up));
        self.actual_up = glam::Vec3::normalize(glam::Vec3::cross(self.rght, self.front));

        self.rh = 0.0;
        self.rv = 0.0;

        let mut flag = true;

        if self.old_pos != self.pos || self.old_dir != dir || self.old_fov != self.fov {
            flag = false;
        }

        self.old_dir = dir;
        self.old_pos = self.pos;
        self.old_fov = self.fov;

        flag
    }
}
