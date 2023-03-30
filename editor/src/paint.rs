use fasing::struc::space::KeyPointType;
use once_cell::sync::Lazy;

const INNER_MARGIN: f32 = 12.0;
const INNER_MARGIN_B: f32 = 32.0;
const STRUC_UI_SIZE: egui::Vec2 = egui::vec2(
    PAINT_SIZE + INNER_MARGIN * 2.0,
    PAINT_SIZE + INNER_MARGIN + INNER_MARGIN_B,
);

pub const PAINT_SIZE: f32 = 160.0;
pub const STRUCT_OUT_MARGIN: f32 = 0.15;

pub static MARK_STROK: Lazy<egui::Stroke> =
    Lazy::new(|| egui::Stroke::new(1.5, egui::Color32::DARK_RED));
pub static STRUC_STROK_NORMAL: Lazy<egui::Stroke> =
    Lazy::new(|| egui::Stroke::new(3.0, egui::Color32::WHITE));
pub static STRUC_STROK_SELECTED: Lazy<egui::Stroke> =
    Lazy::new(|| egui::Stroke::new(3.0, egui::Color32::BLACK));

pub fn struc_painter(
    name: &String,
    ui: &mut egui::Ui,
    selected: bool,
    add_contents: impl FnOnce(egui::Rect, egui::Painter, egui::Response),
) {
    let (response, painter) = ui.allocate_painter(STRUC_UI_SIZE, egui::Sense::click());
    let rect = response.rect;

    if ui.is_rect_visible(response.rect) {
        let struc_area = egui::Rect::from_min_size(
            rect.min + egui::Vec2::splat(INNER_MARGIN),
            egui::Vec2::splat(PAINT_SIZE),
        );
        if selected || response.hovered() {
            painter.rect_filled(rect, egui::Rounding::none(), ui.visuals().selection.bg_fill);
            painter.rect_filled(struc_area, egui::Rounding::none(), egui::Color32::WHITE);
        } else {
            painter.rect_filled(
                struc_area,
                egui::Rounding::none(),
                match ui.is_enabled() {
                    true => ui.style().visuals.extreme_bg_color,
                    false => ui.style().visuals.noninteractive().weak_bg_fill,
                },
            );
        }
        painter.text(
            egui::pos2(
                rect.min.x + STRUC_UI_SIZE.x * 0.5,
                rect.min.y + PAINT_SIZE + INNER_MARGIN + 4.0,
            ),
            egui::Align2::CENTER_TOP,
            name,
            ui.style()
                .text_styles
                .get(&egui::TextStyle::Body)
                .cloned()
                .unwrap_or(egui::FontId::new(12.0, egui::FontFamily::Proportional)),
            ui.style().interact(&response).text_color(),
        );
        add_contents(struc_area, painter.with_clip_rect(struc_area), response);
    }
}

pub fn struc_scroll_area(
    id_source: &str,
    ui: &mut egui::Ui,
    content: impl FnOnce(&mut egui::Ui, Option<std::ops::Range<usize>>) -> usize,
) {
    let id = ui.make_persistent_id(id_source.to_string() + "Start_End");
    let (visual_range, column, pre, next): (Option<std::ops::Range<usize>>, f32, f32, f32) =
        ui.data_mut(|d| d.get_persisted(id)).unwrap_or_default();

    let scroll = egui::ScrollArea::vertical()
        .id_source(id_source)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::splat(0.0);

            if pre > 0.0 {
                ui.allocate_space(egui::vec2(column, pre));
            }
            let count = ui.horizontal_wrapped(|ui| content(ui, visual_range)).inner;
            if next > 0.0 {
                ui.allocate_space(egui::vec2(column, next));
            }
            count
        });

    let column = (scroll.inner_rect.width() / STRUC_UI_SIZE.x).floor();
    let start = (scroll.state.offset.y / STRUC_UI_SIZE.y).floor() * column;
    let end = ((scroll.inner_rect.height() / STRUC_UI_SIZE.y).ceil() + 1.0) * column + start;

    let pre_height = (scroll.state.offset.y / STRUC_UI_SIZE.y).floor() * STRUC_UI_SIZE.y;
    let next_height = match column > 0.0 {
        false => 0.0,
        true => ((scroll.inner as f32 - end) / column).ceil() * STRUC_UI_SIZE.y,
    };

    ui.data_mut(|d| {
        d.insert_persisted(
            id,
            (
                Some(start as usize..end as usize),
                column * STRUC_UI_SIZE.x,
                pre_height,
                next_height,
            ),
        )
    });
}

