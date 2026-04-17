use crate::app::state::{DashApp, OwnedPlanetPopupMode};
use crate::buffer::PlayfieldBuffer;
use crate::layout::{self, MapWidgetFrame, dashboard};
use crate::modal::Rect;
use crate::overlays::frame::{
    OverlayFrame, OverlaySizePolicy, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent,
};
use crate::planet_view::selected_planet_detail;
use crate::table::{TableFooter, with_command_line_toast};
use crate::theme;
use nc_data::ProductionItemKind;
use nc_engine::{
    ArmyTransportMode, PlanetBuildSpecifyEntry, build_unit_spec_by_kind,
    transport_fleet_candidates_for_planet,
};
const BUILD_TABLE_NUMBER_WIDTH: usize = 2;
const BUILD_TABLE_COST_WIDTH: usize = 4;
const BUILD_TABLE_QUEUE_WIDTH: usize = 5;
const BUILD_TABLE_STATUS_WIDTH: usize = 6;
const BUILD_TABLE_MIN_UNIT_WIDTH: usize = 4;
const BUILD_TABLE_SEPARATOR_WIDTH: usize = 1;

pub fn draw(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    map_frame: MapWidgetFrame,
    planet_record_index_1_based: usize,
) {
    let max_body_width = map_frame.outer.width.saturating_sub(6).max(1);
    let popup = popup_layout(app, max_body_width);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        &popup.title,
        popup.body_width,
        popup.body_height,
        OverlaySizePolicy::default(),
        popup.footer,
        app.popup_position_for(crate::app::state::ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        }),
    );
    draw_popup_body(buf, frame, popup.body);
}

pub fn popup_rect(
    app: &DashApp,
    map_frame: MapWidgetFrame,
    planet_record_index_1_based: usize,
) -> Rect {
    let max_body_width = map_frame.outer.width.saturating_sub(6).max(1);
    let popup = popup_layout(app, max_body_width);
    overlay_popup_rect_for_body_in_parent(
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        &popup.title,
        popup.body_width,
        popup.body_height,
        OverlaySizePolicy::default(),
        popup.footer,
        app.popup_position_for(crate::app::state::ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        }),
    )
}

struct PopupLayout<'a> {
    title: String,
    body_width: usize,
    body_height: usize,
    footer: TableFooter<'a>,
    body: PopupBody,
}

enum PopupBody {
    Plain(Vec<String>),
    BuildSpecify(BuildSpecifyPopupLayout),
}

#[derive(Debug, Clone)]
struct BuildSpecifyPopupLayout {
    budget_title: String,
    body_width: usize,
    table_height: usize,
    entries: Vec<PlanetBuildSpecifyEntry>,
}

