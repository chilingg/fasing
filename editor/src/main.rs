use fasing_editor::App;

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        hardware_acceleration: eframe::HardwareAcceleration::Required,
        initial_window_size: Some(egui::vec2(960.0, 720.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Fasing",
        options,
        Box::new(|context| {
            let mut app = Box::new(App::new());
            app.start(context);

            app
        }),
    )
}
