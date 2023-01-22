use std::{cmp::Ordering, sync::mpsc::Receiver};

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
use winit::{dpi::PhysicalSize, window::Window};

use crate::cpu::CpuScreenMem;

const SCREEN_PX_WIDTH: usize = 64;
const SCREEN_PX_HEIGHT: usize = 32;
const SCREEN_PX_STRIDE: usize = 4;

pub struct Graphics {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    window_size: PhysicalSize<u32>,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_indices: u32,
    ratio_buffer: Buffer,
    ratio_bind_group: BindGroup,
    screen_texture_size: Extent3d,
    screen_texture: Texture,
    screen_texture_bind_group: BindGroup,
    screen_update_receiver: Receiver<CpuScreenMem>,
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

// must only be created and maintained by the main thread
impl Graphics {
    pub async fn new(window: &Window, screen_update_receiver: Receiver<CpuScreenMem>) -> Self {
        let window_size = window.inner_size();

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

        if window_size.width == 0 || window_size.height == 0 {
            panic!(
                "Window's width or height is 0, this is not allowed. Size = {} x {}",
                window_size.width, window_size.height
            );
        }

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: window_size.width,
            height: window_size.height,
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
            contents: bytemuck::cast_slice(&calculate_screen_ratio(&window_size)),
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
            width: SCREEN_PX_WIDTH as u32,
            height: SCREEN_PX_HEIGHT as u32,
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
            .take(
                (SCREEN_PX_STRIDE as u32 * screen_texture_size.width * screen_texture_size.height)
                    as usize,
            )
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

        initial_pixels[(SCREEN_PX_STRIDE * screen_texture_size.width as usize) - 4] = 255;
        initial_pixels[(SCREEN_PX_STRIDE * screen_texture_size.width as usize) - 3] = 255;
        initial_pixels[(SCREEN_PX_STRIDE * screen_texture_size.width as usize) - 2] = 0;

        initial_pixels[(SCREEN_PX_STRIDE
            * screen_texture_size.width as usize
            * screen_texture_size.height as usize)
            - SCREEN_PX_STRIDE] = 0;
        initial_pixels[(SCREEN_PX_STRIDE
            * screen_texture_size.width as usize
            * screen_texture_size.height as usize)
            - 3] = 255;
        initial_pixels[(SCREEN_PX_STRIDE
            * screen_texture_size.width as usize
            * screen_texture_size.height as usize)
            - 2] = 255;

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
                bytes_per_row: std::num::NonZeroU32::new(
                    SCREEN_PX_STRIDE as u32 * screen_texture_size.width,
                ),
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
            window_size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            ratio_buffer,
            ratio_bind_group,
            screen_texture_size,
            screen_texture,
            screen_texture_bind_group,
            screen_update_receiver,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            log::warn!(
                "window inner size cannot be 0! Current size = {} x {}",
                new_size.width,
                new_size.height
            );
            return;
        }

        self.window_size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;

        self.surface.configure(&self.device, &self.config);
        self.queue.write_buffer(
            &self.ratio_buffer,
            0,
            bytemuck::cast_slice(&calculate_screen_ratio(&self.window_size)),
        );
    }

    fn handle_screen_updates(&mut self) {
        // TODO: Can this be improved for performance?
        let mut final_update = None;

        while let Ok(update) = self.screen_update_receiver.try_recv() {
            final_update = Some(update);
        }

        if let Some(update) = final_update {
            let mut final_pixels: Vec<u8> = Vec::with_capacity(
                (SCREEN_PX_STRIDE as u32
                    * self.screen_texture_size.width
                    * self.screen_texture_size.height) as usize,
            );
            update.iter().for_each(|pixels| {
                let mut mask = 1u64 << 63;
                while mask > 0 {
                    if pixels & mask != 0 {
                        final_pixels.push(255);
                        final_pixels.push(255);
                        final_pixels.push(255);
                        final_pixels.push(255);
                    } else {
                        final_pixels.push(0);
                        final_pixels.push(0);
                        final_pixels.push(0);
                        final_pixels.push(255);
                    }
                    mask >>= 1;
                }
            });

            self.queue.write_texture(
                ImageCopyTexture {
                    texture: &self.screen_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &final_pixels,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: std::num::NonZeroU32::new(
                        SCREEN_PX_STRIDE as u32 * self.screen_texture_size.width,
                    ),
                    rows_per_image: std::num::NonZeroU32::new(self.screen_texture_size.height),
                },
                self.screen_texture_size,
            );
        }
    }

    pub fn render(&mut self) -> Result<(), SurfaceError> {
        self.handle_screen_updates();

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
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
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
