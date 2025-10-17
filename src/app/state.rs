use std::{fmt::Write, time::Instant};

use glam::{IVec3, Vec3};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, MouseButton, VirtualKeyCode, WindowEvent};
use winit::window::{CursorGrabMode, Window};

use crate::camera::{Camera, CameraUniform, Projection};
use crate::config::{self, AppConfig, RenderMethodSetting};
use crate::fps::FpsCounter;
use crate::input::{CameraController, MouseState};
use crate::render::{FrameContext, RasterRenderer, RayTraceRenderer, RenderTimings, Renderer};
use crate::text::DebugOverlay;
use crate::texture::TextureAtlas;
use crate::world::{ChunkCoord, World, chunk_coord_from_block};

const CHUNK_LOAD_RADIUS: i32 = 4;
const CHUNK_VERTICAL_RADIUS: i32 = 1;

pub struct AppState {
    window: Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    camera: Camera,
    projection: Projection,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
    mouse_state: MouseState,
    debug_overlay: DebugOverlay,
    fps_counter: FpsCounter,
    last_frame: Instant,
    last_frame_time: f32,
    world: World,
    _block_atlas: TextureAtlas,
    renderer: Box<dyn Renderer>,
    loaded_chunk_center: ChunkCoord,
    chunk_radius: i32,
    chunk_vertical_radius: i32,
}

