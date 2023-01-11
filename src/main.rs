use wgpu::{
    Backends, CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, Features,
    Instance, Limits, LoadOp, Operations, PowerPreference, PresentMode, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Surface,
    SurfaceConfiguration, SurfaceError, TextureUsages, TextureViewDescriptor,
};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

struct Application {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
}

impl Application {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = Instance::new(Backends::all());

        // TODO: The other safety, window must be valid for entire lifetime, is not really
        // guaranteed. See whether we can provide a better guarantee.
        // SAFETY: Called on main thread.
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Cannot find a graphics card for render!");

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: Features::empty(),
                    limits: Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .expect("Cannot find a graphics device to render on!");

        if size.width == 0 || size.height == 0 {
            panic!(
                "Window's width or height is 0, this is not allowed. Size = {} x {}",
                size.width, size.height
            );
        }

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
        };

        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            log::warn!(
                "window inner size cannot be 0! Current size = {} x {}",
                new_size.width,
                new_size.height
            );
        }
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
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
                    Err(SurfaceError::Lost) => application.resize(application.size),
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
