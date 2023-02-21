#[derive(Clone)]
pub enum MsgType {
    Info(String),
    Warning(String),
    Error(String),
}

pub struct MessagePanel {
    y_offset: f32,
    messages: Vec<MsgType>,
}

impl Default for MessagePanel {
    fn default() -> Self {
        Self {
            y_offset: 0.0,
            messages: vec![],
        }
    }
}

impl MessagePanel {
    pub fn add_info<T: Into<String>>(&mut self, msg: T) {
        self.messages.push(MsgType::Info(msg.into()));
    }

    pub fn add_warning<T: Into<String>>(&mut self, msg: T) {
        self.messages.push(MsgType::Warning(msg.into()));
    }

    pub fn add_error<T: Into<String>>(&mut self, msg: T) {
        self.messages.push(MsgType::Error(msg.into()));
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        const PANEL_WIDTH: f32 = 240.0;
        const OUT_MARGIN: f32 = 10.0;

        let rect = ctx.screen_rect();

        egui::Area::new("Messages")
            .fixed_pos(egui::pos2(
                0.0_f32.max(rect.width() - PANEL_WIDTH - 30.0),
                rect.height() - self.y_offset,
            ))
            .show(ctx, |ui| {
                let mut height = 0.0;
                self.messages.retain(|msg| {
                    let response = egui::Frame::none()
                        .fill(ui.style().visuals.panel_fill)
                        .inner_margin(egui::Margin::symmetric(12.0, 12.0))
                        .outer_margin(egui::Margin {
                            bottom: OUT_MARGIN,
                            ..Default::default()
                        })
                        .shadow(ui.style().visuals.window_shadow)
                        .show(ui, |ui| {
                            ui.set_min_width(PANEL_WIDTH);
                            match msg {
                                MsgType::Info(msg) => ui.label(msg),
                                MsgType::Warning(msg) => {
                                    ui.colored_label(ui.style().visuals.warn_fg_color, msg)
                                }
                                MsgType::Error(msg) => {
                                    ui.colored_label(ui.style().visuals.error_fg_color, msg)
                                }
                            }
                        })
                        .response;

                    height += response.rect.height() + 3.0;
                    !response.hovered()
                });

                if self.y_offset < height {
                    self.y_offset += 4.0;
                    ctx.request_repaint();
                } else {
                    self.y_offset = height;
                }
            });
    }
}
