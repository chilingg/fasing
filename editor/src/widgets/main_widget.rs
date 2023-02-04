use super::*;
use crate::gui::{
    prelude::*,
    theme,
};

use std::fs;
use anyhow::Result;

pub struct  MainWidget {
    children: Vec<(&'static str, Box<dyn Widget>)>,
}

impl MainWidget {
    pub fn new() -> Self {
        Self { 
            children: vec![
                ("sidbar", widget_box(Sidebar::default())),
                ("style editor", widget_box(theme::StyleEditor::new(false, "style.json".to_string(), theme::default_style()))),
                ("query", widget_box(QueryWindow::default())),
                ("center", widget_box(Center::default())),
            ]
        }
    }
}

pub fn get_fonts<P>(font_key: String, path: P) -> Result<egui::FontDefinitions>
where
    P: AsRef<std::path::Path>
{
    let font_data = fs::read(path)?;

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        font_key.clone(),
        egui::FontData::from_owned(font_data)
    );

    // Put my font first (highest priority):
    fonts.families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, font_key.clone());

    // Put my font as last fallback for monospace:
    fonts.families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(font_key);
    
    Ok(fonts)
}

impl Widget for MainWidget {
    fn start(&mut self, app_state: &mut AppState) {
        app_state.egui.ctx.set_style(theme::default_style());

        let font_path = "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc";
        app_state.egui.ctx.set_fonts(
            get_fonts("Fasing Font".to_string(), font_path)
                .expect(format!("Failed to set font `{font_path}`").as_str())
        );
    }

    fn children(&mut self) -> Children {
        self.children.iter_mut().map(|(_, child)| child).collect()
    }

    fn process(&mut self, window_event: &we::WindowEvent, _: &mut AppState) -> bool {
        use we::WindowEvent::*;

        match window_event {
            KeyboardInput {
                input: we::KeyboardInput {
                    virtual_keycode: Some(we::VirtualKeyCode::F12),
                    state: we::ElementState::Pressed,
                    ..
                },
                ..
            } => {
                let w_data = self.children
                    .iter_mut()
                    .find(|(name, _)| { *name == "style editor"})
                    .unwrap()
                    .1
                    .widget_data()
                    .unwrap();
                w_data.open = !w_data.open;

                true
            },
            KeyboardInput {
                input: we::KeyboardInput {
                    virtual_keycode: Some(we::VirtualKeyCode::F5),
                    state: we::ElementState::Pressed,
                    ..
                },
                ..
            } => {
                let w_data = self.children
                    .iter_mut()
                    .find(|(name, _)| { *name == "query"})
                    .unwrap()
                    .1
                    .widget_data()
                    .unwrap();
                w_data.open = !w_data.open;

                true
            },
            _ => false
        }
    }
}