pub fn pos_mark(
    pos: egui::Pos2,
    point_type: KeyPointType,
    width: f32,
    mark_stroke: egui::Stroke,
) -> egui::Shape {
    let half = width * 0.5;
    match point_type {
        KeyPointType::Line => egui::Shape::rect_stroke(
            egui::Rect::from_center_size(pos, egui::Vec2::splat(width)),
            egui::Rounding::none(),
            mark_stroke,
        ),
        KeyPointType::Mark => egui::Shape::Vec(vec![
            egui::Shape::line_segment(
                [pos + egui::vec2(-half, -half), pos + egui::vec2(half, half)],
                mark_stroke,
            ),
            egui::Shape::line_segment(
                [pos + egui::vec2(half, -half), pos + egui::vec2(-half, half)],
                mark_stroke,
            ),
        ]),
        KeyPointType::Hide => egui::Shape::Noop,
        KeyPointType::Horizontal => egui::Shape::line_segment(
            [pos - egui::Vec2::Y * half, pos + egui::Vec2::Y * half],
            mark_stroke,
        ),
        KeyPointType::Vertical => egui::Shape::line_segment(
            [pos - egui::Vec2::X * half, pos + egui::Vec2::X * half],
            mark_stroke,
        ),
    }
}

pub fn break_text_in_width(text: String, ui: &mut egui::Ui) -> egui::Response {
    let (response, painter) = ui.allocate_painter(
        egui::vec2(
            ui.available_width(),
            ui.text_style_height(&egui::TextStyle::Body),
        ),
        egui::Sense::click(),
    );

    let job = egui::text::LayoutJob {
        sections: vec![egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: 0..text.len(),
            format: egui::TextFormat::simple(
                ui.style()
                    .text_styles
                    .get(&egui::TextStyle::Body)
                    .cloned()
                    .unwrap_or(egui::FontId::proportional(12.0)),
                ui.style().interact(&response).text_color(),
            ),
        }],
        text,
        wrap: egui::epaint::text::TextWrapping {
            max_width: response.rect.width(),
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        },
        break_on_newline: true,
        ..Default::default()
    };
    painter.add(egui::epaint::TextShape {
        pos: response.rect.left_top(),
        galley: egui::WidgetText::from(job)
            .into_galley(
                ui,
                Some(true),
                response.rect.width(),
                egui::style::TextStyle::Body,
            )
            .galley,
        underline: egui::Stroke::NONE,
        override_text_color: None,
        angle: 0.0,
    });

    response
}

pub fn regex_edite_label(
    id_source: &str,
    regex: &mut regex::Regex,
    ui: &mut egui::Ui,
) -> egui::Response {
    let id = ui.make_persistent_id(id_source);
    let (mut editing, mut content): (bool, String) = ui.data_mut(|d| {
        d.get_persisted(id)
            .unwrap_or((false, regex.as_str().to_owned()))
    });

    let response = match editing {
        true => {
            let response = ui.text_edit_singleline(&mut content);
            if response.hovered() {
                response.request_focus();
            }
            if response.lost_focus() {
                if let Ok(reg) = regex::Regex::new(content.as_str()) {
                    *regex = reg;
                }
                editing = false;
            }

            ui.data_mut(|d| d.insert_persisted(id, (editing, content)));
            response
        }
        false => {
            let response = break_text_in_width(regex.to_string(), ui);
            if response.double_clicked_by(egui::PointerButton::Primary) {
                editing = true;
                ui.data_mut(|d| d.insert_persisted(id, (editing, regex.to_string())));
            }
            response
        }
    };

    response
}

pub fn orger_drag_drop(
    ui: &mut egui::Ui,
    source_id: egui::Id,
    num: usize,
    drag_target: &mut Option<usize>,
    drop_target: &mut Option<usize>,
) {
    let id = source_id.with(num);
    let response = ui.label((num + 1).to_string());
    let response = ui.interact(response.rect, id, egui::Sense::drag());

    if response.dragged_by(egui::PointerButton::Primary) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }

    if response.drag_released_by(egui::PointerButton::Primary) {
        *drag_target = Some(num);
    };

    if response.hovered {
        if ui.memory(|mem| mem.is_anything_being_dragged()) {
            *drop_target = Some(num);
        } else {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
    }
}
