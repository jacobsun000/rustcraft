use std::{iter, path::Path, time::Instant};

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, Window, WindowBuilder},
};

mod config;
mod mesh;
mod text;
mod texture;
mod world;

struct State {
    window: Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    vertex_count: u32,
    chunk_count: u32,
    atlas_bind_group: wgpu::BindGroup,
    _block_atlas: texture::TextureAtlas,
    depth_texture: DepthTexture,
    camera: Camera,
    projection: Projection,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
    debug_overlay: text::DebugOverlay,
    fps_counter: FpsCounter,
    mouse_state: MouseState,
    _config: config::AppConfig,
    _world: world::World,
    last_frame: Instant,
    last_frame_time: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

struct DepthTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl DepthTexture {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

    fn create(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
        }
    }
}

struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
        }
    }

    fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward(), Vec3::Y)
    }

    fn forward(&self) -> Vec3 {
        let yaw_radians = self.yaw.to_radians();
        let pitch_radians = self.pitch.to_radians();
        Vec3::new(
            yaw_radians.cos() * pitch_radians.cos(),
            pitch_radians.sin(),
            yaw_radians.sin() * pitch_radians.cos(),
        )
        .normalize()
    }
}

struct Projection {
    fovy: f32,
    aspect: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
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

    fn resize(&mut self, width: u32, height: u32) {
        if height != 0 {
            self.aspect = width as f32 / height as f32;
        }
    }