fn popup_layout<'a>(app: &'a DashApp, max_body_width: usize) -> PopupLayout<'a> {
    match app.owned_planet_popup.mode {
        OwnedPlanetPopupMode::Browse => {
            let lines = browse_lines(app, max_body_width);
            plain_popup_layout(
                String::from("PLANET STATUS"),
                lines,
                command_line_toast_footer(
                    app,
                    TableFooter::CommandPrompt {
                        label: "COMMAND",
                        prompt: "? B D A C M L U X <ESC> ->",
                    },
                ),
            )
        }
        OwnedPlanetPopupMode::BuildList => plain_popup_layout(
            build_popup_title(app),
            wrap_plain_lines(&build_list_lines(app), max_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "<ESC> ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::BuildAbortConfirm => plain_popup_layout(
            build_popup_title(app),
            vec![String::from("Abort all queued builds for this planet?")],
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "Abort queued builds? Y/[N] ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::BuildSpecify => {
            let build = build_specify_popup_layout(app, max_body_width);
            PopupLayout {
                title: build_popup_title(app),
                body_width: build.body_width,
                body_height: build_specify_body_height(&build),
                footer: command_line_toast_footer(
                    app,
                    TableFooter::CommandInput {
                        label: "COMMAND",
                        prompt: "Unit ",
                        default: "0",
                        input: &app.owned_planet_popup.input,
                    },
                ),
                body: PopupBody::BuildSpecify(build),
            }
        }
        OwnedPlanetPopupMode::BuildQuantity => plain_popup_layout(
            build_popup_title(app),
            wrap_plain_lines(&build_quantity_lines(app), max_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: "Qty ",
                    default: "MAX",
                    input: &app.owned_planet_popup.input,
                },
            ),
        ),
        OwnedPlanetPopupMode::CommissionSelect => {
            let default = if app.owned_planet_popup.default.is_empty() {
                "01"
            } else {
                app.owned_planet_popup.default.as_str()
            };
            plain_popup_layout(
                String::from("PLANET STATUS"),
                wrap_plain_lines(&commission_lines(app), max_body_width),
                command_line_toast_footer(
                    app,
                    TableFooter::CommandInput {
                        label: "COMMAND",
                        prompt: "Commission slot #",
                        default,
                        input: &app.owned_planet_popup.input,
                    },
                ),
            )
        }
        OwnedPlanetPopupMode::CommissionResult => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&app.owned_planet_popup.report_lines, max_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "<ESC> ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::MassCommissionConfirm => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(
                &[String::from(
                    "Automatically commission all ships and starbases in stardock?",
                )],
                max_body_width,
            ),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "Mass commission? Y/[N] ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::MassCommissionReport => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&app.owned_planet_popup.report_lines, max_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "<ESC> ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::TransportFleetSelect { mode } => {
            let prompt = match mode {
                ArmyTransportMode::Load => "Load Fleet #",
                ArmyTransportMode::Unload => "Unload Fleet #",
            };
            let default = if app.owned_planet_popup.default.is_empty() {
                "1"
            } else {
                app.owned_planet_popup.default.as_str()
            };
            plain_popup_layout(
                String::from("PLANET STATUS"),
                wrap_plain_lines(&transport_fleet_lines(app, mode), max_body_width),
                command_line_toast_footer(
                    app,
                    TableFooter::CommandInput {
                        label: "COMMAND",
                        prompt,
                        default,
                        input: &app.owned_planet_popup.input,
                    },
                ),
            )
        }
        OwnedPlanetPopupMode::TransportQuantity { mode } => {
            let prompt = match mode {
                ArmyTransportMode::Load => "How many armies to load?",
                ArmyTransportMode::Unload => "How many armies to unload?",
            };
            let default = if app.owned_planet_popup.default.is_empty() {
                "0"
            } else {
                app.owned_planet_popup.default.as_str()
            };
            plain_popup_layout(
                String::from("PLANET STATUS"),
                wrap_plain_lines(&transport_quantity_lines(app, mode), max_body_width),
                command_line_toast_footer(
                    app,
                    TableFooter::CommandInput {
                        label: "COMMAND",
                        prompt,
                        default,
                        input: &app.owned_planet_popup.input,
                    },
                ),
            )
        }
        OwnedPlanetPopupMode::ScorchConfirm1 => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&scorch_lines(app), max_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "Are you sure? Y/[N] ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::ScorchConfirm2 => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&scorch_lines(app), max_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "Are you really sure? Y/[N] ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::ScorchConfirm3 => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&scorch_lines(app), max_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "Are you sure-sure? Last chance to bail! Y/[N] ->",
                },
            ),
        ),
    }
}

fn command_line_toast_footer<'a>(app: &'a DashApp, footer: TableFooter<'a>) -> TableFooter<'a> {
    with_command_line_toast(footer, app.active_command_line_toast())
}

fn plain_popup_layout<'a>(
    title: String,
    lines: Vec<String>,
    footer: TableFooter<'a>,
) -> PopupLayout<'a> {
    PopupLayout {
        title,
        body_width: max_line_width(&lines).max(1),
        body_height: lines.len().max(1),
        footer,
        body: PopupBody::Plain(lines),
    }
}

fn draw_popup_body(buf: &mut PlayfieldBuffer, frame: OverlayFrame, body: PopupBody) {
    match body {
        PopupBody::Plain(lines) => {
            for (idx, line) in lines.into_iter().enumerate().take(frame.body_height) {
                layout::write_clipped(
                    buf,
                    frame.body_row + idx,
                    frame.body_col,
                    frame.body_width,
                    &line,
                    theme::value_style(),
                );
            }
        }
        PopupBody::BuildSpecify(build) => {
            render_build_specify_playfield(buf, frame, &build);
        }
    }
}

