use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Clone, Copy)]
pub struct TileId {
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Copy)]
pub struct AtlasLayout {
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
    pub _tiles_x: u32,
    pub _tiles_y: u32,
}

impl AtlasLayout {
    pub fn map_uv(&self, tile: TileId, uv: [f32; 2]) -> [f32; 2] {
        let tile_size = self.tile_size as f32;
        let tile_origin_x = tile.x as f32 * tile_size;
        let tile_origin_y = tile.y as f32 * tile_size;
        let pixel_x = tile_origin_x + uv[0].clamp(0.0, 1.0) * (tile_size - 1.0) + 0.5;
        let pixel_y = tile_origin_y + uv[1].clamp(0.0, 1.0) * (tile_size - 1.0) + 0.5;
        [pixel_x / self.width as f32, pixel_y / self.height as f32]
    }
}

pub struct TextureAtlas {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    layout: AtlasLayout,
}

#[derive(Deserialize)]
struct AtlasMetadata {
    texture: String,
    tile_size: u32,
}

impl TextureAtlas {
    pub fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        metadata_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let metadata_path = metadata_path.as_ref();
        let metadata: AtlasMetadata =
            serde_json::from_slice(&fs::read(metadata_path)?).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("metadata parse error: {err}"),
                )
            })?;

        let texture_path = resolve_texture_path(metadata_path, &metadata.texture);
        let image = image::open(&texture_path).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "failed to open atlas image {}: {err}",
                    texture_path.display()
                ),
            )
        })?;
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        if metadata.tile_size == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "tile_size must be > 0",
            ));
        }
        if width % metadata.tile_size != 0 || height % metadata.tile_size != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "image dimensions {}x{} are not divisible by tile_size {}",
                    width, height, metadata.tile_size
                ),
            ));
        }

        let tiles_x = width / metadata.tile_size;
        let tiles_y = height / metadata.tile_size;
        let pixel_data = rgba.into_raw();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Block atlas texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
            &pixel_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Block atlas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            _texture: texture,
            view,
            sampler,
            layout: AtlasLayout {
                width,
                height,
                tile_size: metadata.tile_size,
                _tiles_x: tiles_x,
                _tiles_y: tiles_y,
            },
        })
    }

    pub fn layout(&self) -> AtlasLayout {
        self.layout
    }

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Block atlas bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }
}

fn resolve_texture_path(metadata_path: &Path, texture: &str) -> PathBuf {
    let base = metadata_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(texture)
}
