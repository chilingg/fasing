use eframe::egui;

pub type Children<'a, C, R> = Vec<Box<&'a mut dyn Widget<C, R>>>;

#[allow(unused)]
pub trait Widget<C, R> {
    // Required method

    fn children<'a>(&'a mut self) -> Children<'a, C, R>;

    // Provided method

    fn update(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        core_data: &C,
        run_data: &mut R,
    ) {
    }

    fn update_ui(
        &mut self,
        ui: &mut egui::Ui,
        frame: &mut eframe::Frame,
        core_data: &C,
        run_data: &mut R,
    ) {
        self.children()
            .iter_mut()
            .for_each(|widget| widget.update_ui(ui, frame, core_data, run_data))
    }

    fn start(&mut self, context: &eframe::CreationContext, core_data: &C, run_data: &mut R) {}

    fn finished(&self, core_data: &C, run_data: &mut R) {}

    fn input_process(&mut self, input: &mut egui::InputState, core_data: &C, run_data: &mut R) {}

    fn save(&mut self, storage: &mut dyn eframe::Storage) {}
}
