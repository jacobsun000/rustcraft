use glam::{Mat4, Vec3};

#[derive(Clone)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
        }
    }

    pub fn forward(&self) -> Vec3 {
        let yaw_radians = self.yaw.to_radians();
        let pitch_radians = self.pitch.to_radians();
        Vec3::new(
            yaw_radians.cos() * pitch_radians.cos(),
            pitch_radians.sin(),
            yaw_radians.sin() * pitch_radians.cos(),
        )
        .normalize()
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward(), Vec3::Y)
    }
}

pub struct Projection {
    pub fovy: f32,
    pub aspect: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        let aspect = if height == 0 {
            1.0
        } else {
            width as f32 / height as f32
        };
        Self {
            fovy,
            aspect,
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if height != 0 {
            self.aspect = width as f32 / height as f32;
        }
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::perspective_rh_gl(self.fovy.to_radians(), self.aspect, self.znear, self.zfar)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update(&mut self, camera: &Camera, projection: &Projection) {
        let view_proj = projection.matrix() * camera.view_matrix();
        self.view_proj = view_proj.to_cols_array_2d();
    }
}
