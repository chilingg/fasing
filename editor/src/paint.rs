use fasing::struc::space::KeyPointType;
use once_cell::sync::Lazy;

const INNER_MARGIN: f32 = 12.0;
const INNER_MARGIN_B: f32 = 32.0;
const UI_SIZE: egui::Vec2 = egui::vec2(
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

pub fn struct_painter(
    name: &String,
    ui: &mut egui::Ui,
    selected: bool,
    add_contents: impl FnOnce(egui::Rect, egui::Painter, egui::Response),
) {
    let (response, painter) = ui.allocate_painter(UI_SIZE, egui::Sense::click());
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
                ui.style().visuals.extreme_bg_color,
            );
        }
        painter.text(
            egui::pos2(
                rect.min.x + UI_SIZE.x * 0.5,
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

pub fn pos_mark(
    pos: egui::Pos2,
    point_type: KeyPointType,
    width: f32,
    mark_stroke: egui::Stroke,
) -> egui::Shape {
    match point_type {
        KeyPointType::Line => egui::Shape::rect_stroke(
            egui::Rect::from_center_size(pos, egui::Vec2::splat(width)),
            egui::Rounding::none(),
            mark_stroke,
        ),
        KeyPointType::Mark => {
            let half = width * 0.5;
            egui::Shape::Vec(vec![
                egui::Shape::line_segment(
                    [pos + egui::vec2(-half, -half), pos + egui::vec2(half, half)],
                    mark_stroke,
                ),
                egui::Shape::line_segment(
                    [pos + egui::vec2(half, -half), pos + egui::vec2(-half, half)],
                    mark_stroke,
                ),
            ])
        }
        KeyPointType::Hide => egui::Shape::Noop,
    }
}
