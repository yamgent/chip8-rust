use std::cmp::Ordering;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    AddressMode, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferAddress, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, Extent3d, Face,
    Features, FilterMode, FragmentState, FrontFace, ImageCopyTexture, ImageDataLayout, IndexFormat,
    Instance, Limits, LoadOp, MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor,
    PolygonMode, PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, Surface, SurfaceConfiguration, SurfaceError, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureViewDescriptor, TextureViewDimension, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
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
    color: wgpu::Color,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_indices: u32,
    ratio_buffer: Buffer,
    ratio_bind_group: BindGroup,
    screen_texture: Texture,
    screen_texture_bind_group: BindGroup,
}

fn calculate_screen_ratio(size: &PhysicalSize<u32>) -> [f32; 2] {
    match (size.height * 2).cmp(&size.width) {
        Ordering::Equal => [1.0, 1.0],
        Ordering::Greater => [1.0, (size.width as f32 * 0.5) / (size.height as f32)],
        Ordering::Less => [(size.height as f32 * 2.0) / (size.width as f32), 1.0],
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

const SCREEN_VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-1.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
];
const SCREEN_INDICES: [u16; 6] = [0, 1, 3, 3, 1, 2];

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

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&SCREEN_VERTICES),
            usage: BufferUsages::VERTEX,
        });
        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                },
            ],
        };

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&SCREEN_INDICES),
            usage: BufferUsages::INDEX,
        });
        let num_indices = SCREEN_INDICES.len() as u32;

        let ratio_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Ratio Buffer"),
            contents: bytemuck::cast_slice(&calculate_screen_ratio(&size)),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let ratio_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("ratio_bind_group_layout"),
        });
        let ratio_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &ratio_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: ratio_buffer.as_entire_binding(),
            }],
            label: Some("ratio_bind_group"),
        });

        let screen_texture_size = Extent3d {
            width: 64,
            height: 32,
            depth_or_array_layers: 1,
        };
        let screen_texture = device.create_texture(&TextureDescriptor {
            label: Some("Screen Texture"),
            size: screen_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        });

        let mut initial_pixels = std::iter::repeat(255u8)
            .take((4 * screen_texture_size.width * screen_texture_size.height) as usize)
            .collect::<Vec<_>>();

        initial_pixels[0] = 0;
        initial_pixels[1] = 0;
        initial_pixels[2] = 0;

        initial_pixels[4] = 255;
        initial_pixels[5] = 0;
        initial_pixels[6] = 0;

        initial_pixels[8] = 0;
        initial_pixels[9] = 255;
        initial_pixels[10] = 0;

        initial_pixels[12] = 0;
        initial_pixels[13] = 0;
        initial_pixels[14] = 255;

        initial_pixels[(4 * screen_texture_size.width as usize) - 4] = 255;
        initial_pixels[(4 * screen_texture_size.width as usize) - 3] = 255;
        initial_pixels[(4 * screen_texture_size.width as usize) - 2] = 0;

        initial_pixels
            [(4 * screen_texture_size.width as usize * screen_texture_size.height as usize) - 4] =
            0;
        initial_pixels
            [(4 * screen_texture_size.width as usize * screen_texture_size.height as usize) - 3] =
            255;
        initial_pixels
            [(4 * screen_texture_size.width as usize * screen_texture_size.height as usize) - 2] =
            255;

        queue.write_texture(
            ImageCopyTexture {
                texture: &screen_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &initial_pixels,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * screen_texture_size.width),
                rows_per_image: std::num::NonZeroU32::new(screen_texture_size.height),
            },
            screen_texture_size,
        );

        let screen_texture_view = screen_texture.create_view(&TextureViewDescriptor::default());
        let screen_texture_sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let screen_texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("screen_texture_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let screen_texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("screen_texture_bind_group"),
            layout: &screen_texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&screen_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&screen_texture_sampler),
                },
            ],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&ratio_bind_group_layout, &screen_texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            color: wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            ratio_buffer,
            ratio_bind_group,
            screen_texture,
            screen_texture_bind_group,
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
            self.queue.write_buffer(
                &self.ratio_buffer,
                0,
                bytemuck::cast_slice(&calculate_screen_ratio(&self.size)),
            );
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.color.r = position.x / self.size.width as f64;
                self.color.g = position.y / self.size.height as f64;
                true
            }
            _ => false,
        }
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
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.color),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.ratio_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.set_bind_group(1, &self.screen_texture_bind_group, &[]);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
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
