use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use glam::IVec3;
use wgpu::util::DeviceExt;

use crate::block::{self, BLOCK_AIR, BlockId};
use crate::render::{FrameContext, Renderer, RendererKind};
use crate::texture::{AtlasLayout, TextureAtlas, TileId};
use crate::world::{CHUNK_SIZE, World, chunk_min_corner};

pub struct RayTraceRenderer {
    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    fullscreen_vertex: wgpu::Buffer,
    fullscreen_index: wgpu::Buffer,
    index_count: u32,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group: Option<wgpu::BindGroup>,
    uniform_buffer: wgpu::Buffer,
    voxel_buffer: Option<wgpu::Buffer>,
    block_info_buffer: wgpu::Buffer,
    atlas_view: wgpu::TextureView,
    atlas_sampler: wgpu::Sampler,
    atlas_layout: AtlasLayout,
    screen: Option<ScreenTexture>,
    scene: Option<VoxelScene>,
    surface_format: wgpu::TextureFormat,
    last_log: Instant,
}

impl RayTraceRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        atlas: &TextureAtlas,
    ) -> Self {
        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Ray traced blit bind group layout"),
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

        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Ray traced blit sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let (fullscreen_vertex, fullscreen_index, index_count) = create_fullscreen_quad(device);
        let blit_pipeline = create_blit_pipeline(device, &blit_bind_group_layout, surface_format);

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Ray tracing compute bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                                RayUniforms,
                            >(
                            )
                                as u64),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let compute_pipeline = create_compute_pipeline(device, &compute_bind_group_layout);

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Ray tracing uniforms"),
            size: std::mem::size_of::<RayUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let block_info_data = build_block_metadata();
        let block_info_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Block metadata buffer"),
            contents: bytemuck::cast_slice(&block_info_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let atlas_view = atlas.create_view();
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Ray traced atlas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let atlas_layout = atlas.layout();

        Self {
            blit_pipeline,
            blit_bind_group_layout,
            blit_sampler,
            fullscreen_vertex,
            fullscreen_index,
            index_count,
            compute_pipeline,
            compute_bind_group_layout,
            compute_bind_group: None,
            uniform_buffer,
            voxel_buffer: None,
            block_info_buffer,
            atlas_view,
            atlas_sampler,
            atlas_layout,
            screen: None,
            scene: None,
            surface_format,
            last_log: Instant::now(),
        }
    }

    fn ensure_screen_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            self.screen = None;
            self.compute_bind_group = None;
            return;
        }

        let recreate = match self.screen.as_ref() {
            Some(screen) => screen.size != (width, height),
            None => true,
        };

        if !recreate {
            return;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ray traced storage texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ray traced blit bind group"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                },
            ],
        });

        self.screen = Some(ScreenTexture {
            _texture: texture,
            view,
            bind_group,
            size: (width, height),
        });

        self.recreate_compute_bind_group(device);
    }

    fn ensure_scene(&mut self, device: &wgpu::Device, world: &World) {
        let chunk_count = world.chunk_count();
        let needs_rebuild = match &self.scene {
            Some(scene) => scene.chunk_count != chunk_count,
            None => true,
        };

        if !needs_rebuild {
            return;
        }

        let Some(grid) = VoxelGrid::from_world(world) else {
            self.scene = None;
            self.voxel_buffer = None;
            self.compute_bind_group = None;
            return;
        };

        let voxel_data = grid.pack_voxels();

        let voxel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ray traced voxel buffer"),
            contents: bytemuck::cast_slice(&voxel_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        self.voxel_buffer = Some(voxel_buffer);
        self.scene = Some(VoxelScene { grid, chunk_count });
        self.recreate_compute_bind_group(device);
    }

    fn recreate_compute_bind_group(&mut self, device: &wgpu::Device) {
        let (screen, voxel) = match (&self.screen, &self.voxel_buffer) {
            (Some(screen), Some(voxel)) => (screen, voxel),
            _ => {
                self.compute_bind_group = None;
                return;
            }
        };

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ray tracing compute bind group"),
            layout: &self.compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&screen.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: voxel.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.block_info_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.atlas_sampler),
                },
            ],
        });

        self.compute_bind_group = Some(bind_group);
    }

    fn update_uniforms(&self, queue: &wgpu::Queue, ctx: &FrameContext, grid: &VoxelGrid) {
        let view = ctx.camera.view_matrix();
        let proj = ctx.projection.matrix();
        let inv_projection = proj.inverse();
        let view_to_world = view.inverse();

        let eye = ctx.camera.position;

        let uniforms = RayUniforms {
            inv_projection: inv_projection.to_cols_array_2d(),
            view_to_world: view_to_world.to_cols_array_2d(),
            eye: [eye.x, eye.y, eye.z, 1.0],
            grid_origin: [grid.origin.x, grid.origin.y, grid.origin.z, 0],
            grid_size: [
                grid.size.x as u32,
                grid.size.y as u32,
                grid.size.z as u32,
                0,
            ],
            stride: [
                grid.stride_y as u32,
                grid.stride_z as u32,
                ctx.surface_config.width,
                ctx.surface_config.height,
            ],
            atlas: [
                self.atlas_layout.tile_size,
                self.atlas_layout.width,
                self.atlas_layout.height,
                0,
            ],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }
}

