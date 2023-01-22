use std::{collections::HashMap, fs::File, io::Read, sync::mpsc::Sender};

use chip8_rust::{
    cpu::{Cpu, CpuIoEvents},
    graphics::Graphics,
};
use clap::Parser;
use once_cell::sync::Lazy;
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
    cpu_io_sender: Sender<CpuIoEvents>,
}

const KEYMAP: Lazy<HashMap<VirtualKeyCode, u8>> = Lazy::new(|| {
    HashMap::from([
        (VirtualKeyCode::Key1, 0x1),
        (VirtualKeyCode::Key2, 0x2),
        (VirtualKeyCode::Key3, 0x3),
        (VirtualKeyCode::Key4, 0xC),
        (VirtualKeyCode::Q, 0x4),
        (VirtualKeyCode::W, 0x5),
        (VirtualKeyCode::E, 0x6),
        (VirtualKeyCode::R, 0xD),
        (VirtualKeyCode::A, 0x7),
        (VirtualKeyCode::S, 0x8),
        (VirtualKeyCode::D, 0x9),
        (VirtualKeyCode::F, 0xE),
        (VirtualKeyCode::Z, 0xA),
        (VirtualKeyCode::X, 0x0),
        (VirtualKeyCode::C, 0xB),
        (VirtualKeyCode::V, 0xF),
    ])
});

impl Application {
    async fn new(window: &Window, program: Vec<u8>) -> Self {
        let window_size = window.inner_size();

        let (screen_update_sender, screen_update_receiver) = std::sync::mpsc::channel();
        let (cpu_io_sender, cpu_io_receiver) = std::sync::mpsc::channel();

        let graphics = Graphics::new(window, screen_update_receiver).await;

        // TODO: A better way of handling the program, rather than just using unwrap?
        std::thread::spawn(move || {
            let mut cpu = Cpu::new(program, screen_update_sender, cpu_io_receiver).unwrap();
            cpu.run();
        });

        Self {
            window_size,
            graphics,
            cpu_io_sender,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.window_size = new_size;
        self.graphics.resize(new_size);
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        if let WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    state,
                    virtual_keycode: Some(virtual_keycode),
                    ..
                },
            ..
        } = event
        {
            match KEYMAP.get(&virtual_keycode) {
                Some(value) => {
                    self.cpu_io_sender
                        .send(match state {
                            ElementState::Pressed => CpuIoEvents::KeyPressed(*value),
                            ElementState::Released => CpuIoEvents::KeyReleased(*value),
                        })
                        .expect("Cannot send IO to cpu");
                    true
                }
                None => false,
            }
        } else {
            false
        }
    }

    // TODO: Is this method redundant?
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
