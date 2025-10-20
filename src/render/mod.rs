mod mesh;
mod raster;
mod raytrace;

pub use raster::RasterRenderer;
pub use raytrace::RayTraceRenderer;

use crate::camera::{Camera, Projection};
use crate::world::World;

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderTimings {
    pub total_ms: f32,
    pub scene_ms: f32,
    pub uniforms_ms: f32,
    pub compute_ms: f32,
    pub present_ms: f32,
    pub gpu_compute_ms: f32,
    pub gpu_present_ms: f32,
    pub voxels: u32,
    pub solid_blocks: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererKind {
    Rasterized,
    RayTraced,
}

impl RendererKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RendererKind::Rasterized => "Rasterized",
            RendererKind::RayTraced => "Ray Traced",
        }
    }
}

pub struct FrameContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub surface_config: &'a wgpu::SurfaceConfiguration,
    pub world: &'a World,
    pub camera: &'a Camera,
    pub projection: &'a Projection,
    pub camera_bind_group: &'a wgpu::BindGroup,
}

pub trait Renderer {
    fn kind(&self) -> RendererKind;

    fn resize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    );

    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        ctx: &FrameContext,
    );

    #[allow(dead_code)]
    fn timings(&self) -> Option<RenderTimings> {
        None
    }
}