impl Renderer for RayTraceRenderer {
    fn kind(&self) -> RendererKind {
        RendererKind::RayTraced
    }

    fn resize(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) {
        self.surface_format = config.format;
        self.blit_pipeline =
            create_blit_pipeline(device, &self.blit_bind_group_layout, self.surface_format);
        self.screen = None;
        self.compute_bind_group = None;
    }

    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        ctx: &FrameContext,
    ) {
        let width = ctx.surface_config.width;
        let height = ctx.surface_config.height;

        self.ensure_screen_texture(ctx.device, width, height);
        self.ensure_scene(ctx.device, ctx.world);

        let (scene, compute_bind_group) = match (&self.scene, &self.compute_bind_group) {
            (Some(scene), Some(bind_group)) => (scene, bind_group),
            _ => return,
        };

        self.update_uniforms(ctx.queue, ctx, &scene.grid);

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Ray tracing compute pass"),
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, compute_bind_group, &[]);

            let workgroup_size = 8u32;
            let dispatch_x = width.div_ceil(workgroup_size);
            let dispatch_y = height.div_ceil(workgroup_size);

            compute_pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
        }

        if self.last_log.elapsed().as_secs_f32() > 1.0 {
            log::info!(
                "Ray tracer: {}x{}, voxels {}x{}x{}",
                width,
                height,
                scene.grid.size.x,
                scene.grid.size.y,
                scene.grid.size.z
            );
            self.last_log = Instant::now();
        }

        let screen = self.screen.as_ref().expect("screen texture must exist");

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ray traced present"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.blit_pipeline);
        render_pass.set_bind_group(0, &screen.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.fullscreen_vertex.slice(..));
        render_pass.set_index_buffer(self.fullscreen_index.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

struct ScreenTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
}

struct VoxelScene {
    grid: VoxelGrid,
    chunk_count: usize,
}

struct VoxelGrid {
    origin: IVec3,
    size: IVec3,
    stride_y: usize,
    stride_z: usize,
    voxels: Vec<BlockId>,
}