fn render_build_specify_playfield(
    buf: &mut PlayfieldBuffer,
    frame: OverlayFrame,
    layout: &BuildSpecifyPopupLayout,
) {
    if frame.body_width < 3 || frame.body_height < 4 {
        return;
    }

    let top = frame.body_row;
    let left = frame.body_col;
    let bottom = frame.body_row + frame.body_height.saturating_sub(1);
    let right = frame.body_col + frame.body_width.saturating_sub(1);
    let chrome = theme::table_chrome_style();

    for col in left..=right {
        buf.set_cell(top, col, '─', chrome);
        buf.set_cell(bottom, col, '─', chrome);
    }
    for row in top..=bottom {
        buf.set_cell(row, left, '│', chrome);
        buf.set_cell(row, right, '│', chrome);
    }
    buf.set_cell(top, left, '┌', chrome);
    buf.set_cell(top, right, '┐', chrome);
    buf.set_cell(bottom, left, '└', chrome);
    buf.set_cell(bottom, right, '┘', chrome);

    let title_width = layout
        .budget_title
        .chars()
        .count()
        .min(frame.body_width.saturating_sub(4));
    if title_width > 0 {
        let title_col = left + frame.body_width.saturating_sub(title_width + 2);
        layout::write_clipped(
            buf,
            top,
            title_col,
            title_width,
            &layout.budget_title,
            theme::label_style(),
        );
    }

    let inner_width = frame.body_width.saturating_sub(2);
    let unit_width = inner_width
        .saturating_sub(
            BUILD_TABLE_NUMBER_WIDTH
                + BUILD_TABLE_SEPARATOR_WIDTH
                + BUILD_TABLE_SEPARATOR_WIDTH
                + BUILD_TABLE_COST_WIDTH
                + BUILD_TABLE_SEPARATOR_WIDTH
                + BUILD_TABLE_QUEUE_WIDTH
                + BUILD_TABLE_SEPARATOR_WIDTH
                + BUILD_TABLE_STATUS_WIDTH,
        )
        .max(BUILD_TABLE_MIN_UNIT_WIDTH);

    let header = format!(
        "{:>2}│{:<unit_width$}│{:>4}│{:>5}│{:<6}",
        "#", "Unit", "Cost", "Queue", "Status"
    );
    layout::write_clipped(
        buf,
        top + 1,
        left + 1,
        inner_width,
        &header,
        theme::table_header_style(),
    );

    let divider = format!(
        "{}┼{}┼{}┼{}┼{}",
        "─".repeat(BUILD_TABLE_NUMBER_WIDTH),
        "─".repeat(unit_width),
        "─".repeat(BUILD_TABLE_COST_WIDTH),
        "─".repeat(BUILD_TABLE_QUEUE_WIDTH),
        "─".repeat(BUILD_TABLE_STATUS_WIDTH),
    );
    layout::write_clipped(
        buf,
        top + 2,
        left + 1,
        inner_width,
        &divider,
        theme::table_chrome_style(),
    );

    for (offset, entry) in layout.entries.iter().enumerate() {
        let row = top + 3 + offset;
        if row >= bottom {
            break;
        }
        let status = if entry.selectable { "" } else { "FULL" };
        let line = format!(
            "{:>2}│{:<unit_width$}│{:>4}│{:>5}│{:<6}",
            format!("{:02}", entry.number),
            entry.label,
            entry.cost,
            entry.queued_qty,
            status,
        );
        let style = if entry.selectable {
            theme::table_body_style()
        } else {
            theme::dim_style()
        };
        layout::write_clipped(buf, row, left + 1, inner_width, &line, style);
    }
}

fn build_specify_popup_layout(app: &DashApp, max_body_width: usize) -> BuildSpecifyPopupLayout {
    let entries = app.owned_planet_build_entries();
    let fixed_width = BUILD_TABLE_NUMBER_WIDTH
        + BUILD_TABLE_SEPARATOR_WIDTH
        + BUILD_TABLE_SEPARATOR_WIDTH
        + BUILD_TABLE_COST_WIDTH
        + BUILD_TABLE_SEPARATOR_WIDTH
        + BUILD_TABLE_QUEUE_WIDTH
        + BUILD_TABLE_SEPARATOR_WIDTH
        + BUILD_TABLE_STATUS_WIDTH
        + 2;
    let natural_unit_width = entries
        .iter()
        .map(|entry| entry.label.chars().count())
        .max()
        .unwrap_or("Unit".chars().count())
        .max("Unit".chars().count());
    let unit_width = natural_unit_width
        .min(max_body_width.saturating_sub(fixed_width))
        .max(BUILD_TABLE_MIN_UNIT_WIDTH);
    let budget_title = format!("BUDGET: {}", app.owned_planet_build_budget());
    let title_width = build_popup_title(app).chars().count().min(max_body_width);
    let table_total_width = (fixed_width + unit_width).min(max_body_width.max(fixed_width));
    let body_width = table_total_width
        .max(budget_title.chars().count())
        .max(title_width);
    BuildSpecifyPopupLayout {
        budget_title,
        body_width,
        table_height: entries.len() + 4,
        entries,
    }
}

fn build_specify_body_height(layout: &BuildSpecifyPopupLayout) -> usize {
    layout.table_height
}

fn build_popup_title(app: &DashApp) -> String {
    format!("BUILD ON PLANET: {}", popup_planet_name(app))
}

fn popup_planet_name(app: &DashApp) -> String {
    app.owned_planet_row()
        .map(|planet| planet.planet_name)
        .or_else(|| {
            app.owned_planet_record()
                .map(|planet| planet.status_or_name_summary())
        })
        .unwrap_or_else(|| String::from("Unknown"))
}

fn max_line_width(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
}

