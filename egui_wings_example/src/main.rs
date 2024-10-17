use ::winit;
use egui_wgpu::wgpu::*;
use egui_wgpu::*;
use egui_wings::*;
use egui_wings_host::*;
use example_host::*;
use geese::*;
use std::sync::*;
use wings_host::*;
use winit::dpi::*;
use winit::event::*;
use winit::event_loop::*;
use winit::keyboard::*;

include!(concat!(env!("OUT_DIR"), "/example_plugin.rs"));

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(run());
    }
}

pub struct EguiExample;

impl EguiExample {
    fn on_error(&mut self, error: &wings_host::on::Error) {
        panic!("Error occurred:\n{:?}", error.error);
    }
}

impl GeeseSystem for EguiExample {
    const DEPENDENCIES: Dependencies = dependencies().with::<WingsHost<ExampleHostSystems>>();

    const EVENT_HANDLERS: EventHandlers<Self> = event_handlers().with(Self::on_error);

    fn new(_: GeeseContextHandle<Self>) -> Self {
        Self
    }
}

/// The wings host type.
pub struct ExampleHostSystems;

impl Host for ExampleHostSystems {
    // Declare the egui system that should be exported to WASM.
    const SYSTEMS: Systems<Self> = systems().with::<EguiHost>(traits().with::<dyn Egui>());

    const EVENTS: Events<Self> = events().with::<example_host::on::Render>();

    type Engine = wasmtime_runtime_layer::Engine;

    fn create_engine(_: &mut GeeseContextHandle<WingsHost<Self>>) -> Self::Engine {
        wasmtime_runtime_layer::Engine::default()
    }
}

/// Creates the `GeeseContext` that will hold the host plugin systems.
fn create_geese_context() -> GeeseContext {
    let mut ctx = GeeseContext::default();
    ctx.flush().with(geese::notify::add_system::<EguiExample>());

    let mut host = ctx.get_mut::<WingsHost<ExampleHostSystems>>();

    let mut image = WingsImage::default();
    let plugin = host
        .load(EXAMPLE_PLUGIN_WASM)
        .expect("Failed to load plugin.");
    image.add::<ExampleHost>(&plugin);
    host.instantiate(&image);
    drop(host);

    ctx
}

