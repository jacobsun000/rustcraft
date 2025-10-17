#![allow(dead_code)]

#[path = "../app/state.rs"]
mod app_state;
#[path = "../block.rs"]
mod block;
#[path = "../camera.rs"]
mod camera;
#[path = "../config.rs"]
mod config;
#[path = "../fps.rs"]
mod fps;
#[path = "../input.rs"]
mod input;
#[path = "../render/mod.rs"]
mod render;
#[path = "../text.rs"]
mod text;
#[path = "../texture.rs"]
mod texture;
#[path = "../world.rs"]
mod world;

use std::time::{Duration, Instant};

use app_state::{AppState, sleep_on_main_events};
use config::{AppConfig, KeyBindings, PresentModeSetting};
use input::CameraController;
use render::RendererKind;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    env_logger::init();
    run_benchmark();
}

fn run_benchmark() {
    let app_config = AppConfig::load();
    let key_bindings = app_config.key_bindings.clone();
    let mouse_sensitivity = app_config.mouse_sensitivity;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rustcraft Benchmark")
        .build(&event_loop)
        .expect("Failed to create benchmark window");

    let mut app_state = pollster::block_on(AppState::new(window));

    let mut script = BenchmarkScript::new(key_bindings.clone());
    let script_duration = script.total_duration();
    let padding_seconds = 2.0;
    let target_duration = Duration::from_secs_f32(script_duration + padding_seconds);
    let mut metrics = BenchmarkMetrics::default();
    let mut last_tick = Instant::now();
    let benchmark_start = last_tick;

    println!(
        "Benchmark: {:.1}s scripted path across {} segments ({} renderer).",
        target_duration.as_secs_f32(),
        script.segment_count(),
        app_state.renderer_kind().as_str(),
    );

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, window_id } if window_id == app_state.window().id() => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => app_state.resize(size),
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        app_state.resize(*new_inner_size)
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if window_id == app_state.window().id() => {
                let now = Instant::now();
                let dt = now.saturating_duration_since(last_tick).as_secs_f32();
                last_tick = now;

                script.advance(dt, app_state.camera_controller_mut(), mouse_sensitivity);

                app_state.update();

                match app_state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        app_state.resize(app_state.window().inner_size())
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        eprintln!("Render device ran out of memory; ending benchmark early.");
                        metrics.print_summary(
                            benchmark_start.elapsed().as_secs_f32(),
                            app_state.renderer_kind(),
                            app_state.surface_size(),
                            app_config.present_mode,
                            script.segment_count(),
                        );
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    Err(err) => {
                        log::warn!("Render error: {err:?}");
                    }
                }

                let timings = app_state.renderer_timings();
                metrics.record(
                    app_state.last_frame_seconds(),
                    app_state.chunk_count(),
                    timings,
                );

                if benchmark_start.elapsed() >= target_duration {
                    metrics.print_summary(
                        benchmark_start.elapsed().as_secs_f32(),
                        app_state.renderer_kind(),
                        app_state.surface_size(),
                        app_config.present_mode,
                        script.segment_count(),
                    );
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::MainEventsCleared => {
                sleep_on_main_events(&app_state);
                app_state.window().request_redraw();
            }
            _ => {}
        }
    });
}

#[derive(Clone, Copy, Default)]
struct MovementState {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

impl MovementState {
    fn with_forward(mut self, enabled: bool) -> Self {
        self.forward = enabled;
        self
    }

    fn with_backward(mut self, enabled: bool) -> Self {
        self.backward = enabled;
        self
    }

    fn with_left(mut self, enabled: bool) -> Self {
        self.left = enabled;
        self
    }

    fn with_right(mut self, enabled: bool) -> Self {
        self.right = enabled;
        self
    }

    fn with_up(mut self, enabled: bool) -> Self {
        self.up = enabled;
        self
    }
}

struct ScriptSegment {
    duration: f32,
    movement: MovementState,
    yaw_rate: f32,
    pitch_rate: f32,
}

impl ScriptSegment {
    fn new(duration: f32, movement: MovementState, yaw_rate: f32, pitch_rate: f32) -> Self {
        Self {
            duration: duration.max(0.0),
            movement,
            yaw_rate,
            pitch_rate,
        }
    }
}

struct BenchmarkScript {
    segments: Vec<ScriptSegment>,
    key_bindings: KeyBindings,
    current: usize,
    elapsed_in_segment: f32,
}

impl BenchmarkScript {
    fn new(key_bindings: KeyBindings) -> Self {
        Self {
            segments: default_segments(),
            key_bindings,
            current: 0,
            elapsed_in_segment: 0.0,
        }
    }

