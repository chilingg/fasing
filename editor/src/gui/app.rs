use winit::{
    event_loop::{ ControlFlow, EventLoop },
    window::Window,
};

pub mod we {
    pub use winit::event::*;
}

use fasing::{
    construct,
    fas_file,
};

pub type Children<'a> = Vec<&'a mut Box<dyn Widget>>;
pub type Task = Box<dyn FnOnce(&mut AppState)>;

use egui_wgpu::renderer::{
    RenderPass,
    ScreenDescriptor
};

use std::{
    time,
    sync::{ Arc, Mutex },
    rc::Rc,
    cell::RefCell,
};

#[derive(Default)]
pub struct WidgetData {
    pub open: bool
}

impl WidgetData {
    pub fn from_open(open: bool) -> Self {
        Self { open }
    }
}

#[allow(unused)]
pub trait Widget {
    fn children(&mut self) -> Children;
    fn widget_data(&mut self) -> Option<&mut WidgetData> { None }

    fn start(&mut self, app_state: &mut AppState) {}
    fn update(&mut self, ctx: &egui::Context, queue: &mut Vec<Task>) {}
    fn process(&mut self, window_event: &we::WindowEvent, app_state: &mut AppState) -> bool { false }
    
    fn recursion_start(&mut self, app_state: &mut AppState) {
        self.start(app_state);
        self.children().iter_mut().for_each(|widget| widget.recursion_start(app_state));
    }

    fn recursion_update(&mut self, ctx: &egui::Context, queue: &mut Vec<Task>) {
        self.update(ctx, queue);
        self.children().iter_mut().for_each(|widget| widget.recursion_update(ctx, queue));
    }

    fn recursion_process(&mut self, window_event: &we::WindowEvent, app_state: &mut AppState) -> bool {
        let mut ok = false;
        for child in self.children() {
            ok = child.recursion_process(window_event, app_state);
            if ok { break; }
        }

        if !ok {
            ok = self.process(window_event, app_state);
        }

        ok
    }
}

pub fn widget_box<'a, T: Widget + 'a>(widget: T) -> Box<dyn Widget + 'a> {
    Box::new(widget)
}

pub struct EguiState {
    pub ctx: egui::Context,
    state: egui_winit::State,
    rpass: egui_wgpu::renderer::RenderPass,
    output_data: Option<(egui::TexturesDelta, Vec<egui::ClippedPrimitive>)>,
}

impl EguiState {
    pub fn new<T>(
        event_loop: &EventLoop<T>,
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat)
    -> Self {
        Self {
            state: egui_winit::State::new(event_loop),
            ctx: egui::Context::default(),
            rpass: RenderPass::new(device, output_format, 1),
            output_data: None,
        }
    }
}

pub struct CoreData {
    pub construction: construct::char_construct::Table,
}

impl CoreData {
    pub fn new() -> Self {
        Self {
            construction: construct::fasing_1_0::generate_table(),
        }
    }
}

pub type UserData = fas_file::FasFile;

pub struct AppState {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub window: Window,

    pub egui: EguiState,

    pub core_data: Rc<CoreData>,
    pub user_data: Rc<RefCell<UserData>>,
}

impl AppState {
    pub fn new<T>(event_loop: &EventLoop<T>, window: Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(&window) };
        // 适配器，指向实际显卡的一个handle
        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(), // LowPower低功耗，HighPerfirnabce高性能（独显）
                compatible_surface: Some(&surface), // 兼容传入的surface
                force_fallback_adapter: false, // 是否强制wgpu选择某个能在所有硬件上工作的适配器（软渲染系统）
            }
        )).expect("Couldn't create the adapter!");
        // let adapter = instance
        //     .enumerate_adapters(wgpu::Backends::all())
        //     .filter(|adapter| {
        //         // 检查该适配器是否支持我们的 surface
        //         surfaces.get_preferred_format(&adapter).is_some()
        //     })
        //     .next()
        //     .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None, // 是否追踪APIg调用路径
        )).expect("Couldn't create the device!");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::all(),
            format: *surface.get_supported_formats(&adapter).first().unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo, // VSync
            // alpha_mode: *surface.get_supported_alpha_modes(&adapter).first().unwrap(),
        };
        surface.configure(&device, &config);

        let egui = EguiState::new(&event_loop, &device, config.format);

        let core_data = Rc::new(CoreData::new());
        let user_data = Rc::new(RefCell::new(UserData::from_template_fasing_1_0()));

        Self {
            window,
            surface,
            device,
            queue,
            config,
            egui,
            core_data,
            user_data
        }
    }

    pub fn on_resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width * new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::new(self.config.width, self.config.height)
    }

    pub fn get_screen_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        
        Ok(output)
    }
}