    fn matrix(&self) -> Mat4 {
        Mat4::perspective_rh_gl(self.fovy.to_radians(), self.aspect, self.znear, self.zfar)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn update(&mut self, camera: &Camera, projection: &Projection) {
        let view_proj = projection.matrix() * camera.view_matrix();
        self.view_proj = view_proj.to_cols_array_2d();
    }
}

struct CameraController {
    key_bindings: config::KeyBindings,
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
    fn new(speed: f32, turn_speed: f32, key_bindings: config::KeyBindings) -> Self {
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

    fn process_keyboard(&mut self, key: VirtualKeyCode, is_pressed: bool) -> bool {
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

    fn update_camera(&mut self, camera: &mut Camera, dt: f32) {
        let forward = camera.forward();
        let right = forward.cross(Vec3::Y).normalize_or_zero();

        let mut move_dir = Vec3::ZERO;
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
            move_dir += Vec3::Y;
        }
        if self.down_pressed {
            move_dir -= Vec3::Y;
        }

        if move_dir.length_squared() > 0.0 {
            camera.position += move_dir.normalize() * self.speed * dt;
        }

        let yaw_delta = (self.yaw_right_pressed as i32 - self.yaw_left_pressed as i32) as f32;
        let pitch_delta = (self.pitch_up_pressed as i32 - self.pitch_down_pressed as i32) as f32;

        self.yaw += yaw_delta * self.turn_speed * dt;
        self.pitch = (self.pitch + pitch_delta * self.turn_speed * dt).clamp(-89.0_f32, 89.0_f32);

        camera.yaw += self.yaw;
        camera.pitch = (camera.pitch + self.pitch).clamp(-89.0_f32, 89.0_f32);
        self.yaw = 0.0;
        self.pitch = 0.0;
    }

    fn add_mouse_delta(&mut self, delta: (f32, f32), sensitivity: f32) {
        self.yaw += delta.0 * sensitivity;
        self.pitch -= delta.1 * sensitivity;
    }
}

impl State {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

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
            .expect("Failed to find an appropriate adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Primary device"),
                    features: wgpu::Features::empty(),
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
        let present_mode = surface_caps
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .unwrap_or(surface_caps.present_modes[0]);
        let alpha_mode = surface_caps.alpha_modes[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let depth_texture = DepthTexture::create(&device, &config);

        let camera = Camera::new(Vec3::new(0.0, 24.0, 60.0), -90.0, -20.0);
        let projection = Projection::new(config.width, config.height, 60.0, 0.1, 100.0);
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let atlas_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures/blocks.json");
        let block_atlas = texture::TextureAtlas::load(&device, &queue, atlas_path)
            .expect("Failed to load block atlas");
        let atlas_layout = block_atlas.layout();
        let atlas_bind_group = block_atlas.create_bind_group(&device, &texture_bind_group_layout);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("World shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("World pipeline layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("World pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::buffer_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthTexture::FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let app_config = config::AppConfig::load();
        let mouse_sensitivity = app_config.mouse_sensitivity;
        let key_bindings = app_config.key_bindings.clone();

        let mut world = world::World::new();
        const CHUNK_RADIUS: i32 = 2;

        for z in -CHUNK_RADIUS..=CHUNK_RADIUS {
            for x in -CHUNK_RADIUS..=CHUNK_RADIUS {
                let coord = world::ChunkCoord { x, y: 0, z };
                world.ensure_chunk(coord);
            }
        }

        let mut combined_vertices: Vec<Vertex> = Vec::new();
        let mut combined_indices: Vec<u32> = Vec::new();

        let mut chunk_count = 0u32;
        for z in -CHUNK_RADIUS..=CHUNK_RADIUS {
            for x in -CHUNK_RADIUS..=CHUNK_RADIUS {
                let coord = world::ChunkCoord { x, y: 0, z };
                let mesh = mesh::build_chunk_mesh(&world, coord, &atlas_layout);
                let base_index = combined_vertices.len() as u32;
                combined_vertices.extend(mesh.vertices.into_iter().map(|v| Vertex {
                    position: v.position,
                    color: v.color,
                    uv: v.uv,
                }));
                combined_indices.extend(mesh.indices.into_iter().map(|i| i + base_index));
                chunk_count += 1;
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Terrain vertex buffer"),
            contents: bytemuck::cast_slice(&combined_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Terrain index buffer"),
            contents: bytemuck::cast_slice(&combined_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let vertex_count = combined_vertices.len() as u32;
        let index_count = combined_indices.len() as u32;

        let debug_overlay = text::DebugOverlay::new(&device, &queue, config.format);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            vertex_count,
            chunk_count,
            atlas_bind_group,
            _block_atlas: block_atlas,
            depth_texture,
            camera,
            projection,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller: CameraController::new(6.0, 90.0, key_bindings),
            debug_overlay,
            fps_counter: FpsCounter::default(),
            mouse_state: MouseState::new(mouse_sensitivity),
            _config: app_config,
            _world: world,
            last_frame: Instant::now(),
            last_frame_time: 0.0,
        }
    }

    fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.depth_texture = DepthTexture::create(&self.device, &self.config);
        self.projection.resize(new_size.width, new_size.height);
        self.camera_uniform.update(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    let is_pressed = input.state == ElementState::Pressed;
                    if is_pressed && keycode == VirtualKeyCode::Escape {
                        if self.mouse_state.captured {
                            self.set_mouse_capture(false);
                            return true;
                        }
                    }
                    self.camera_controller.process_keyboard(keycode, is_pressed)
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

    fn device_input(&mut self, event: &DeviceEvent) {
        if !self.mouse_state.captured {
            return;
        }

        if let DeviceEvent::MouseMotion { delta } = event {
            self.camera_controller.add_mouse_delta(
                (delta.0 as f32, delta.1 as f32),
                self.mouse_state.sensitivity,
            );
        }
    }

    fn update(&mut self) {
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
        let debug_text = format!(
            "FPS: {:>5.1}\nFrame: {:>6.2} ms\nTris: {:>7}\nVerts: {:>7}\nChunks: {:>4}\nPOS: {:+5.1} {:+5.1} {:+5.1}",
            fps,
            self.last_frame_time * 1000.0,
            self.index_count / 3,
            self.vertex_count,
            self.chunk_count,
            pos.x,
            pos.y,
            pos.z
        );
        let viewport = [self.size.width, self.size.height];
        let device = &self.device;
        let queue = &self.queue;
        self.debug_overlay
            .prepare(device, queue, viewport, &debug_text);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear color"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.atlas_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.debug_overlay.render(&mut encoder, &view);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
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

    fn handle_escape(&mut self) -> bool {
        if self.mouse_state.captured {
            self.set_mouse_capture(false);
            false
        } else {
            true
        }
    }
}

#[derive(Default)]
struct FpsCounter {
    elapsed: f32,
    frames: u32,
    fps: f32,
}

impl FpsCounter {
    fn update(&mut self, dt: f32) -> f32 {
        self.elapsed += dt;
        self.frames += 1;
        if self.elapsed >= 0.5 {
            self.fps = self.frames as f32 / self.elapsed.max(1e-6);
            self.elapsed = 0.0;
            self.frames = 0;
        }
        self.fps
    }
}

struct MouseState {
    captured: bool,
    sensitivity: f32,
}

impl MouseState {
    fn new(sensitivity: f32) -> Self {
        let mut clamped = sensitivity;
        if !clamped.is_finite() || clamped <= 0.0 {
            clamped = 0.001;
        }
        Self {
            captured: false,
            sensitivity: clamped,
        }
    }
}

async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rustcraft")
        .build(&event_loop)
        .expect("Failed to create window");

    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => {
                            if state.handle_escape() {
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::DeviceEvent { ref event, .. } => {
                state.device_input(event);
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(err) => log::warn!("Render error: {:?}", err),
                }
            }
            Event::MainEventsCleared => {
                state.window().request_redraw();
            }
            _ => {}
        }
    });
}

fn main() {
    env_logger::init();
    pollster::block_on(run());
}