async fn run() {
    let event_loop = EventLoop::new().unwrap();

    let window_attributes =
        winit::window::Window::default_attributes().with_title("A fantastic window!");
    let window = event_loop.create_window(window_attributes).unwrap();

    let window = Arc::new(window);
    let initial_width = 1360;
    let initial_height = 768;
    let _ = window.request_inner_size(PhysicalSize::new(initial_width, initial_height));
    let instance = Instance::new(InstanceDescriptor::default());
    let surface = instance
        .create_surface(window.clone())
        .expect("Failed to create surface!");
    let power_pref = PowerPreference::default();
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: power_pref,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let features = Features::empty();
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                required_features: features,
                required_limits: Default::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let selected_format = TextureFormat::Bgra8UnormSrgb;
    let swapchain_format = swapchain_capabilities
        .formats
        .iter()
        .find(|d| **d == selected_format)
        .expect("failed to select proper surface texture format!");

    let mut config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: *swapchain_format,
        width: initial_width,
        height: initial_height,
        present_mode: PresentMode::AutoVsync,
        desired_maximum_frame_latency: 0,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    let mut egui_renderer = EguiRenderer::new(&device, config.format, None, 1, &window);

    let mut close_requested = false;

    let scale_factor = 1.0;

    let mut ctx = create_geese_context();
    // Set the context that will be exposed to WASM plugins
    ctx.get_mut::<EguiHost>()
        .set_context(egui_renderer.context().clone());

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            winit::event::Event::WindowEvent { event, .. } => {
                egui_renderer.handle_input(&window, &event);

                match event {
                    WindowEvent::CloseRequested => {
                        close_requested = true;
                    }
                    WindowEvent::ModifiersChanged(_) => {}
                    WindowEvent::KeyboardInput {
                        event: kb_event, ..
                    } => {
                        if kb_event.logical_key == winit::keyboard::Key::Named(NamedKey::Escape) {
                            close_requested = true;
                        }
                    }
                    WindowEvent::ActivationTokenDone { .. } => {}
                    WindowEvent::Resized(new_size) => {
                        // Resize surface:
                        config.width = new_size.width;
                        config.height = new_size.height;
                        surface.configure(&device, &config);
                    }
                    WindowEvent::Moved(_) => {}
                    WindowEvent::Destroyed => {}
                    WindowEvent::DroppedFile(_) => {}
                    WindowEvent::HoveredFile(_) => {}
                    WindowEvent::HoveredFileCancelled => {}
                    WindowEvent::Focused(_) => {}
                    WindowEvent::Ime(_) => {}
                    WindowEvent::CursorMoved { .. } => {}
                    WindowEvent::CursorEntered { .. } => {}
                    WindowEvent::CursorLeft { .. } => {}
                    WindowEvent::MouseWheel { .. } => {}
                    WindowEvent::MouseInput { .. } => {}

                    WindowEvent::TouchpadPressure { .. } => {}
                    WindowEvent::AxisMotion { .. } => {}
                    WindowEvent::Touch(_) => {}
                    WindowEvent::ScaleFactorChanged { .. } => {}
                    WindowEvent::ThemeChanged(_) => {}
                    WindowEvent::Occluded(_) => {}
                    WindowEvent::RedrawRequested => {
                        let surface_texture = surface
                            .get_current_texture()
                            .expect("Failed to acquire next swap chain texture");

                        let surface_view = surface_texture
                            .texture
                            .create_view(&TextureViewDescriptor::default());

                        let mut encoder = device
                            .create_command_encoder(&CommandEncoderDescriptor { label: None });

                        let screen_descriptor = ScreenDescriptor {
                            size_in_pixels: [config.width, config.height],
                            pixels_per_point: window.scale_factor() as f32 * scale_factor,
                        };

                        egui_renderer.draw(
                            &device,
                            &queue,
                            &mut encoder,
                            &window,
                            &surface_view,
                            screen_descriptor,
                            |_| {
                                ctx.flush().with(example_host::on::Render);
                            },
                        );

                        queue.submit(Some(encoder.finish()));
                        surface_texture.present();
                        window.request_redraw();
                    }
                    _ => {}
                }
            }

            winit::event::Event::NewEvents(_) => {}
            winit::event::Event::DeviceEvent { .. } => {}
            winit::event::Event::UserEvent(_) => {}
            winit::event::Event::Suspended => {}
            winit::event::Event::Resumed => {}
            winit::event::Event::AboutToWait => {
                if close_requested {
                    elwt.exit()
                }
            }
            winit::event::Event::LoopExiting => {}
            winit::event::Event::MemoryWarning => {}
        }
    });
}

pub struct EguiRenderer {
    state: egui_winit::State,
    renderer: Renderer,
}

impl EguiRenderer {
    pub fn context(&self) -> &egui::Context {
        self.state.egui_ctx()
    }

    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &winit::window::Window,
    ) -> EguiRenderer {
        let egui_context = egui::Context::default();

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            true,
        );

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
        }
    }

    pub fn handle_input(&mut self, window: &winit::window::Window, event: &WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }

    pub fn ppp(&mut self, v: f32) {
        self.state.egui_ctx().set_pixels_per_point(v);
    }

    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &winit::window::Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
        mut run_ui: impl FnMut(&egui::Context),
    ) {
        self.state
            .egui_ctx()
            .set_pixels_per_point(screen_descriptor.pixels_per_point);

        let raw_input = self.state.take_egui_input(window);
        let full_output = self.state.egui_ctx().run(raw_input, |_| {
            run_ui(self.state.egui_ctx());
        });

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }
        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: window_surface_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                label: Some("egui main render pass"),
                occlusion_query_set: None,
            });
            self.renderer
                .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
        }
        //drop(rpass);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }
    }
}
