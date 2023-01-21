use std::{fs::File, io::Read};

use chip8_rust::{cpu::Cpu, graphics::Graphics};
use clap::Parser;
use wgpu::SurfaceError;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    path: String,
}

struct Application {
    window_size: PhysicalSize<u32>,
    graphics: Graphics,
}

impl Application {
    async fn new(window: &Window, program: Vec<u8>) -> Self {
        let window_size = window.inner_size();

        let (tx, rx) = std::sync::mpsc::channel();

        let graphics = Graphics::new(window, rx).await;
        // TODO: A better way of handling the program, rather than just using unwrap?
        std::thread::spawn(move || {
            let mut cpu = Cpu::new(program, tx).unwrap();
            cpu.run();
        });

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
    let args = Args::parse();

    env_logger::init();

    let mut program_file = match File::open(&args.path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Cannot open program {:?}: {:?}", args.path, err);
            return;
        }
    };

    let mut program = Vec::new();
    if let Err(err) = program_file.read_to_end(&mut program) {
        eprintln!("Read program {:?} failed: {:?}", args.path, err);
        return;
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Failed to build window");

    let mut application = Application::new(&window, program).await;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

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