impl AppState {
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();
        let config = AppConfig::load();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            dx12_shader_compiler: Default::default(),
        });
        let surface =
            unsafe { instance.create_surface(&window) }.expect("Failed to create surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find adapter");
        let adapter_features = adapter.features();
        let mut required_features = wgpu::Features::empty();
        if adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            required_features |= wgpu::Features::TIMESTAMP_QUERY;
        }
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Primary device"),
                    features: required_features,
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let present_mode = choose_present_mode(&surface_caps.present_modes, config.present_mode);
        let alpha_mode = surface_caps.alpha_modes[0];

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let camera = Camera::new(Vec3::new(0.0, 24.0, 60.0), -90.0, -20.0);
        let mut projection = Projection::new(
            surface_config.width,
            surface_config.height,
            60.0,
            0.1,
            200.0,
        );
        projection.resize(surface_config.width, surface_config.height);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let atlas_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures/blocks.json");
        let block_atlas =
            TextureAtlas::load(&device, &queue, atlas_path).expect("Failed to load block atlas");

        let mut world = World::new();
        let start_chunk = chunk_coord_from_block(IVec3::new(
            camera.position.x.floor() as i32,
            camera.position.y.floor() as i32,
            camera.position.z.floor() as i32,
        ));
        populate_world_chunks(
            &mut world,
            start_chunk,
            CHUNK_LOAD_RADIUS,
            CHUNK_VERTICAL_RADIUS,
        );

        let renderer: Box<dyn Renderer> = match config.render_method {
            RenderMethodSetting::Rasterized => Box::new(RasterRenderer::new(
                &device,
                &queue,
                &surface_config,
                &world,
                &block_atlas,
                &camera_bind_group_layout,
            )),
            RenderMethodSetting::RayTraced => Box::new(RayTraceRenderer::new(
                &device,
                &queue,
                surface_format,
                &block_atlas,
            )),
        };

        let debug_overlay = DebugOverlay::new(&device, &queue, surface_config.format);

        Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            size,
            camera,
            projection,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller: CameraController::new(6.0, 90.0, config.key_bindings.clone()),
            mouse_state: MouseState::new(config.mouse_sensitivity, config.max_fps),
            debug_overlay,
            fps_counter: FpsCounter::default(),
            last_frame: Instant::now(),
            last_frame_time: 0.0,
            world,
            _block_atlas: block_atlas,
            renderer,
            loaded_chunk_center: start_chunk,
            chunk_radius: CHUNK_LOAD_RADIUS,
            chunk_vertical_radius: CHUNK_VERTICAL_RADIUS,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    #[allow(dead_code)]
    pub fn camera_controller_mut(&mut self) -> &mut CameraController {
        &mut self.camera_controller
    }

    #[allow(dead_code)]
    pub fn last_frame_seconds(&self) -> f32 {
        self.last_frame_time
    }

    #[allow(dead_code)]
    pub fn chunk_count(&self) -> usize {
        self.world.chunk_count()
    }

    #[allow(dead_code)]
    pub fn renderer_kind(&self) -> crate::render::RendererKind {
        self.renderer.kind()
    }

    #[allow(dead_code)]
    pub fn surface_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    #[allow(dead_code)]
    pub fn renderer_timings(&self) -> Option<RenderTimings> {
        self.renderer.timings()
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.projection.resize(new_size.width, new_size.height);
        self.camera_uniform.update(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
        self.renderer
            .resize(&self.device, &self.queue, &self.surface_config);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(key) = input.virtual_keycode {
                    let is_pressed = input.state == ElementState::Pressed;
                    if is_pressed && key == VirtualKeyCode::Escape && self.mouse_state.captured {
                        self.set_mouse_capture(false);
                        return true;
                    }
                    self.camera_controller.process_keyboard(key, is_pressed)
                } else {
                    false
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left && *state == ElementState::Pressed {
                    if !self.mouse_state.captured {
                        self.set_mouse_capture(true);
                    }
                    true
                } else {
                    false
                }
            }
            WindowEvent::Focused(false) => {
                self.set_mouse_capture(false);
                false
            }
            _ => false,
        }
    }

    pub fn device_input(&mut self, event: &DeviceEvent) {
        self.mouse_state.handle_device_event(
            event,
            self.mouse_state.sensitivity,
            &mut self.camera_controller,
        );
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = now - self.last_frame;
        self.last_frame = now;
        let dt_seconds = dt.as_secs_f32();

        self.camera_controller
            .update_camera(&mut self.camera, dt_seconds);
        self.camera_uniform.update(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        let fps = self.fps_counter.update(dt_seconds);
        self.last_frame_time = dt_seconds;
        let pos = self.camera.position;
        let block_pos = IVec3::new(
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        );
        let cam_chunk = chunk_coord_from_block(block_pos);
        if cam_chunk != self.loaded_chunk_center {
            self.world.ensure_chunks_in_radius(
                cam_chunk,
                self.chunk_radius,
                self.chunk_vertical_radius,
            );
            self.loaded_chunk_center = cam_chunk;
        }
        let chunk_count = self.world.chunk_count();

        let mut chunk_grid = String::new();
        let grid_radius = 2;
        let _ = writeln!(&mut chunk_grid, "Chunk grid (X/Z):");
        for dz in (-grid_radius..=grid_radius).rev() {
            chunk_grid.push(' ');
            for dx in -grid_radius..=grid_radius {
                let coord = ChunkCoord {
                    x: cam_chunk.x + dx,
                    y: cam_chunk.y,
                    z: cam_chunk.z + dz,
                };
                let marker = if dx == 0 && dz == 0 {
                    'C'
                } else if self.world.chunk(coord).is_some() {
                    '#'
                } else {
                    '.'
                };
                chunk_grid.push(marker);
                if dx != grid_radius {
                    chunk_grid.push(' ');
                }
            }
            chunk_grid.push('\n');
        }
        let _ = writeln!(&mut chunk_grid, "C=current chunk, #=loaded");

        let debug_text = format!(
            r#"
Renderer: {}
FPS: {:>5.1}
Frame: {:>6.2} ms
POS: {:+5.1} {:+5.1} {:+5.1}
Chunk: {:+4} {:+4} {:+4}
Chunks: {:>3}
{}
"#,
            self.renderer.kind().as_str(),
            fps,
            self.last_frame_time * 1000.0,
            pos.x,
            pos.y,
            pos.z,
            cam_chunk.x,
            cam_chunk.y,
            cam_chunk.z,
            chunk_count,
            chunk_grid.trim_end(),
        );
        let viewport = [self.size.width, self.size.height];
        self.debug_overlay
            .prepare(&self.device, &self.queue, viewport, &debug_text);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            });

        let frame_ctx = FrameContext {
            device: &self.device,
            queue: &self.queue,
            surface_config: &self.surface_config,
            world: &self.world,
            camera: &self.camera,
            projection: &self.projection,
            camera_bind_group: &self.camera_bind_group,
        };

        self.renderer.render(&mut encoder, &view, &frame_ctx);
        self.debug_overlay.render(&mut encoder, &view);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    pub fn handle_escape(&mut self) -> bool {
        if self.mouse_state.captured {
            self.set_mouse_capture(false);
            false
        } else {
            true
        }
    }

    pub fn sleep_if_needed(&self) {
        let elapsed = self.last_frame.elapsed().as_secs_f32();
        self.mouse_state.frame_sleep(elapsed);
    }

    fn set_mouse_capture(&mut self, capture: bool) {
        if self.mouse_state.captured == capture {
            return;
        }

        if capture {
            if let Err(err) = self.window.set_cursor_grab(CursorGrabMode::Locked) {
                log::warn!("Failed to lock cursor: {err:?}. Falling back to confined mode.");
                if let Err(err) = self.window.set_cursor_grab(CursorGrabMode::Confined) {
                    log::warn!("Unable to grab cursor: {err:?}");
                }
            }
            self.window.set_cursor_visible(false);
        } else {
            if let Err(err) = self.window.set_cursor_grab(CursorGrabMode::None) {
                log::warn!("Failed to release cursor grab: {err:?}");
            }
            self.window.set_cursor_visible(true);
        }

        self.mouse_state.captured = capture;
    }
}

fn populate_world_chunks(world: &mut World, center: ChunkCoord, radius: i32, vertical: i32) {
    world.ensure_chunks_in_radius(center, radius, vertical);
}

fn choose_present_mode(
    available: &[wgpu::PresentMode],
    requested: config::PresentModeSetting,
) -> wgpu::PresentMode {
    let candidates = match requested {
        config::PresentModeSetting::Immediate => vec![
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Fifo,
        ],
        config::PresentModeSetting::Mailbox => vec![
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::Fifo,
        ],
        config::PresentModeSetting::VSync => vec![
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
        ],
    };

    candidates
        .into_iter()
        .find(|mode| available.contains(mode))
        .unwrap_or(wgpu::PresentMode::Fifo)
}

pub fn sleep_on_main_events(state: &AppState) {
    state.sleep_if_needed();
}
