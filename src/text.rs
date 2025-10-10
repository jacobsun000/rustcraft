use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};

const GLYPH_WIDTH: u32 = 5;
const GLYPH_HEIGHT: u32 = 7;
const GLYPH_SPACING_X: u32 = 1;
const GLYPH_SPACING_Y: u32 = 3;
const PADDING_X: f32 = 12.0;
const PADDING_Y: f32 = 14.0;

pub struct DebugOverlay {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    _texture: wgpu::Texture,
    _texture_view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    glyphs: HashMap<char, GlyphInfo>,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    vertex_count: usize,
    vertices: Vec<TextVertex>,
}

#[derive(Clone, Copy)]
struct GlyphInfo {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TextVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

impl DebugOverlay {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let (glyphs, atlas_pixels, atlas_size) = build_font_atlas();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Debug text atlas"),
            size: wgpu::Extent3d {
                width: atlas_size[0],
                height: atlas_size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(atlas_size[0] * 4),
                rows_per_image: Some(atlas_size[1]),
            },
            wgpu::Extent3d {
                width: atlas_size[0],
                height: atlas_size[1],
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Debug text sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Debug text bind group layout"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Debug text bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Debug text shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text_shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Debug text pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Debug text pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TextVertex>() as wgpu::BufferAddress,
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
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let initial_capacity = 256;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Debug text vertex buffer"),
            size: (initial_capacity * std::mem::size_of::<TextVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group,
            _texture: texture,
            _texture_view: texture_view,
            _sampler: sampler,
            glyphs,
            vertex_buffer,
            vertex_capacity: initial_capacity,
            vertex_count: 0,
            vertices: Vec::new(),
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: [u32; 2],
        text: &str,
    ) {
        if viewport[0] == 0 || viewport[1] == 0 {
            self.vertex_count = 0;
            return;
        }

        self.vertices.clear();
        let width = viewport[0] as f32;
        let height = viewport[1] as f32;

        let mut cursor_x = PADDING_X;
        let mut cursor_y = PADDING_Y;
        let line_height = (GLYPH_HEIGHT + GLYPH_SPACING_Y) as f32;
        let advance = (GLYPH_WIDTH + GLYPH_SPACING_X) as f32;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = PADDING_X;
                cursor_y += line_height;
                continue;
            }

            let key = if ch.is_ascii_alphabetic() {
                ch.to_ascii_uppercase()
            } else {
                ch
            };

            let glyph = match self.glyphs.get(&key) {
                Some(info) => info,
                None => {
                    cursor_x += advance;
                    continue;
                }
            };

            let x0 = cursor_x;
            let y0 = cursor_y;
            let x1 = x0 + GLYPH_WIDTH as f32;
            let y1 = y0 + GLYPH_HEIGHT as f32;

            let p0 = screen_to_ndc(x0, y0, width, height);
            let p1 = screen_to_ndc(x1, y0, width, height);
            let p2 = screen_to_ndc(x0, y1, width, height);
            let p3 = screen_to_ndc(x1, y1, width, height);

            let color = [1.0, 1.0, 1.0, 1.0];
            let (u0, v0, u1, v1) = (glyph.u0, glyph.v0, glyph.u1, glyph.v1);

            self.vertices.push(TextVertex {
                position: p0,
                uv: [u0, v0],
                color,
            });
            self.vertices.push(TextVertex {
                position: p1,
                uv: [u1, v0],
                color,
            });
            self.vertices.push(TextVertex {
                position: p2,
                uv: [u0, v1],
                color,
            });
            self.vertices.push(TextVertex {
                position: p2,
                uv: [u0, v1],
                color,
            });
            self.vertices.push(TextVertex {
                position: p1,
                uv: [u1, v0],
                color,
            });
            self.vertices.push(TextVertex {
                position: p3,
                uv: [u1, v1],
                color,
            });

            cursor_x += advance;
        }

        self.vertex_count = self.vertices.len();

