use eframe::egui;

pub type Children<'a, C, U> = Vec<Box<&'a mut dyn Widget<C, U>>>;

#[allow(unused)]
pub trait Widget<C, U> {
    // Required method

    fn children<'a>(&'a mut self) -> Children<'a, C, U>;

    // Provided method

    fn update(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        core_data: &C,
        run_data: &mut U,
    ) {
    }

    fn update_ui(
        &mut self,
        ui: &mut egui::Ui,
        frame: &mut eframe::Frame,
        core_data: &C,
        run_data: &mut U,
    ) {
        self.children()
            .iter_mut()
            .for_each(|widget| widget.update_ui(ui, frame, core_data, run_data))
    }

    fn start(&mut self, context: &eframe::CreationContext, core_data: &C, run_data: &mut U) {}

    fn finished(&self, core_data: &C, run_data: &mut U) {}

    fn input_process(&mut self, input: &mut egui::InputState, core_data: &C, run_data: &mut U) {}

    fn recursion_start(
        &mut self,
        context: &eframe::CreationContext,
        core_data: &C,
        run_data: &mut U,
    ) {
        self.start(context, core_data, run_data);
        self.children()
            .iter_mut()
            .for_each(|widget| widget.recursion_start(context, core_data, run_data));
    }

    fn recursion_input_process(
        &mut self,
        input: &mut egui::InputState,
        core_data: &C,
        run_data: &mut U,
    ) {
        self.children()
            .iter_mut()
            .for_each(|widget| widget.recursion_input_process(input, core_data, run_data));
        self.input_process(input, core_data, run_data);
    }
}
