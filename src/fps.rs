#[derive(Default)]
pub struct FpsCounter {
    elapsed: f32,
    frames: u32,
    fps: f32,
}

impl FpsCounter {
    pub fn update(&mut self, dt: f32) -> f32 {
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