        if self.vertex_count == 0 {
            return;
        }

        if self.vertex_count > self.vertex_capacity {
            self.vertex_capacity = self.vertex_count.next_power_of_two();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Debug text vertex buffer"),
                size: (self.vertex_capacity * std::mem::size_of::<TextVertex>())
                    as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        if self.vertex_count == 0 {
            return;
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Debug text pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count as u32, 0..1);
    }
}

fn screen_to_ndc(x: f32, y: f32, width: f32, height: f32) -> [f32; 2] {
    [(x / width) * 2.0 - 1.0, 1.0 - (y / height) * 2.0]
}

fn build_font_atlas() -> (HashMap<char, GlyphInfo>, Vec<u8>, [u32; 2]) {
    let patterns = glyph_patterns();
    let glyph_count = patterns.len() as u32;
    let cols = 8u32;
    let rows = (glyph_count + cols - 1) / cols;
    let width = cols * GLYPH_WIDTH;
    let height = rows * GLYPH_HEIGHT;

    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let mut glyphs = HashMap::new();

    for (index, pattern) in patterns.iter().enumerate() {
        let idx = index as u32;
        let tile_x = idx % cols;
        let tile_y = idx / cols;
        let base_x = tile_x * GLYPH_WIDTH;
        let base_y = tile_y * GLYPH_HEIGHT;

        for (row, mask) in pattern.rows.iter().enumerate() {
            for col in 0..GLYPH_WIDTH {
                if (mask >> (GLYPH_WIDTH - 1 - col)) & 1 == 1 {
                    let x = base_x + col;
                    let y = base_y + row as u32;
                    let offset = ((y * width + x) * 4) as usize;
                    pixels[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
                }
            }
        }

        let u0 = (base_x as f32 + 0.5) / width as f32;
        let v0 = (base_y as f32 + 0.5) / height as f32;
        let u1 = (base_x as f32 + GLYPH_WIDTH as f32 - 0.5) / width as f32;
        let v1 = (base_y as f32 + GLYPH_HEIGHT as f32 - 0.5) / height as f32;

        glyphs.insert(pattern.ch, GlyphInfo { u0, v0, u1, v1 });
    }

    (glyphs, pixels, [width, height])
}

struct GlyphPattern {
    ch: char,
    rows: [u8; GLYPH_HEIGHT as usize],
}

const fn glyph(ch: char, rows: [u8; GLYPH_HEIGHT as usize]) -> GlyphPattern {
    GlyphPattern { ch, rows }
}

fn glyph_patterns() -> Vec<GlyphPattern> {
    vec![
        glyph(' ', [0, 0, 0, 0, 0, 0, 0]),
        glyph(
            '0',
            [
                0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
            ],
        ),
        glyph(
            '1',
            [
                0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ],
        ),
        glyph(
            '2',
            [
                0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
            ],
        ),
        glyph(
            '3',
            [
                0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
            ],
        ),
        glyph(
            '4',
            [
                0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
            ],
        ),
        glyph(
            '5',
            [
                0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
            ],
        ),
        glyph(
            '6',
            [
                0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
            ],
        ),
        glyph(
            '7',
            [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
            ],
        ),
        glyph(
            '8',
            [
                0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
            ],
        ),
        glyph(
            '9',
            [
                0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
            ],
        ),
        glyph(
            'F',
            [
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
        ),
        glyph(
            'P',
            [
                0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
        ),
        glyph(
            'S',
            [
                0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
            ],
        ),
        glyph(
            'O',
            [
                0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ],
        ),
        glyph(
            ':',
            [
                0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
            ],
        ),
        glyph(
            '.',
            [
                0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
            ],
        ),
        glyph(
            '-',
            [
                0b00000, 0b00000, 0b00000, 0b01110, 0b00000, 0b00000, 0b00000,
            ],
        ),
        glyph(
            '+',
            [
                0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
            ],
        ),
    ]
}
