use super::ExtendWorks;
use super::MeteCompWorks;
use crate::{app::recursion_start, prelude::*};

pub struct Center {
    pub current: usize,
    mete_comp_works: MeteCompWorks,
    extend_works: ExtendWorks,
}

impl std::default::Default for Center {
    fn default() -> Self {
        Self {
            current: 0,
            mete_comp_works: Default::default(),
            extend_works: Default::default(),
        }
    }
}

impl Center {
    fn get_children(&mut self, index: usize) -> &mut dyn Widget<CoreData, RunData> {
        match index {
            0 => &mut self.mete_comp_works,
            1 => &mut self.extend_works,
            _ => unreachable!(),
        }
    }

    fn current_children(&mut self) -> &mut dyn Widget<CoreData, RunData> {
        self.get_children(self.current)
    }
}

impl Widget<CoreData, RunData> for Center {
    fn children(&mut self) -> Children {
        vec![Box::new(self.current_children())]
    }

    fn start(
        &mut self,
        context: &eframe::CreationContext,
        core_data: &CoreData,
        run_data: &mut RunData,
    ) {
        (0..=1).into_iter().for_each(|i| {
            if i != self.current {
                let child = self.get_children(i);
                recursion_start(child, context, core_data, run_data);
            }
        })
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        core_data: &CoreData,
        run_data: &mut RunData,
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(ctx.style().visuals.faint_bg_color))
            .show(ctx, |ui| {
                self.children()[0].update_ui(ui, frame, core_data, run_data)
            });
    }
}
