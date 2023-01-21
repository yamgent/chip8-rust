use chip8_rust::graphics::Graphics;
use wgpu::SurfaceError;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

struct Application {
    window_size: PhysicalSize<u32>,
    graphics: Graphics,
}

impl Application {
    async fn new(window: &Window) -> Self {
        let window_size = window.inner_size();
        let graphics = Graphics::new(window).await;

        Self {
            window_size,
            graphics,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.window_size = new_size;
        self.graphics.resize(new_size);
    }

    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self) -> Result<(), SurfaceError> {
        self.graphics.render()
    }
}

async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Failed to build window");

    let mut application = Application::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        // it is ok to wait until we have the next re-draw, no need to keep
        // spinning
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !application.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            application.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            application.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                application.update();
                match application.render() {
                    Ok(_) => {}
                    Err(SurfaceError::Lost) => application.resize(application.window_size),
                    Err(SurfaceError::OutOfMemory) => {
                        log::error!("Surface ran out of memory!");
                        *control_flow = ControlFlow::Exit;
                    }
                    Err(e) => {
                        // all other errors (Outdated, Timeout) should be resolved by the next
                        // frame
                        log::error!("Render error: {:?}", e);
                    }
                }
            }
            Event::MainEventsCleared => {
                // while the library docs say that a redraw always happens after this event, my
                // experiment so far contradicts that claim. So just request redraw always.
                window.request_redraw();
            }
            _ => {}
        }
    });
}

fn main() {
    pollster::block_on(run());
}
