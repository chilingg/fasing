use super::{
    gui::{
        self,
        prelude::*,
        theme::StyleEditor,
    },
    sidebar::Sidebar,
};

use std::fs;
use anyhow::Result;
use std::collections::HashMap;

pub struct  MainWidget {
    children: HashMap<String, Box<dyn Widget>>,
}

impl MainWidget {
    pub fn new() -> Self {
        let mut children: HashMap<String, Box<dyn Widget>> = HashMap::new();
        children.insert("sidbar".to_string(), Box::new(Sidebar::default()));
        children.insert("style_editor".to_string(), Box::new(StyleEditor::new(
            false,
            "style.json".to_string(),
            gui::theme::default_style(),)
        ));

        Self { children }
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

impl RootWidget for MainWidget {
    fn start(&mut self, app_state: &mut AppState) {
        app_state.egui.ctx.set_style(gui::theme::default_style());

        let font_path = "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc";
        app_state.egui.ctx.set_fonts(
            get_fonts("Fasing Font".to_string(), font_path)
                .expect("Failed to set font `{font_path}`")
        );
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
                let w_data = self.children.get_mut("style_editor").unwrap().widget_data().unwrap();
                w_data.open = !w_data.open;

                true
            }
            _ => false
        }
    }
}

impl Widget for MainWidget {
    fn children(&mut self) -> Children {
        self.children.values_mut().collect()
    }
}