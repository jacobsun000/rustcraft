use std::time::Duration;

use winit::event::{DeviceEvent, VirtualKeyCode};

use crate::camera::Camera;
use crate::config::KeyBindings;

pub struct CameraController {
    key_bindings: KeyBindings,
    speed: f32,
    turn_speed: f32,
    forward_pressed: bool,
    backward_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    up_pressed: bool,
    down_pressed: bool,
    yaw_left_pressed: bool,
    yaw_right_pressed: bool,
    pitch_up_pressed: bool,
    pitch_down_pressed: bool,
    yaw: f32,
    pitch: f32,
}

impl CameraController {
    pub fn new(speed: f32, turn_speed: f32, key_bindings: KeyBindings) -> Self {
        Self {
            key_bindings,
            speed,
            turn_speed,
            forward_pressed: false,
            backward_pressed: false,
            left_pressed: false,
            right_pressed: false,
            up_pressed: false,
            down_pressed: false,
            yaw_left_pressed: false,
            yaw_right_pressed: false,
            pitch_up_pressed: false,
            pitch_down_pressed: false,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, is_pressed: bool) -> bool {
        if key == self.key_bindings.forward {
            self.forward_pressed = is_pressed;
            true
        } else if key == self.key_bindings.backward {
            self.backward_pressed = is_pressed;
            true
        } else if key == self.key_bindings.left {
            self.left_pressed = is_pressed;
            true
        } else if key == self.key_bindings.right {
            self.right_pressed = is_pressed;
            true
        } else if key == self.key_bindings.up {
            self.up_pressed = is_pressed;
            true
        } else if key == self.key_bindings.down {
            self.down_pressed = is_pressed;
            true
        } else {
            match key {
                VirtualKeyCode::Left => {
                    self.yaw_left_pressed = is_pressed;
                    true
                }
                VirtualKeyCode::Right => {
                    self.yaw_right_pressed = is_pressed;
                    true
                }
                VirtualKeyCode::Up => {
                    self.pitch_up_pressed = is_pressed;
                    true
                }
                VirtualKeyCode::Down => {
                    self.pitch_down_pressed = is_pressed;
                    true
                }
                _ => false,
            }
        }
    }

    pub fn add_mouse_delta(&mut self, delta: (f32, f32), sensitivity: f32) {
        self.yaw += delta.0 * sensitivity;
        self.pitch -= delta.1 * sensitivity;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt_seconds: f32) {
        let forward = camera.forward();
        let right = forward.cross(glam::Vec3::Y).normalize_or_zero();

        let mut move_dir = glam::Vec3::ZERO;
        if self.forward_pressed {
            move_dir += forward;
        }
        if self.backward_pressed {
            move_dir -= forward;
        }
        if self.left_pressed {
            move_dir -= right;
        }
        if self.right_pressed {
            move_dir += right;
        }
        if self.up_pressed {
            move_dir += glam::Vec3::Y;
        }
        if self.down_pressed {
            move_dir -= glam::Vec3::Y;
        }

        if move_dir.length_squared() > 0.0 {
            camera.position += move_dir.normalize() * self.speed * dt_seconds;
        }

        let yaw_delta = (self.yaw_right_pressed as i32 - self.yaw_left_pressed as i32) as f32;
        let pitch_delta = (self.pitch_up_pressed as i32 - self.pitch_down_pressed as i32) as f32;

        self.yaw += yaw_delta * self.turn_speed * dt_seconds;
        self.pitch += pitch_delta * self.turn_speed * dt_seconds;

        camera.yaw += self.yaw;
        camera.pitch = (camera.pitch + self.pitch).clamp(-89.0_f32, 89.0_f32);

        self.yaw = 0.0;
        self.pitch = 0.0;
    }
}

#[derive(Default)]
pub struct MouseState {
    pub captured: bool,
    pub sensitivity: f32,
    pub max_frame_time: Option<f32>,
}

impl MouseState {
    pub fn new(sensitivity: f32, max_fps: Option<f32>) -> Self {
        let mut clamped = sensitivity;
        if !clamped.is_finite() || clamped <= 0.0 {
            clamped = 0.001;
        }
        let max_frame_time = max_fps.map(|fps| 1.0 / fps.max(1.0));
        Self {
            captured: false,
            sensitivity: clamped,
            max_frame_time,
        }
    }

    pub fn handle_device_event(
        &self,
        event: &DeviceEvent,
        sensitivity: f32,
        controller: &mut CameraController,
    ) {
        if !self.captured {
            return;
        }

        if let DeviceEvent::MouseMotion { delta } = event {
            controller.add_mouse_delta((delta.0 as f32, delta.1 as f32), sensitivity);
        }
    }

    pub fn frame_sleep(&self, frame_elapsed: f32) {
        if let Some(cap) = self.max_frame_time {
            if frame_elapsed < cap {
                std::thread::sleep(Duration::from_secs_f32(cap - frame_elapsed));
            }
        }
    }
}