fn browse_lines(app: &DashApp, max_body_width: usize) -> Vec<String> {
    selected_planet_detail(app)
        .map(|detail| {
            crate::popups::planet_detail::popup_lines(&detail.popup_lines, max_body_width)
        })
        .unwrap_or_else(|| vec![String::from("No planet selected.")])
}

fn build_list_lines(app: &DashApp) -> Vec<String> {
    let mut lines = vec![String::from("Queued build orders:")];
    for entry in app.owned_planet_build_list_entries() {
        lines.push(format!(
            "{:<16} qty {:>3}  cost {:>3}",
            build_item_label(entry.kind),
            entry.queue_qty,
            entry.points
        ));
    }
    lines
}

fn build_quantity_lines(app: &DashApp) -> Vec<String> {
    let kind = app
        .owned_planet_popup
        .build_selected_kind
        .unwrap_or(ProductionItemKind::Destroyer);
    let label = build_item_label(kind);
    vec![
        format!("Selected unit: {label}"),
        format!("Maximum quantity: {}", app.owned_planet_popup.default),
    ]
}

fn commission_lines(app: &DashApp) -> Vec<String> {
    let mut lines = vec![String::from("Stardock slots ready to commission:")];
    for row in app.owned_planet_commission_entries() {
        lines.push(format!(
            "{:02}  {:<16} qty {:>3}",
            row.slot_0_based + 1,
            build_item_label(row.kind),
            row.qty
        ));
    }
    lines
}

fn transport_fleet_lines(app: &DashApp, mode: ArmyTransportMode) -> Vec<String> {
    let Some(planet) = app.owned_planet_row() else {
        return vec![String::from("Planet unavailable.")];
    };
    let mut lines = vec![format!(
        "{} ({:02},{:02})",
        planet.planet_name, planet.coords[0], planet.coords[1]
    )];
    for fleet in transport_fleet_candidates_for_planet(
        &app.game_data,
        app.player_record_index_1_based as u8,
        mode,
        &planet,
    )
    .into_iter()
    .filter(|fleet| fleet.available_qty > 0)
    {
        lines.push(format!(
            "Fleet {:02}  TTs {:>2}  loaded {:>2}  available {:>2}",
            fleet.fleet_number, fleet.troop_transports, fleet.loaded_armies, fleet.available_qty
        ));
    }
    lines
}

fn transport_quantity_lines(app: &DashApp, mode: ArmyTransportMode) -> Vec<String> {
    let action = match mode {
        ArmyTransportMode::Load => "load",
        ArmyTransportMode::Unload => "unload",
    };
    let fleet_number = app
        .owned_planet_popup
        .transport_selected_fleet_number
        .unwrap_or_default();
    let lines = vec![
        format!("Fleet {:02} selected.", fleet_number),
        format!(
            "Available armies to {action}: {}",
            app.owned_planet_popup.transport_available_qty
        ),
    ];
    lines
}

fn scorch_lines(app: &DashApp) -> Vec<String> {
    let planet = app
        .owned_planet_record()
        .map(|planet| {
            format!(
                "Planet \"{}\" at ({:02},{:02}).",
                planet.status_or_name_summary(),
                planet.coords_raw()[0],
                planet.coords_raw()[1]
            )
        })
        .unwrap_or_else(|| String::from("Selected planet unavailable."));
    vec![
        planet,
        String::new(),
        String::from("Scorch-Earth destroys anything useful to an invading force."),
        String::from("Factories, treasury, and surface infrastructure will be ruined."),
    ]
}

fn wrap_plain_lines(lines: &[String], width: usize) -> Vec<String> {
    let mut wrapped = Vec::new();
    for line in lines {
        if line.is_empty() {
            wrapped.push(String::new());
            continue;
        }
        wrapped.extend(wrap_plain_line(line, width.max(1)));
    }
    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

fn wrap_plain_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    if line.chars().count() <= width {
        return vec![line.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let word_width = word.chars().count();
        if current.is_empty() {
            if word_width <= width {
                current.push_str(word);
            } else {
                lines.extend(chunk_word(word, width));
            }
            continue;
        }
        if current.chars().count() + 1 + word_width <= width {
            current.push(' ');
            current.push_str(word);
            continue;
        }
        lines.push(current);
        current = String::new();
        if word_width <= width {
            current.push_str(word);
        } else {
            let mut chunks = chunk_word(word, width);
            current = chunks.pop().unwrap_or_default();
            lines.extend(chunks);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn chunk_word(word: &str, width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        if current.chars().count() == width {
            chunks.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn build_item_label(kind: ProductionItemKind) -> &'static str {
    build_unit_spec_by_kind(kind)
        .map(|unit| unit.label)
        .unwrap_or_else(|| match kind {
            ProductionItemKind::Starbase => "Starbase",
            ProductionItemKind::GroundBattery => "Ground Battery",
            ProductionItemKind::Army => "Army",
            _ => "?",
        })
}
