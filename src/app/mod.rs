use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

mod state;

pub async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rustcraft")
        .build(&event_loop)
        .expect("Failed to create window");

    let mut app_state = state::AppState::new(window).await;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app_state.window().id() => {
                if !app_state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput { input, .. } => {
                            if input.state == winit::event::ElementState::Pressed {
                                if let Some(winit::event::VirtualKeyCode::Escape) =
                                    input.virtual_keycode
                                {
                                    if app_state.handle_escape() {
                                        *control_flow = ControlFlow::Exit;
                                    }
                                }
                            }
                        }
                        WindowEvent::Resized(physical_size) => {
                            app_state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            app_state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::DeviceEvent { ref event, .. } => {
                app_state.device_input(event);
            }
            Event::RedrawRequested(window_id) if window_id == app_state.window().id() => {
                app_state.update();
                match app_state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        app_state.resize(app_state.window().inner_size())
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(err) => log::warn!("Render error: {err:?}"),
                }
            }
            Event::MainEventsCleared => {
                state::sleep_on_main_events(&app_state);
                app_state.window().request_redraw();
            }
            Event::LoopDestroyed => {}
            _ => {}
        }
    });
}
