use std::default::Default;
use std::iter::once;
use std::sync::Arc;

use web_time::Instant;
use wgpu::util::DeviceExt;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

use crate::scene::Scene;

mod aabb;
mod bvh;
mod camera;
mod interval;
mod material;
mod object;
mod scene;
mod texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

const VERT2: &[Vertex] = &[
    Vertex {
        pos: [-1.0, 1.0, 0.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        pos: [-1.0, -1.0, 0.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        pos: [1.0, -1.0, 0.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        pos: [1.0, 1.0, 0.0],
        uv: [1.0, 0.0],
    },
];

impl Vertex {
    const ATTR: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTR,
        }
    }
}

const INDICES: &[u16] = &[0, 1, 3, 1, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    pos: glam::Vec4,
    forward: glam::Vec4,
    right: glam::Vec4,
    up: glam::Vec4,
    fov: f32,
    _pad: [f32; 3],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            pos: glam::vec4(0.0, 0.0, 0.0, 0.0),
            forward: glam::vec4(0.0, 0.0, 0.0, 0.0),
            right: glam::vec4(0.0, 0.0, 0.0, 0.0),
            up: glam::vec4(0.0, 0.0, 0.0, 0.0),
            fov: 45_f32.to_radians(),
            _pad: [0., 0., 0.],
        }
    }

    fn update(&mut self, camera: &camera::Camera) {
        self.pos = camera.pos.extend(1.0);
        self.forward = camera.front.extend(1.0);
        self.right = camera.rght.extend(1.0);
        self.up = camera.actual_up.extend(1.0);

        self.fov = camera.fov;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Settings {
    res: [f32; 2],
    frame: u32,
    _pad: u32,
}

pub struct State {
    window: Arc<winit::window::Window>,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_config: bool,
    pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    settings: Settings,
    settings_buffer: wgpu::Buffer,
    settings_bind_group: wgpu::BindGroup,

    tex_bind_group_layout: wgpu::BindGroupLayout,
    tex_bind_groups: [wgpu::BindGroup; 2],

    camera: camera::Camera,

    bvh_bind_group: wgpu::BindGroup,

    camera_uniform: CameraUniform,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,

    last: Instant,
}

//will be DRY later on
impl State {
    pub async fn new(window: Arc<winit::window::Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            //#[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            //#[cfg(target_arch = "wasm32")]
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        //chosen gpu by instance
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::defaults(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]); //srgb mentioned

        //https://github.com/gfx-rs/wgpu/issues/3976
        //if the backend is webgpu, srgb handling is different (?)
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: format.remove_srgb_suffix(), //wgpu::TextureFormat::Rgba8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![format.add_srgb_suffix()],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };

        let settings = Settings {
            res: [config.width as f32, config.height as f32],
            frame: 0,
            _pad: 0,
        };

        let settings_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("settings"),
            contents: bytemuck::cast_slice(&[settings]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let settings_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],

                label: Some("settings_layout"),
            });

        let settings_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &settings_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: settings_buffer.as_entire_binding(),
            }],

            label: Some("settings_group"),
        });

        //camera starts

        //[26.885092, 20.502254, 22.690096]
        let camera = camera::Camera::new(glam::vec3(26.88, 20.50, 22.69),   3.79, -0.43);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cam"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],

                label: Some("cam_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],

            label: Some("cam_group"),
        });

        //camera ends

        //PING PONG

        let tex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("tex_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba32Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        let tex_bind_groups = temp(&device, &config, &tex_bind_group_layout);

        //PING PoNG

        //BVH related stuff

        let (bvh_cont, sphere_cont, triangle_cont, material_cont) = Scene::test_model();

        let bvh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bvh_buf"),
            contents: bytemuck::cast_slice(&bvh_cont),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let sphere_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sphere_buf"),
            contents: bytemuck::cast_slice(&sphere_cont),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let triangle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("triangle_buf"),
            contents: bytemuck::cast_slice(&triangle_cont),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("material_buf"),
            contents: bytemuck::cast_slice(&material_cont),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bvh_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bvh_layout"),

            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bvh_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bvh_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bvh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sphere_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: triangle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: material_buffer.as_entire_binding(),
                },
            ],

            label: Some("bvh_group"),
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("layout"),
            bind_group_layouts: &[
                Some(&settings_bind_group_layout),
                Some(&camera_bind_group_layout),
                Some(&tex_bind_group_layout),
                Some(&bvh_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format.add_srgb_suffix(), //wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),

            //how to interpret
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },

            depth_stencil: None,

            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },

            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertices"),
            contents: bytemuck::cast_slice(VERT2),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indices"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            is_config: false,
            pipeline,

            vertex_buffer,
            index_buffer,

            settings,
            settings_buffer,
            settings_bind_group,

            tex_bind_group_layout,
            tex_bind_groups,

            camera,
            camera_uniform,
            camera_bind_group,
            camera_buffer,

            bvh_bind_group,

            last: Instant::now(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_config = true;

            self.tex_bind_groups = temp(&self.device, &self.config, &self.tex_bind_group_layout);
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_config {
            //resize fires at the beginning
            return Ok(());
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }

            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(());
            }

            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }

            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("lost");
            }
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_bind_group(0, &self.settings_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(
            2,
            &self.tex_bind_groups[(self.settings.frame % 2) as usize],
            &[],
        );
        render_pass.set_bind_group(3, &self.bvh_bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);

        drop(render_pass);
        self.queue.submit(once(encoder.finish()));
        self.queue.present(output);

        Ok(())
    }

    fn update(&mut self) {
        let cur = Instant::now();
        let delta = cur - self.last;
        self.last = cur;
        let same = self.camera.update(delta.as_secs_f32());

        //println!("{}, {}, {}", self.camera.pos, self.camera.yaw, self.camera.pitch);

        self.camera_uniform.update(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        self.settings = Settings {
            res: [self.config.width as f32, self.config.height as f32],
            frame: if same == true {
                self.settings.frame + 1
            } else {
                1
            },
            _pad: 0,
        };

        self.queue.write_buffer(
            &self.settings_buffer,
            0,
            bytemuck::cast_slice(&[self.settings]),
        );
    }

    fn handle_key(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        code: winit::keyboard::KeyCode,
        press: bool,
    ) {
        match (code, press) {
            (winit::keyboard::KeyCode::Escape, true) => {
                self.window.set_cursor_visible(true); //event_loop.exit(),
                self.window
                    .set_cursor_grab(winit::window::CursorGrabMode::None)
                    .unwrap();
            }
            _ => self.camera.handle_key(code, press),
        }
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>, //big mystery
    state: Option<State>,
}

impl App {
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &winit::event_loop::EventLoop<State>,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl winit::application::ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut wa = winit::window::Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::{HtmlCanvasElement, window};
            use winit::platform::web::WindowAttributesExtWebSys;

            let window = window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id("canvas").unwrap_throw();
            let element: HtmlCanvasElement = canvas.unchecked_into();
            wa = wa.with_canvas(Some(element));
        }

        let window = Arc::new(event_loop.create_window(wa).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(State::new(window).await.expect("ggs"))
                            .is_ok()
                    )
                });
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, mut event: State) {
        event.window.request_redraw();
        let size = event.window.inner_size();
        event.resize(size.width, size.height);
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        //this is needed because there can be a window event before state is Some
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        use winit::event::WindowEvent::*;

        match event {
            CloseRequested => event_loop.exit(),
            Resized(size) => state.resize(size.width, size.height),
            RedrawRequested => {
                state.update();

                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            }

            KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),

            MouseInput { .. } => {
                state
                    .window
                    .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                    .unwrap();
                state.window.set_cursor_visible(false);
            }

            MouseWheel {
                device_id,
                delta,
                phase,
            } => {
                state.camera.handle_wheel(delta);
            }

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            winit::event::DeviceEvent::MouseMotion { delta } => {
                state.camera.handle_dir(delta.0, delta.1)
            }

            _ => {}
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }

    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new();
        event_loop.run_app(&mut app)?;
    }

    //if we need proxy
    #[cfg(target_arch = "wasm32")]
    {
        let app = App::new(&event_loop);
        event_loop.spawn_app(app);
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}

fn temp(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    layout: &wgpu::BindGroupLayout,
) -> [wgpu::BindGroup; 2] {
    let size = wgpu::Extent3d {
        width: config.width.max(1),
        height: config.height.max(1),
        depth_or_array_layers: 1,
    };

    let desc = wgpu::TextureDescriptor {
        label: Some("tex_desc"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    };

    let tex1 = device.create_texture(&desc);
    let tex2 = device.create_texture(&desc);

    let view1 = tex1.create_view(&wgpu::TextureViewDescriptor::default());
    let view2 = tex2.create_view(&wgpu::TextureViewDescriptor::default());

    let tex_bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view1),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view2),
            },
        ],

        label: Some("tex_group"),
    });

    let tex_bind_group2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view2),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view1),
            },
        ],

        label: Some("tex_group"),
    });

    [tex_bind_group1, tex_bind_group2]
}
