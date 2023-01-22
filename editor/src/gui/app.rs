use winit::{
    event_loop::{ ControlFlow, EventLoop },
    window::Window,
};

use super::event::*;

#[allow(unused)]
pub trait Widget {
    fn start(&mut self, event: StartEvent) {}
    fn finish(&mut self, event: FinishEvent) {}
    fn update(&mut self, event: UpdateEvent) {}
    fn process(&mut self, event: ProcessEvent) -> bool { false }
}

use egui_wgpu::renderer::{
    RenderPass,
    ScreenDescriptor
};

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

pub struct AppState {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub window: Window,

    pub egui: EguiState,
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

        let estate = EguiState::new(&event_loop, &device, config.format);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            egui: estate,
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
) -> !{
    env_logger::init();
    
    let mut state = AppState::new(&event_loop, window);
    main_widget.start(StartEvent { app_state: &mut state });

    event_loop.run(move |event, _, control_flow| {
        match event {
            we::Event::WindowEvent { window_id, ref event }
                if window_id == state.window.id()
                && !state.egui.state.on_event(&state.egui.ctx, event)
                && !main_widget.process(ProcessEvent {
                    app_state: &mut state,
                    window_event: &event,
                })
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
                // Begin to draw the UI frame.
                let raw_input = state.egui.state.take_egui_input(&state.window);
                let full_output = state.egui.ctx.run(raw_input, |ctx| {
                    main_widget.update(UpdateEvent { egui_ctx: ctx });
                });
        
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