pub fn run<E, W: Widget + 'static>(
    event_loop: EventLoop<E>,
    window: Window,
    mut main_widget: W
) -> ! {
    env_logger::init();
    
    let mut state = AppState::new(&event_loop, window);
    main_widget.recursion_start(&mut state);

    let animation_timer1: Arc<Mutex<Option<time::Instant>>> = Arc::new(Mutex::new(None));
    let animation_timer2 = animation_timer1.clone();
    state.egui.ctx.set_request_repaint_callback(move || {
        animation_timer2.lock().unwrap().replace(time::Instant::now());
    });

    event_loop.run(move |event, _, control_flow| {
        {
            let mut timer = animation_timer1.lock().unwrap();
            if let Some(now) = timer.as_ref() {
                if now.elapsed().as_secs_f32() > state.egui.ctx.style().animation_time {
                    *timer = None;
                }
                control_flow.set_poll();
            } else {
                control_flow.set_wait();
            };
        }

        match event {
            we::Event::WindowEvent { window_id, ref event }
                if window_id == state.window.id()
                && !state.egui.state.on_event(&state.egui.ctx, event)
                && !main_widget.recursion_process(event, &mut state)
                => {
                match event {
                    we::WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    we::WindowEvent::Resized(physical_size) => state.on_resize(*physical_size),
                    we::WindowEvent::ScaleFactorChanged { new_inner_size, .. } => state.on_resize(**new_inner_size),
                    _ => {}
                }
            },
            we::Event::RedrawRequested(window_id) if window_id == state.window.id() => {
                match state.get_screen_texture() {
                    Ok(output) => {
                        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
                        if let Some((textures_delta, paint_jobs)) = state.egui.output_data.take() {
                            let mut encoder = state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("encoder"),
                            });
                    
                            // Upload all resources for the GPU.
                            let screen_descriptor = ScreenDescriptor {
                                size_in_pixels: [state.config.width, state.config.height],
                                pixels_per_point: state.egui.state.pixels_per_point(),
                            };
                            for (id, ref image_delta) in textures_delta.set {
                                state.egui.rpass.update_texture(&state.device, &state.queue, id, image_delta);
                            }
                            state.egui.rpass.update_buffers(&state.device, &state.queue, &paint_jobs, &screen_descriptor);
                    
                            // Record all render passes.
                            state.egui.rpass.execute(
                                &mut encoder,
                                &view,
                                &paint_jobs,
                                &screen_descriptor,
                                None,
                            );
                            // Submit the commands.
                            state.queue.submit(std::iter::once(encoder.finish()));
                    
                            for id in &textures_delta.free {
                                state.egui.rpass.free_texture(id);
                            }        
                        }

                        output.present();
                    },
                    // 如果发生上下文丢失，就重新配置 surface
                    Err(wgpu::SurfaceError::Lost) => state.on_resize(state.size()),
                    // 系统内存不足，此时应该退出
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // 所有其他错误（如过时、超时等）都应在下一帧解决
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            we::Event::MainEventsCleared => {
                let mut queue = vec![];

                // Begin to draw the UI frame.
                let raw_input = state.egui.state.take_egui_input(&state.window);
                let full_output = state.egui.ctx.run(raw_input, |ctx| {
                    main_widget.recursion_update(ctx, &mut queue);
                });

                queue.into_iter().for_each(|task| task(&mut state));
        
                // End the UI frame. We could now handle the output and draw the UI with the backend.
                let paint_jobs = state.egui.ctx.tessellate(full_output.shapes);
        
                state.egui.output_data = Some((full_output.textures_delta, paint_jobs));
                state.egui.state.handle_platform_output(&state.window, &state.egui.ctx, full_output.platform_output);

                // 除非手动请求，否则 RedrawRequested 只会触发一次
                state.window.request_redraw();
            }
            _ => {}
        }
    })
}