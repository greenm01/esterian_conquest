use crate::branding::NOSTRIAN_CONQUEST_LOGO;
use crate::buffer::{CellStyle, GameColor, PlayfieldBuffer};
use crate::geometry::ScreenGeometry;
use crate::modal::{Rect, centered_rect, draw_box_without_close_button, wrap_modal_text_lines};
use crate::native_grid::{logical_cell_metrics, logical_text_metrics};
use crate::theme;
use crate::ui::scene::{SceneGraph, ScenePoint, SceneRect};
use crate::ui::UiScene;

use super::state::{FirstRunField, LobbyRoute, LobbyState};

pub fn initial_route(keychain_exists: bool) -> LobbyRoute {
    if keychain_exists {
        LobbyRoute::Locked
    } else {
        LobbyRoute::FirstRun
    }
}

const GATE_MIN_POPUP_WIDTH: u16 = 54;
const GATE_MIN_POPUP_HEIGHT: u16 = 18;
const GATE_SIDE_PADDING: usize = 3;
const GATE_FIELD_LABEL_WIDTH: usize = 14;
const MATRIX_MIN_STREAM_LENGTH: usize = 3;
const MATRIX_GLYPHS: &[char] = &[
    'Α', 'Β', 'Γ', 'Δ', 'Ε', 'Ζ', 'Η', 'Θ', 'Ι', 'Κ', 'Λ', 'Μ', 'Ν', 'Ξ', 'Ο', 'Π', 'Ρ', 'Σ', 'Τ',
    'Υ', 'Φ', 'Χ', 'Ψ', 'Ω', '+', '#', '%', '*',
];

#[derive(Clone)]
pub struct MatrixRain {
    width: usize,
    height: usize,
    tick: u64,
    rng: u64,
    columns: Vec<MatrixColumn>,
}

#[derive(Clone)]
struct MatrixColumn {
    gap_remaining: usize,
    length: usize,
    update_every: usize,
    phase: usize,
    head_row: isize,
    tail_row: isize,
    glyphs: Vec<char>,
}

impl MatrixRain {
    pub fn new(width: usize, height: usize) -> Self {
        let mut rain = Self {
            width,
            height,
            tick: 0,
            rng: seed_for_size(width, height),
            columns: Vec::new(),
        };
        rain.reset_for_size(width, height);
        rain
    }