    fn total_duration(&self) -> f32 {
        self.segments.iter().map(|segment| segment.duration).sum()
    }

    fn segment_count(&self) -> usize {
        self.segments.len()
    }

    fn advance(&mut self, mut dt: f32, controller: &mut CameraController, sensitivity: f32) {
        while dt > 0.0 {
            if self.current >= self.segments.len() {
                Self::apply_movement(controller, &self.key_bindings, &MovementState::default());
                break;
            }

            let segment_duration = self.segments[self.current].duration;
            if segment_duration <= 0.0 {
                self.current += 1;
                self.elapsed_in_segment = 0.0;
                continue;
            }

            if self.elapsed_in_segment >= segment_duration {
                self.current += 1;
                self.elapsed_in_segment = 0.0;
                continue;
            }

            let segment = &self.segments[self.current];
            Self::apply_movement(controller, &self.key_bindings, &segment.movement);

            let remaining = (segment_duration - self.elapsed_in_segment).max(0.0);
            let step = dt.min(remaining);

            if step > 0.0 {
                Self::apply_rotation(
                    controller,
                    sensitivity,
                    segment.yaw_rate,
                    segment.pitch_rate,
                    step,
                );
                self.elapsed_in_segment += step;
                dt -= step;
            } else {
                dt = 0.0;
            }

            if self.elapsed_in_segment + 1e-4 >= segment_duration {
                self.current += 1;
                self.elapsed_in_segment = 0.0;
            }
        }
    }

    fn apply_movement(
        controller: &mut CameraController,
        bindings: &KeyBindings,
        movement: &MovementState,
    ) {
        controller.process_keyboard(bindings.forward, movement.forward);
        controller.process_keyboard(bindings.backward, movement.backward);
        controller.process_keyboard(bindings.left, movement.left);
        controller.process_keyboard(bindings.right, movement.right);
        controller.process_keyboard(bindings.up, movement.up);
        controller.process_keyboard(bindings.down, movement.down);
    }

    fn apply_rotation(
        controller: &mut CameraController,
        sensitivity: f32,
        yaw_rate: f32,
        pitch_rate: f32,
        dt: f32,
    ) {
        if !sensitivity.is_finite() || sensitivity <= 0.0 {
            return;
        }
        let yaw_delta = yaw_rate * dt;
        let pitch_delta = pitch_rate * dt;
        if yaw_delta.abs() < 1e-4 && pitch_delta.abs() < 1e-4 {
            return;
        }
        let dx = yaw_delta / sensitivity;
        let dy = -pitch_delta / sensitivity;
        controller.add_mouse_delta((dx, dy), sensitivity);
    }
}

fn default_segments() -> Vec<ScriptSegment> {
    vec![
        ScriptSegment::new(3.5, MovementState::default().with_forward(true), 18.0, 0.0),
        ScriptSegment::new(
            3.0,
            MovementState::default().with_forward(true).with_right(true),
            26.0,
            -6.0,
        ),
        ScriptSegment::new(
            3.5,
            MovementState::default().with_forward(true).with_up(true),
            -22.0,
            4.0,
        ),
        ScriptSegment::new(
            3.0,
            MovementState::default().with_backward(true).with_left(true),
            35.0,
            -8.0,
        ),
        ScriptSegment::new(4.0, MovementState::default().with_up(true), 90.0, 12.0),
    ]
}

#[derive(Default)]
struct BenchmarkMetrics {
    frame_times: Vec<f32>,
    chunk_counts: Vec<usize>,
    timings: TimingStats,
}

impl BenchmarkMetrics {
    fn record(
        &mut self,
        frame_time: f32,
        chunk_count: usize,
        timings: Option<render::RenderTimings>,
    ) {
        if frame_time.is_finite() && frame_time > 0.0 {
            self.frame_times.push(frame_time);
        }
        self.chunk_counts.push(chunk_count);
        if let Some(timing) = timings {
            self.timings.record(timing);
        }
    }

