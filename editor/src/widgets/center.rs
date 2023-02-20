use super::mete_comp_works::MeteCompWorks;
use crate::prelude::*;

pub struct Center {
    current: usize,
    mete_comp_works: MeteCompWorks,
}

impl std::default::Default for Center {
    fn default() -> Self {
        Self {
            current: 0,
            mete_comp_works: MeteCompWorks::default(),
        }
    }
}

impl Widget<CoreData, RunData> for Center {
    fn children(&mut self) -> Children {
        vec![Box::new(&mut self.mete_comp_works)]
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        core_data: &CoreData,
        run_data: &mut RunData,
    ) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none().fill(ctx.style().visuals.faint_bg_color), // .inner_margin(egui::style::Margin::symmetric(12.0, 6.0)),
            )
            .show(ctx, |ui| {
                self.mete_comp_works
                    .update_ui(ui, frame, core_data, run_data)
            });
    }
}