    pub fn reset_for_size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.tick = 0;
        self.rng = seed_for_size(width, height);
        self.columns = (0..width)
            .map(|column| self.make_column(column))
            .collect::<Vec<_>>();
        let warmup_steps = (height / 3).max(1);
        for _ in 0..warmup_steps {
            self.advance();
        }
    }

    pub fn reset(&mut self) {
        self.reset_for_size(self.width, self.height);
    }

    pub fn advance(&mut self) {
        self.tick = self.tick.saturating_add(1);
        for column_index in 0..self.columns.len() {
            if column_index % 2 == 1 {
                continue;
            }
            let update_every = self.columns[column_index].update_every;
            let phase = self.columns[column_index].phase;
            if ((self.tick as usize) + phase) % update_every != 0 {
                continue;
            }
            self.advance_column(column_index);
        }
    }

    fn advance_column(&mut self, column_index: usize) {
        if self.height == 0 {
            return;
        }
        let height = self.height as isize;
        if self.columns[column_index].gap_remaining > 0 {
            self.columns[column_index].gap_remaining -= 1;
            return;
        }
        if self.columns[column_index].head_row < 0 {
            let glyph = self.random_glyph();
            let column = &mut self.columns[column_index];
            column.head_row = 0;
            column.tail_row = 0;
            column.glyphs[0] = glyph;
            return;
        }

        {
            let column = &mut self.columns[column_index];
            column.head_row += 1;
        }
        let head_row = self.columns[column_index].head_row;
        if head_row < height {
            let glyph = self.random_glyph();
            self.columns[column_index].glyphs[head_row as usize] = glyph;
        }

        {
            let column = &mut self.columns[column_index];
            if column.head_row - column.tail_row + 1 > column.length as isize {
                column.tail_row += 1;
            }
        }

        let head = self.columns[column_index].head_row.min(height - 1);
        let tail = self.columns[column_index].tail_row.max(0);
        for row in tail..head {
            if self.next_random(8) == 0 {
                let glyph = self.random_glyph();
                self.columns[column_index].glyphs[row as usize] = glyph;
            }
        }

        if self.columns[column_index].tail_row >= height {
            let next = self.make_column(column_index);
            self.columns[column_index] = next;
        }
    }

    pub fn render(&self, buffer: &mut PlayfieldBuffer) {
        let background = theme::body_style().bg;
        let trail_style = CellStyle::new(GameColor::Green, background, false);
        let head_style = CellStyle::new(GameColor::BrightGreen, background, true);

        for (x, column) in self.columns.iter().enumerate() {
            if x >= buffer.width() || column.head_row < 0 {
                continue;
            }
            let visible_top = column.tail_row.max(0) as usize;
            let visible_bottom = column
                .head_row
                .min((self.height.saturating_sub(1)) as isize);
            for y in visible_top..=visible_bottom as usize {
                if y >= buffer.height() {
                    break;
                }
                let style = if y as isize == visible_bottom {
                    head_style
                } else {
                    trail_style
                };
                buffer.set_cell(y, x, column.glyphs[y], style);
            }
        }
    }

    fn make_column(&mut self, column_index: usize) -> MatrixColumn {
        let height = self.height.max(1);
        let length_max = height.saturating_sub(3).max(MATRIX_MIN_STREAM_LENGTH);
        let length =
            MATRIX_MIN_STREAM_LENGTH + self.next_random(length_max - MATRIX_MIN_STREAM_LENGTH + 1);
        MatrixColumn {
            gap_remaining: 1 + self.next_random(height),
            length,
            update_every: 1 + self.next_random(3),
            phase: (column_index * 3 + self.next_random(7)) % 3,
            head_row: -1,
            tail_row: 0,
            glyphs: vec![' '; height],
        }
    }

    fn random_glyph(&mut self) -> char {
        MATRIX_GLYPHS[self.next_random(MATRIX_GLYPHS.len())]
    }

    fn next_random(&mut self, limit: usize) -> usize {
        if limit <= 1 {
            return 0;
        }
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.rng >> 32) as usize) % limit
    }
}

fn seed_for_size(width: usize, height: usize) -> u64 {
    ((width as u64) << 32) ^ (height as u64) ^ 0x9E37_79B9_7F4A_7C15
}

pub fn render_first_run(buffer: &mut PlayfieldBuffer, state: &LobbyState) {
    let copy_lines = vec![
        "Create your local hosted identity.".to_string(),
        "Choose a handle, set a password, and confirm it.".to_string(),
    ];
    let fields = vec![
        GateField {
            label: "Handle",
            value: display_or_cursor(&state.first_run_handle_input),
            active: state.first_run_field == FirstRunField::Handle,
            cursor_offset: state.first_run_handle_input.chars().count(),
        },
        GateField {
            label: "Set Password",
            value: masked_or_cursor(&state.first_run_password_input),
            active: state.first_run_field == FirstRunField::Password,
            cursor_offset: state.first_run_password_input.chars().count(),
        },
        GateField {
            label: "Confirm",
            value: masked_or_cursor(&state.first_run_confirm_input),
            active: state.first_run_field == FirstRunField::Confirm,
            cursor_offset: state.first_run_confirm_input.chars().count(),
        },
    ];
    render_gate(
        buffer,
        "FIRST RUN",
        state.status_message.as_deref(),
        &copy_lines,
        &fields,
    );
}

pub fn render_locked(buffer: &mut PlayfieldBuffer, state: &LobbyState) {
    let copy_lines = vec!["Enter your keychain password.".to_string()];
    let fields = vec![GateField {
        label: "Password",
        value: masked_or_cursor(&state.unlock_password_input),
        active: true,
        cursor_offset: state.unlock_password_input.chars().count(),
    }];
    render_gate(
        buffer,
        "UNLOCK KEYCHAIN",
        state.status_message.as_deref(),
        &copy_lines,
        &fields,
    );
}