    fn print_summary(
        &self,
        elapsed: f32,
        renderer: RendererKind,
        resolution: (u32, u32),
        present_mode: PresentModeSetting,
        segments: usize,
    ) {
        if self.frame_times.is_empty() {
            println!("Benchmark finished with no recorded frames.");
            return;
        }

        let total_frames = self.frame_times.len();
        let total_time: f32 = self.frame_times.iter().copied().sum();
        let avg_frame = total_time / total_frames as f32;
        let min_frame = self.frame_times.iter().copied().fold(f32::MAX, f32::min);
        let max_frame = self.frame_times.iter().copied().fold(f32::MIN, f32::max);

        let mut sorted = self.frame_times.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p95_index = ((sorted.len() as f32 * 0.95).ceil() as usize).clamp(1, sorted.len()) - 1;
        let p95_frame = sorted[p95_index];

        let average_fps = if total_time > 0.0 {
            total_frames as f32 / total_time
        } else {
            0.0
        };

        let (chunk_min, chunk_max, chunk_avg) = if self.chunk_counts.is_empty() {
            (0usize, 0usize, 0.0)
        } else {
            let min_c = *self.chunk_counts.iter().min().unwrap();
            let max_c = *self.chunk_counts.iter().max().unwrap();
            let avg_c = self.chunk_counts.iter().copied().sum::<usize>() as f32
                / self.chunk_counts.len() as f32;
            (min_c, max_c, avg_c)
        };

        println!(
            "Benchmark complete: {:.1}s, {} frames, {} segments.",
            elapsed, total_frames, segments
        );
        println!(
            "- Renderer: {} @ {}x{} (present: {})",
            renderer.as_str(),
            resolution.0,
            resolution.1,
            present_mode_label(present_mode)
        );
        println!(
            "- Frame ms: avg {:>5.4} | p95 {:>5.4} | min {:>5.4} | max {:>5.4}",
            avg_frame * 1000.0,
            p95_frame * 1000.0,
            min_frame * 1000.0,
            max_frame * 1000.0
        );
        println!(
            "- FPS: avg {:>5.1} | runtime {:.2}s",
            average_fps, total_time
        );
        println!(
            "- Loaded chunks: avg {:>5.1} | min {:>3} | max {:>3}",
            chunk_avg, chunk_min, chunk_max
        );

        if self.timings.samples > 0 {
            let averages = self.timings.averages();
            println!(
                "- Render timings avg ms: total {:>5.4} | prep {:>5.4} | uniforms {:>5.4} | compute {:>5.4} | present {:>5.4}",
                averages.total,
                averages.scene,
                averages.uniforms,
                averages.compute,
                averages.present
            );
            println!(
                "- GPU timings avg ms: compute {:>5.4} | blit {:>5.4}",
                averages.gpu_compute, averages.gpu_present
            );
            println!(
                "- Voxels traced: avg {:>8.0} | max {:>8}",
                averages.voxels_avg, self.timings.voxels_max
            );
        }
    }
}

#[derive(Default)]
struct TimingStats {
    samples: u32,
    total_ms: f64,
    scene_ms: f64,
    uniforms_ms: f64,
    compute_ms: f64,
    present_ms: f64,
    gpu_compute_ms: f64,
    gpu_present_ms: f64,
    voxels_total: u64,
    voxels_max: u32,
}

impl TimingStats {
    fn record(&mut self, timings: render::RenderTimings) {
        self.samples = self.samples.saturating_add(1);
        self.total_ms += timings.total_ms as f64;
        self.scene_ms += timings.scene_ms as f64;
        self.uniforms_ms += timings.uniforms_ms as f64;
        self.compute_ms += timings.compute_ms as f64;
        self.present_ms += timings.present_ms as f64;
        self.gpu_compute_ms += timings.gpu_compute_ms as f64;
        self.gpu_present_ms += timings.gpu_present_ms as f64;
        self.voxels_total = self.voxels_total.saturating_add(timings.voxels as u64);
        self.voxels_max = self.voxels_max.max(timings.voxels);
    }

    fn averages(&self) -> TimingAverages {
        if self.samples == 0 {
            return TimingAverages::default();
        }
        let inv = 1.0 / self.samples as f64;
        TimingAverages {
            total: (self.total_ms * inv) as f32,
            scene: (self.scene_ms * inv) as f32,
            uniforms: (self.uniforms_ms * inv) as f32,
            compute: (self.compute_ms * inv) as f32,
            present: (self.present_ms * inv) as f32,
            gpu_compute: (self.gpu_compute_ms * inv) as f32,
            gpu_present: (self.gpu_present_ms * inv) as f32,
            voxels_avg: self.voxels_total as f64 * inv,
        }
    }
}

#[derive(Default)]
struct TimingAverages {
    total: f32,
    scene: f32,
    uniforms: f32,
    compute: f32,
    present: f32,
    gpu_compute: f32,
    gpu_present: f32,
    voxels_avg: f64,
}

fn present_mode_label(mode: PresentModeSetting) -> &'static str {
    match mode {
        PresentModeSetting::Immediate => "immediate",
        PresentModeSetting::Mailbox => "mailbox",
        PresentModeSetting::VSync => "vsync",
    }
}
