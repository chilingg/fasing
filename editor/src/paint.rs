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
    id_source: String,
    ui: &mut egui::Ui,
    content: impl FnOnce(&mut egui::Ui, Option<std::ops::Range<usize>>) -> usize,
) {
    let id = ui.make_persistent_id(id_source.clone() + "Start_End");
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

    let column = (scroll.content_size.x / STRUC_UI_SIZE.x).floor();
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