pub fn render_first_run_scene(geometry: ScreenGeometry, state: &LobbyState) -> UiScene {
    let copy_lines = vec![
        "Create your local hosted identity.".to_string(),
        "Choose a handle, set a password, and confirm it.".to_string(),
    ];
    let fields = vec![
        GateField {
            label: "Handle",
            value: display_or_cursor(&state.first_run_handle_input),
            active: state.first_run_field == FirstRunField::Handle,
            cursor_offset: state.first_run_handle_input.chars().count(),
        },
        GateField {
            label: "Set Password",
            value: masked_or_cursor(&state.first_run_password_input),
            active: state.first_run_field == FirstRunField::Password,
            cursor_offset: state.first_run_password_input.chars().count(),
        },
        GateField {
            label: "Confirm",
            value: masked_or_cursor(&state.first_run_confirm_input),
            active: state.first_run_field == FirstRunField::Confirm,
            cursor_offset: state.first_run_confirm_input.chars().count(),
        },
    ];
    build_gate_scene(
        geometry,
        "FIRST RUN",
        state.status_message.as_deref(),
        &copy_lines,
        &fields,
    )
}

pub fn render_locked_scene(geometry: ScreenGeometry, state: &LobbyState) -> UiScene {
    let copy_lines = vec!["Enter your keychain password.".to_string()];
    let fields = vec![GateField {
        label: "Password",
        value: masked_or_cursor(&state.unlock_password_input),
        active: true,
        cursor_offset: state.unlock_password_input.chars().count(),
    }];
    build_gate_scene(
        geometry,
        "UNLOCK KEYCHAIN",
        state.status_message.as_deref(),
        &copy_lines,
        &fields,
    )
}

pub fn render_matrix_locked(buffer: &mut PlayfieldBuffer, rain: &MatrixRain) {
    rain.render(buffer);
}

#[doc(hidden)]
pub fn matrix_glyph(x: usize, y: usize, frame: u64) -> char {
    let index = ((frame as usize) + (x * 13) + (y * 7)) % MATRIX_GLYPHS.len();
    MATRIX_GLYPHS[index]
}

struct GateField<'a> {
    label: &'a str,
    value: String,
    active: bool,
    cursor_offset: usize,
}

