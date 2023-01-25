mod gui;
mod widgets;

use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Fasing")
        .with_inner_size(winit::dpi::PhysicalSize::new(960, 720))
        .build(&event_loop)
        .unwrap();

    let main_widget = widgets::MainWidget::new();
    
    gui::app::run(
        event_loop,
        window,
        main_widget,
    );
}
