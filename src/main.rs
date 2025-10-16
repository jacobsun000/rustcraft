mod app;
mod block;
mod camera;
mod config;
mod fps;
mod input;
mod render;
mod text;
mod texture;
mod world;

fn main() {
    env_logger::init();
    pollster::block_on(app::run());
}