impl VoxelGrid {
    fn from_world(world: &World) -> Option<Self> {
        let mut min = IVec3::new(i32::MAX, i32::MAX, i32::MAX);
        let mut max = IVec3::new(i32::MIN, i32::MIN, i32::MIN);
        let mut has_chunks = false;

        for (coord, _) in world.iter_chunks() {
            has_chunks = true;
            let base = chunk_min_corner(*coord);
            let chunk_max = base + IVec3::splat(CHUNK_SIZE as i32) - IVec3::new(1, 1, 1);
            min = min.min(base);
            max = max.max(chunk_max);
        }

        if !has_chunks {
            return None;
        }

        let size = max - min + IVec3::new(1, 1, 1);
        let size_x = size.x as usize;
        let size_y = size.y as usize;
        let size_z = size.z as usize;
        let stride_y = size_x;
        let stride_z = stride_y * size_y;
        let mut voxels = vec![BLOCK_AIR; stride_z * size_z];

        for (coord, chunk) in world.iter_chunks() {
            let base = chunk_min_corner(*coord);
            for (index, block) in chunk.blocks().iter().enumerate() {
                let lx = (index % CHUNK_SIZE) as i32;
                let temp = index / CHUNK_SIZE;
                let lz = (temp % CHUNK_SIZE) as i32;
                let ly = (temp / CHUNK_SIZE) as i32;

                let world_pos = base + IVec3::new(lx, ly, lz);
                let local = world_pos - min;

                if local.x < 0
                    || local.y < 0
                    || local.z < 0
                    || local.x as usize >= size_x
                    || local.y as usize >= size_y
                    || local.z as usize >= size_z
                {
                    continue;
                }

                let idx =
                    local.x as usize + local.y as usize * stride_y + local.z as usize * stride_z;
                voxels[idx] = *block;
            }
        }

        Some(Self {
            origin: min,
            size,
            stride_y,
            stride_z,
            voxels,
        })
    }

    fn pack_voxels(&self) -> Vec<u32> {
        let total = self.voxels.len();
        let words = (total + 3) / 4;
        let mut packed = Vec::with_capacity(words);

        for chunk in 0..words {
            let mut word = 0u32;
            for lane in 0..4 {
                let index = chunk * 4 + lane;
                if index >= total {
                    break;
                }
                let value = self.voxels[index] as u32;
                word |= value << (lane * 8);
            }
            packed.push(word);
        }

        packed
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuBlockInfo {
    face_tiles: [u32; 6],
    luminance: f32,
    specular: f32,
    diffuse: f32,
    roughness: f32,
}

fn build_block_metadata() -> Vec<GpuBlockInfo> {
    let mut entries = Vec::with_capacity(u8::MAX as usize + 1);
    for id in 0..=u8::MAX {
        let definition = block::block_definition(id);
        let mut face_tiles = [0u32; 6];
        for (idx, tile) in definition.face_tiles.iter().enumerate() {
            face_tiles[idx] = encode_tile_id(*tile);
        }
        entries.push(GpuBlockInfo {
            face_tiles,
            luminance: definition.luminance,
            specular: definition.specular,
            diffuse: definition.diffuse,
            roughness: definition.roughness,
        });
    }
    entries
}

fn encode_tile_id(tile: TileId) -> u32 {
    let x = tile.x & 0xFFFF;
    let y = tile.y & 0xFFFF;
    x | (y << 16)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RayUniforms {
    inv_projection: [[f32; 4]; 4],
    view_to_world: [[f32; 4]; 4],
    eye: [f32; 4],
    grid_origin: [i32; 4],
    grid_size: [u32; 4],
    stride: [u32; 4],
    atlas: [u32; 4],
}

fn create_fullscreen_quad(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct QuadVertex {
        position: [f32; 2],
        uv: [f32; 2],
    }

    const VERTICES: [QuadVertex; 4] = [
        QuadVertex {
            position: [-1.0, -1.0],
            uv: [0.0, 1.0],
        },
        QuadVertex {
            position: [1.0, -1.0],
            uv: [1.0, 1.0],
        },
        QuadVertex {
            position: [1.0, 1.0],
            uv: [1.0, 0.0],
        },
        QuadVertex {
            position: [-1.0, 1.0],
            uv: [0.0, 0.0],
        },
    ];

    const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Ray traced quad vertices"),
        contents: bytemuck::cast_slice(&VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Ray traced quad indices"),
        contents: bytemuck::cast_slice(&INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });

    (vertex_buffer, index_buffer, INDICES.len() as u32)
}

fn create_blit_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Ray traced blit pipeline layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Ray traced blit shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("raytrace_display.wgsl").into()),
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Ray traced blit pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 4 * 4,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                    wgpu::VertexAttribute {
                        offset: 8,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::ComputePipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Ray tracing compute pipeline layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Ray tracing compute shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("raytrace_compute.wgsl").into()),
    });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Ray tracing compute pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "cs_main",
    })
}