fn render_gate(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    status_message: Option<&str>,
    copy_lines: &[String],
    fields: &[GateField<'_>],
) {
    let width = buffer.width() as u16;
    let height = buffer.height() as u16;
    if width < 60 || height < 24 {
        render_tiny(buffer, title);
        return;
    }

    let logo_width = NOSTRIAN_CONQUEST_LOGO
        .iter()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    let field_width = fields.iter().map(field_row_width).max().unwrap_or(0);
    let copy_width = copy_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let status_width = status_message
        .map(|message| message.chars().count())
        .unwrap_or(0);
    let natural_content_width = logo_width
        .max(field_width)
        .max(copy_width)
        .max(status_width);
    let popup_width = (natural_content_width + GATE_SIDE_PADDING * 2 + 2)
        .max(usize::from(GATE_MIN_POPUP_WIDTH))
        .min(width.saturating_sub(2) as usize) as u16;
    let content_width = popup_content_width(popup_width);
    let wrapped_status = status_message
        .map(|message| wrap_modal_text_lines(&[message.to_string()], content_width))
        .unwrap_or_default();
    let wrapped_copy = wrap_modal_text_lines(copy_lines, content_width);
    let fixed_rows = NOSTRIAN_CONQUEST_LOGO.len() + 1 + wrapped_copy.len() + 1 + fields.len();
    let popup_height = (fixed_rows + wrapped_status.len() + 2)
        .min(height.saturating_sub(2) as usize)
        .max(usize::from(GATE_MIN_POPUP_HEIGHT)) as u16;
    let popup = centered_rect(
        popup_width,
        popup_height,
        Rect::new(1, 1, width.saturating_sub(2), height.saturating_sub(2)),
    );

    draw_box_without_close_button(
        buffer,
        popup,
        title,
        theme::table_chrome_style(),
        theme::table_header_style(),
    );
    buffer.fill_rect(
        popup.y as usize + 1,
        popup.x as usize + 1,
        popup.width.saturating_sub(2) as usize,
        popup.height.saturating_sub(2) as usize,
        theme::table_body_style(),
    );

    let left = popup.x as usize + GATE_SIDE_PADDING;
    let content_width = popup_content_width(popup.width);
    let content_bottom = popup.y as usize + popup.height.saturating_sub(2) as usize;
    let mut row = popup.y as usize + 1;
    row += write_wrapped_rows(
        buffer,
        row,
        left,
        content_width,
        content_bottom.saturating_add(1).saturating_sub(row),
        &wrapped_status,
        theme::error_style(),
    );
    if !wrapped_status.is_empty() {
        row += 1;
    }

    row += draw_logo(buffer, row, popup);
    row += 1;

    row += write_wrapped_rows(
        buffer,
        row,
        left,
        content_width,
        content_bottom.saturating_add(1).saturating_sub(row),
        &wrapped_copy,
        theme::prompt_notice_action_style(),
    );
    row += 1;

    for field in fields {
        if row > content_bottom {
            break;
        }
        write_field(buffer, row, left, content_width, field);
        row += 1;
    }
}

fn build_gate_scene(
    geometry: ScreenGeometry,
    title: &str,
    status_message: Option<&str>,
    copy_lines: &[String],
    fields: &[GateField<'_>],
) -> UiScene {
    let metrics = logical_cell_metrics();
    let scene_width = geometry.width() as f32 * metrics.cell_width_px as f32;
    let scene_height = geometry.height() as f32 * metrics.cell_height_px as f32;
    let mut scene = SceneGraph::new(scene_width, scene_height);
    scene.push_quad(
        SceneRect::new(0.0, 0.0, scene_width, scene_height),
        theme::body_style().bg,
    );

    let width = geometry.width() as u16;
    let height = geometry.height() as u16;
    if width < 60 || height < 24 {
        render_tiny_scene(&mut scene, title, metrics.cell_width_px as f32, metrics.cell_height_px as f32);
        return UiScene::Graph(scene);
    }

    let logo_width = NOSTRIAN_CONQUEST_LOGO
        .iter()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    let field_width = fields.iter().map(field_row_width).max().unwrap_or(0);
    let copy_width = copy_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let status_width = status_message
        .map(|message| message.chars().count())
        .unwrap_or(0);
    let natural_content_width = logo_width
        .max(field_width)
        .max(copy_width)
        .max(status_width);
    let popup_width = (natural_content_width + GATE_SIDE_PADDING * 2 + 2)
        .max(usize::from(GATE_MIN_POPUP_WIDTH))
        .min(width.saturating_sub(2) as usize) as u16;
    let content_width = popup_content_width(popup_width);
    let wrapped_status = status_message
        .map(|message| wrap_modal_text_lines(&[message.to_string()], content_width))
        .unwrap_or_default();
    let wrapped_copy = wrap_modal_text_lines(copy_lines, content_width);
    let fixed_rows = NOSTRIAN_CONQUEST_LOGO.len() + 1 + wrapped_copy.len() + 1 + fields.len();
    let popup_height = (fixed_rows + wrapped_status.len() + 2)
        .min(height.saturating_sub(2) as usize)
        .max(usize::from(GATE_MIN_POPUP_HEIGHT)) as u16;
    let popup = centered_rect(
        popup_width,
        popup_height,
        Rect::new(1, 1, width.saturating_sub(2), height.saturating_sub(2)),
    );

    let popup_rect = scene_rect_from_cells(popup);
    let interior_top = popup_rect.y + metrics.cell_height_px as f32;
    let interior_height =
        (popup_rect.height - metrics.cell_height_px as f32 - 2.0).max(0.0);
    scene.push_quad(
        SceneRect::new(
            popup_rect.x + 1.0,
            interior_top,
            (popup_rect.width - 2.0).max(0.0),
            interior_height,
        ),
        theme::table_body_style().bg,
    );
    draw_gate_frame_scene(&mut scene, popup, title);

    let left = popup.x as usize + GATE_SIDE_PADDING;
    let content_bottom = popup.y as usize + popup.height.saturating_sub(2) as usize;
    let mut row = popup.y as usize + 1;
    row += write_wrapped_scene_rows(
        &mut scene,
        row,
        left,
        content_width,
        content_bottom.saturating_add(1).saturating_sub(row),
        &wrapped_status,
        theme::error_style(),
    );
    if !wrapped_status.is_empty() {
        row += 1;
    }

    row += draw_logo_scene(&mut scene, row, popup);
    row += 1;

    row += write_wrapped_scene_rows(
        &mut scene,
        row,
        left,
        content_width,
        content_bottom.saturating_add(1).saturating_sub(row),
        &wrapped_copy,
        theme::prompt_notice_action_style(),
    );
    row += 1;

    for field in fields {
        if row > content_bottom {
            break;
        }
        write_scene_field(&mut scene, row, left, content_width, field);
        row += 1;
    }

    UiScene::Graph(scene)
}

fn render_tiny_scene(
    scene: &mut SceneGraph,
    title: &str,
    cell_width: f32,
    cell_height: f32,
) {
    let lines = ["Nostrian Conquest", title, "Resize the window to continue."];
    let rows = (scene.logical_size().height / cell_height).floor().max(1.0) as usize;
    let cols = (scene.logical_size().width / cell_width).floor().max(1.0) as usize;
    let start_row = rows.saturating_sub(lines.len()) / 2;
    for (idx, line) in lines.iter().enumerate() {
        let row = start_row + idx;
        let col = cols.saturating_sub(line.chars().count()) / 2;
        let style = if idx == 0 {
            theme::logo_style()
        } else {
            theme::table_body_style()
        };
        scene.push_text(
            scene_point_from_cell(col, row),
            *line,
            style,
            None,
        );
    }
}

fn draw_gate_frame_scene(scene: &mut SceneGraph, popup: Rect, title: &str) {
    let metrics = logical_cell_metrics();
    let rect = scene_rect_from_cells(popup);
    let color = theme::table_chrome_style().fg;
    let title_text = format!("- {title} -");
    let title_width = title_text.chars().count() as f32 * metrics.cell_width_px as f32;
    let title_x = rect.x + ((rect.width - title_width).max(0.0) / 2.0);
    let right = (rect.right() - 1.0).max(rect.x);
    let bottom = (rect.bottom() - 1.0).max(rect.y);
    let horizontal_left = rect.x + 1.0;
    let horizontal_right = (right - 1.0).max(horizontal_left);
    let gap_left = (title_x - metrics.cell_width_px as f32 * 0.5)
        .max(horizontal_left)
        .min(horizontal_right);
    let gap_right = (title_x + title_width + metrics.cell_width_px as f32 * 0.5)
        .max(horizontal_left)
        .min(horizontal_right);

    if gap_left > horizontal_left {
        scene.push_line(
            ScenePoint::new(horizontal_left, rect.y),
            ScenePoint::new(gap_left - 1.0, rect.y),
            1.0,
            color,
        );
    }
    if gap_right < horizontal_right {
        scene.push_line(
            ScenePoint::new(gap_right + 1.0, rect.y),
            ScenePoint::new(horizontal_right, rect.y),
            1.0,
            color,
        );
    }
    scene.push_line(
        ScenePoint::new(rect.x, bottom),
        ScenePoint::new(right, bottom),
        1.0,
        color,
    );
    scene.push_line(
        ScenePoint::new(rect.x, rect.y),
        ScenePoint::new(rect.x, bottom),
        1.0,
        color,
    );
    scene.push_line(
        ScenePoint::new(right, rect.y),
        ScenePoint::new(right, bottom),
        1.0,
        color,
    );
    scene.push_text(
        ScenePoint::new(title_x, rect.y),
        title_text,
        theme::table_header_style(),
        Some(SceneRect::new(
            rect.x + 1.0,
            rect.y,
            rect.width - 2.0,
            metrics.cell_height_px as f32,
        )),
    );
}

fn scene_rect_from_cells(rect: Rect) -> SceneRect {
    let metrics = logical_cell_metrics();
    SceneRect::new(
        rect.x as f32 * metrics.cell_width_px as f32,
        rect.y as f32 * metrics.cell_height_px as f32,
        rect.width as f32 * metrics.cell_width_px as f32,
        rect.height as f32 * metrics.cell_height_px as f32,
    )
}

fn scene_point_from_cell(col: usize, row: usize) -> ScenePoint {
    let metrics = logical_cell_metrics();
    ScenePoint::new(
        col as f32 * metrics.cell_width_px as f32,
        row as f32 * metrics.cell_height_px as f32,
    )
}

fn write_scene_field(
    scene: &mut SceneGraph,
    row: usize,
    left: usize,
    content_width: usize,
    field: &GateField<'_>,
) {
    let marker = if field.active { ">" } else { " " };
    let marker_style = if field.active {
        theme::prompt_hotkey_style()
    } else {
        theme::status_label_style()
    };
    let value_style = if field.active {
        theme::prompt_style()
    } else {
        theme::table_body_style()
    };
    let label = format!(
        "{marker} {:<width$} :",
        field.label,
        width = GATE_FIELD_LABEL_WIDTH
    );
    let value_col = left + label.chars().count() + 1;
    let value_width = content_width.saturating_sub(value_col.saturating_sub(left));
    let value = clip_to_width(&field.value, value_width);
    let cell_metrics = logical_cell_metrics();
    let text_metrics = logical_text_metrics();
    let clip = Some(SceneRect::new(
        left as f32 * cell_metrics.cell_width_px as f32,
        row as f32 * cell_metrics.cell_height_px as f32,
        content_width as f32 * cell_metrics.cell_width_px as f32,
        cell_metrics.cell_height_px as f32,
    ));
    scene.push_text(scene_point_from_cell(left, row), label, marker_style, clip);
    scene.push_text(scene_point_from_cell(value_col, row), value.clone(), value_style, clip);
    if field.active {
        let caret_col = value_col + field.cursor_offset.min(value.chars().count());
        scene.push_caret(
            SceneRect::new(
                caret_col as f32 * cell_metrics.cell_width_px as f32 + text_metrics.left_inset_px,
                row as f32 * cell_metrics.cell_height_px as f32 + text_metrics.top_inset_px,
                2.0,
                (text_metrics.line_height_px - text_metrics.top_inset_px * 2.0).max(2.0),
            ),
            theme::prompt_hotkey_style().fg,
        );
    }
}

fn draw_logo_scene(scene: &mut SceneGraph, start_row: usize, popup: Rect) -> usize {
    let logo_width = NOSTRIAN_CONQUEST_LOGO
        .iter()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    let inner_left = popup.x as usize + 1;
    let inner_width = popup.width.saturating_sub(2) as usize;
    let logo_left = inner_left + inner_width.saturating_sub(logo_width) / 2;

    for (row_offset, line) in NOSTRIAN_CONQUEST_LOGO.iter().enumerate() {
        let mut run_start = None;
        let mut run_style = None;
        let mut run_text = String::new();
        let flush_run = |scene: &mut SceneGraph,
                         run_start: &mut Option<usize>,
                         run_style: &mut Option<crate::buffer::CellStyle>,
                         run_text: &mut String| {
            let Some(start) = run_start.take() else {
                return;
            };
            let style = run_style.take().expect("style exists for logo run");
            if run_text.is_empty() {
                return;
            }
            scene.push_text(
                scene_point_from_cell(start, start_row + row_offset),
                std::mem::take(run_text),
                style,
                Some(scene_rect_from_cells(popup)),
            );
        };

        for (col_offset, ch) in line.chars().enumerate() {
            let style = if is_star_decoration(ch) {
                theme::classic::star_decoration_style(row_offset + col_offset)
            } else {
                theme::logo_style()
            };
            if ch == ' ' {
                flush_run(scene, &mut run_start, &mut run_style, &mut run_text);
                continue;
            }
            let absolute_col = logo_left + col_offset;
            if run_style == Some(style) {
                run_text.push(ch);
                continue;
            }
            flush_run(scene, &mut run_start, &mut run_style, &mut run_text);
            run_start = Some(absolute_col);
            run_style = Some(style);
            run_text.push(ch);
        }
        flush_run(scene, &mut run_start, &mut run_style, &mut run_text);
    }

    NOSTRIAN_CONQUEST_LOGO.len()
}

fn write_wrapped_scene_rows(
    scene: &mut SceneGraph,
    start_row: usize,
    left: usize,
    content_width: usize,
    max_rows: usize,
    lines: &[String],
    style: crate::buffer::CellStyle,
) -> usize {
    if content_width == 0 || max_rows == 0 {
        return 0;
    }

    let visible_rows = lines.len().min(max_rows);
    for idx in 0..visible_rows {
        let is_last_visible = idx + 1 == max_rows;
        let overflow_hidden = lines.len() > max_rows;
        let line = if is_last_visible && overflow_hidden {
            truncate_with_continuation(&lines[idx], content_width)
        } else {
            clip_to_width(&lines[idx], content_width)
        };
        scene.push_text(
            scene_point_from_cell(left, start_row + idx),
            line,
            style,
            None,
        );
    }
    visible_rows
}

fn write_field(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    left: usize,
    content_width: usize,
    field: &GateField<'_>,
) {
    let marker = if field.active { ">" } else { " " };
    let marker_style = if field.active {
        theme::prompt_hotkey_style()
    } else {
        theme::status_label_style()
    };
    let value_style = if field.active {
        theme::prompt_style()
    } else {
        theme::table_body_style()
    };
    let label = format!(
        "{marker} {:<width$} :",
        field.label,
        width = GATE_FIELD_LABEL_WIDTH
    );
    let value_col = left + label.chars().count() + 1;
    buffer.write_text_clipped(row, left, &label, marker_style);
    let value_width = content_width.saturating_sub(value_col.saturating_sub(left));
    let value = clip_to_width(&field.value, value_width);
    buffer.write_text_clipped(row, value_col, &value, value_style);
    if field.active {
        let cursor_col = value_col + field.cursor_offset.min(value.chars().count());
        if cursor_col < buffer.width() {
            buffer.set_cursor(cursor_col as u16, row as u16);
        }
    }
}

fn draw_logo(buffer: &mut PlayfieldBuffer, start_row: usize, popup: Rect) -> usize {
    let logo_width = NOSTRIAN_CONQUEST_LOGO
        .iter()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    let inner_left = popup.x as usize + 1;
    let inner_width = popup.width.saturating_sub(2) as usize;
    let logo_left = inner_left + inner_width.saturating_sub(logo_width) / 2;

    for (row_offset, line) in NOSTRIAN_CONQUEST_LOGO.iter().enumerate() {
        for (col_offset, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let style = if is_star_decoration(ch) {
                theme::classic::star_decoration_style(row_offset + col_offset)
            } else {
                theme::logo_style()
            };
            buffer.write_text(
                start_row + row_offset,
                logo_left + col_offset,
                &ch.to_string(),
                style,
            );
        }
    }

    NOSTRIAN_CONQUEST_LOGO.len()
}

fn render_tiny(buffer: &mut PlayfieldBuffer, title: &str) {
    let lines = ["Nostrian Conquest", title, "Resize the window to continue."];
    let start_row = buffer.height().saturating_sub(lines.len()) / 2;
    for (idx, line) in lines.iter().enumerate() {
        let row = start_row + idx;
        let col = buffer.width().saturating_sub(line.chars().count()) / 2;
        let style = if idx == 0 {
            theme::logo_style()
        } else {
            theme::table_body_style()
        };
        buffer.write_text_clipped(row, col, line, style);
    }
}

fn field_row_width(field: &GateField<'_>) -> usize {
    2 + GATE_FIELD_LABEL_WIDTH + 2 + field.value.chars().count()
}

fn is_star_decoration(ch: char) -> bool {
    matches!(ch, '.' | '*' | 'o')
}

fn masked_or_cursor(value: &str) -> String {
    if value.is_empty() {
        "_".to_string()
    } else {
        "*".repeat(value.chars().count())
    }
}

fn display_or_cursor(value: &str) -> String {
    if value.is_empty() {
        "_".to_string()
    } else {
        value.to_string()
    }
}

fn popup_content_width(popup_width: u16) -> usize {
    popup_width.saturating_sub((GATE_SIDE_PADDING * 2 + 2) as u16) as usize
}

fn write_wrapped_rows(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    left: usize,
    content_width: usize,
    max_rows: usize,
    lines: &[String],
    style: crate::buffer::CellStyle,
) -> usize {
    if content_width == 0 || max_rows == 0 {
        return 0;
    }

    let visible_rows = lines.len().min(max_rows);
    for idx in 0..visible_rows {
        let is_last_visible = idx + 1 == max_rows;
        let overflow_hidden = lines.len() > max_rows;
        let line = if is_last_visible && overflow_hidden {
            truncate_with_continuation(&lines[idx], content_width)
        } else {
            clip_to_width(&lines[idx], content_width)
        };
        buffer.write_text_clipped(start_row + idx, left, &line, style);
    }
    visible_rows
}

fn clip_to_width(text: &str, max_width: usize) -> String {
    text.chars().take(max_width).collect()
}

fn truncate_with_continuation(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let clipped = clip_to_width(text, max_width.saturating_sub(3));
    format!("{clipped}...")
}

#[cfg(test)]
mod tests {
    use super::{render_first_run_scene, render_locked_scene};
    use crate::geometry::ScreenGeometry;
    use crate::lobby::state::LobbyRoute;
    use crate::lobby::LobbyApp;
    use crate::ui::scene::SceneNode;
    use crate::ui::UiScene;

    #[test]
    fn locked_scene_uses_explicit_title_and_caret_nodes() {
        let mut app = LobbyApp::new_for_tests(LobbyRoute::Locked, ScreenGeometry::new(120, 40));
        app.state.unlock_password_input = "hunter2".to_string();

        let scene = render_locked_scene(app.geometry, &app.state);
        let UiScene::Graph(graph) = scene else {
            panic!("locked scene should render as scene graph");
        };

        assert!(graph.nodes().iter().any(|node| matches!(
            node,
            SceneNode::Text(text) if text.text == "- UNLOCK KEYCHAIN -"
        )));
        assert!(graph
            .nodes()
            .iter()
            .any(|node| matches!(node, SceneNode::Caret(_))));
    }

    #[test]
    fn locked_scene_caret_stays_on_password_row() {
        let mut app = LobbyApp::new_for_tests(LobbyRoute::Locked, ScreenGeometry::new(120, 40));
        app.state.unlock_password_input = "hunter2".to_string();

        let scene = render_locked_scene(app.geometry, &app.state);
        let UiScene::Graph(graph) = scene else {
            panic!("locked scene should render as scene graph");
        };

        let caret = graph
            .nodes()
            .iter()
            .find_map(|node| match node {
                SceneNode::Caret(caret) => Some(caret),
                _ => None,
            })
            .expect("locked scene should include caret");
        let password_row = graph
            .nodes()
            .iter()
            .find_map(|node| match node {
                SceneNode::Text(text) if text.text.contains("Password") => Some(text.origin.y),
                _ => None,
            })
            .expect("locked scene should include password row");

        assert!(caret.rect.y >= password_row);
        assert!(caret.rect.y < password_row + crate::native_grid::logical_cell_metrics().cell_height_px as f32);
    }

    #[test]
    fn first_run_scene_uses_logical_scene_size_from_geometry() {
        let app = LobbyApp::new_for_tests(LobbyRoute::FirstRun, ScreenGeometry::new(120, 40));

        let scene = render_first_run_scene(app.geometry, &app.state);
        let UiScene::Graph(graph) = scene else {
            panic!("first-run scene should render as scene graph");
        };

        assert!(graph.logical_size().width > 0.0);
        assert!(graph.logical_size().height > 0.0);
        assert!(graph.nodes().iter().any(|node| matches!(node, SceneNode::Quad(_))));
    }
}
