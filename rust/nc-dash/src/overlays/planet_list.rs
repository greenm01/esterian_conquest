//! P overlay: dashboard-sized planet management table.

use std::cmp::Ordering;

use nc_data::{EmpirePlanetEconomyRow, PlanetRecord, ProductionItemKind, STARDOCK_SLOT_COUNT};
use nc_engine::{BUILD_UNITS, build_kind_count_label, planet_build_view};
use nc_ui::PlayfieldBuffer;
use nc_ui::coords::{format_sector_coords_default, format_sector_coords_table};
use nc_ui::table::{
    SplitTableRow, TABLE_TEXT_INSET, TableColumn, TableFooter, TableWidthMode,
    centered_table_start_col, resolve_table_columns, table_render_width, write_split_table_at,
    write_stacked_table_window_with_theme_at, write_table_window_with_theme_at,
};
use nc_ui::table_filter::{FilterKind, TableFilterClause, TableFilterColumn};
use nc_ui::table_selection;

use crate::app::state::{
    ActiveOverlay, DashApp, PlanetOverlayFilter, PlanetOverlayPromptMode, PlanetOverlaySort,
    SortDirection,
};
use crate::layout::MapWidgetFrame;
use crate::layout::dashboard;
use crate::modal::Rect;
use crate::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin, max_overlay_body_width,
    overlay_popup_rect_for_body_in_parent, stacked_table_body_height, standard_table_body_height,
    write_clipped,
};
use crate::theme;

pub(crate) const HOTKEYS: &str = "? F S B D A <Q>";
const TOP_HEADERS: [&str; 13] = [
    "Coord", "", "Max", "Curr", "Trsry", "", "", "", "Build", "Star", "", "", "",
];
const COLUMNS: [TableColumn<'static>; 13] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 13),
    TableColumn::right("Prod", 4),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Bdgt", 5),
    TableColumn::right("Rev", 3),
    TableColumn::right("Grow", 4),
    TableColumn::right("Queue", 5),
    TableColumn::right("Dock", 4),
    TableColumn::right("SBs", 3),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
];

const FILTER_COLUMNS: &[TableFilterColumn] = &[
    TableFilterColumn {
        code: "coo",
        label: "Coord",
        aliases: &["coordinates", "location"],
        kind: FilterKind::Coord,
    },
    TableFilterColumn {
        code: "pla",
        label: "Planet",
        aliases: &["name"],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "max",
        label: "Max",
        aliases: &["maximum", "potential"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "cur",
        label: "Current",
        aliases: &["currentprod", "production", "current production"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "trs",
        label: "Treasury",
        aliases: &["points", "treasury points"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "bdg",
        label: "Budget",
        aliases: &["bdgt", "bgdt"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "rev",
        label: "Revenue",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "gro",
        label: "Growth",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "bui",
        label: "Build",
        aliases: &["queue"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "sta",
        label: "Dock",
        aliases: &["stardock"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "sbs",
        label: "Starbase",
        aliases: &["starbases"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "ars",
        label: "Armies",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "gbs",
        label: "Batteries",
        aliases: &["groundbatteries"],
        kind: FilterKind::Number,
    },
];

fn overlay_parent_rect(app: &DashApp) -> Rect {
    dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanetOverlayRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub planet_name: String,
    pub current_prod: u16,
    pub max_prod: u16,
    pub treasury: u32,
    pub budget: u32,
    pub revenue: i16,
    pub growth: i16,
    pub build_queue: u32,
    pub has_starbase: bool,
    pub docked: u32,
    pub armies: u8,
    pub batteries: u8,
    pub cells: Vec<String>,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::BuildList => {
            draw_build_list(buf, app, map_frame);
            return;
        }
        PlanetOverlayPromptMode::BuildSpecify => {
            draw_build_specify(buf, app, map_frame);
            return;
        }
        PlanetOverlayPromptMode::BuildQuantity => {
            draw_build_quantity(buf, app, map_frame);
            return;
        }
        _ => {}
    }
    let rows = table_rows(app);
    let selected = app
        .planet_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let command_bar = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: selected_default.as_deref(),
        input: &app.planet_overlay.jump_input,
    };
    let notice_footer_rows;
    let mut filter_prompt;
    let footer = match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::None => {
            if let Some(notice) = app.planet_overlay.footer_notice.as_deref() {
                notice_footer_rows = [
                    TableFooter::CommandText {
                        label: "COMMAND",
                        text: notice,
                    },
                    command_bar,
                ];
                TableFooter::Stacked {
                    rows: &notice_footer_rows,
                    active_row: 1,
                }
            } else {
                command_bar
            }
        }
        PlanetOverlayPromptMode::BuildAbortConfirm => TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "Abort queued builds? Y/[N] -> ",
        },
        PlanetOverlayPromptMode::SortMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = app
                    .planet_overlay
                    .prompt_status
                    .as_deref()
                    .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                    .map(|value| value.trim_start().to_string())
                    .unwrap_or_else(|| "Sort column [?] ".to_string());
                filter_prompt.as_str()
            },
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = "Filter column [?] ".to_string();
                if let Some(status) = app.planet_overlay.prompt_status.as_deref() {
                    filter_prompt.push_str(status);
                }
                filter_prompt.as_str()
            },
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterValueInput => {
            filter_prompt = format!(
                "Filter {} ",
                app.planet_overlay
                    .pending_filter_column
                    .map(|column| column.code)
                    .unwrap_or("value")
            );
            if let Some(status) = app.planet_overlay.prompt_status.as_deref() {
                filter_prompt.push_str(status);
            }
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.planet_overlay.prompt_default,
                input: &app.planet_overlay.prompt_input,
            }
        }
        PlanetOverlayPromptMode::BuildList
        | PlanetOverlayPromptMode::BuildSpecify
        | PlanetOverlayPromptMode::BuildQuantity => {
            unreachable!("build flows render separately")
        }
    };
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns)
        .max("You do not currently control any planets.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    let visible_rows = frame.body_height.saturating_sub(5);
    assert_overlay_body_write_fits(
        frame,
        &title,
        table_render_width(&columns),
        stacked_table_body_height(visible_rows),
    );
    let scroll = clamp_scroll(
        app.planet_overlay.scroll,
        selected,
        visible_rows,
        rows.len(),
    );
    let table_col = frame.body_col + centered_table_start_col(frame.body_width, &columns);
    let metrics = write_stacked_table_window_with_theme_at(
        buf,
        frame.body_row,
        table_col,
        &TOP_HEADERS,
        &columns,
        &table_cells,
        scroll,
        visible_rows,
        theme::table_theme(),
        table_cells.get(selected).map(|_| selected),
        0,
        None,
    );

    if rows.is_empty() {
        write_clipped(
            buf,
            metrics.bottom_row.saturating_sub(1),
            frame.body_col,
            frame.body_width,
            if app.planet_overlay.filter_clause.is_some() {
                "No planets match current filter."
            } else {
                "You do not currently control any planets."
            },
            theme::dim_style(),
        );
    }
}

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Option<Rect> {
    match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::BuildList => {
            return Some(build_list_popup_rect(app));
        }
        PlanetOverlayPromptMode::BuildSpecify => {
            return Some(build_specify_popup_rect(app));
        }
        PlanetOverlayPromptMode::BuildQuantity => {
            return Some(build_quantity_popup_rect(app));
        }
        _ => {}
    }
    let rows = table_rows(app);
    let selected = app
        .planet_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let command_bar = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: selected_default.as_deref(),
        input: &app.planet_overlay.jump_input,
    };
    let notice_footer_rows;
    let mut filter_prompt;
    let footer = match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::None => {
            if let Some(notice) = app.planet_overlay.footer_notice.as_deref() {
                notice_footer_rows = [
                    TableFooter::CommandText {
                        label: "COMMAND",
                        text: notice,
                    },
                    command_bar,
                ];
                TableFooter::Stacked {
                    rows: &notice_footer_rows,
                    active_row: 1,
                }
            } else {
                command_bar
            }
        }
        PlanetOverlayPromptMode::BuildAbortConfirm => TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "Abort queued builds? Y/[N] -> ",
        },
        PlanetOverlayPromptMode::SortMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = app
                    .planet_overlay
                    .prompt_status
                    .as_deref()
                    .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                    .map(|value| value.trim_start().to_string())
                    .unwrap_or_else(|| "Sort column [?] ".to_string());
                filter_prompt.as_str()
            },
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = "Filter column [?] ".to_string();
                if let Some(status) = app.planet_overlay.prompt_status.as_deref() {
                    filter_prompt.push_str(status);
                }
                filter_prompt.as_str()
            },
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterValueInput => {
            filter_prompt = format!(
                "Filter {} ",
                app.planet_overlay
                    .pending_filter_column
                    .map(|column| column.code)
                    .unwrap_or("value")
            );
            if let Some(status) = app.planet_overlay.prompt_status.as_deref() {
                filter_prompt.push_str(status);
            }
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.planet_overlay.prompt_default,
                input: &app.planet_overlay.prompt_input,
            }
        }
        PlanetOverlayPromptMode::BuildList
        | PlanetOverlayPromptMode::BuildSpecify
        | PlanetOverlayPromptMode::BuildQuantity => {
            unreachable!("build flows are not draggable")
        }
    };
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns)
        .max("You do not currently control any planets.".chars().count() + 4);
    Some(overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::PlanetList),
    ))
}

fn draw_build_specify(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let view = app.planet_build_view();
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries);
    let table_width = build_specify_table_width();
    let status_rows = usize::from(app.planet_overlay.build_unit_status.is_some());
    let max_unit_num = app.planet_build_max_selectable_unit_number();
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        "SPECIFY BUILD ORDERS",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
        OverlaySizePolicy::default(),
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &format!("Unit number or 0 if done (0 - {}) ", max_unit_num),
            default: "0",
            input: &app.planet_overlay.build_unit_input,
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    assert_overlay_body_write_fits(
        frame,
        "SPECIFY BUILD ORDERS",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
    );

    let Some(view) = view else {
        write_clipped(
            buf,
            frame.body_row,
            frame.body_col,
            frame.body_width,
            "No owned planets available for building.",
            theme::dim_style(),
        );
        return;
    };

    write_build_points_line(buf, frame, view.points_left, table_width);
    let table_col =
        frame.body_col + centered_table_start_col(frame.body_width, &build_specify_all_columns());
    let _ = write_split_table_at(
        buf,
        frame.body_row + 1,
        table_col,
        &BUILD_HALF_COLUMNS,
        &BUILD_HALF_COLUMNS,
        &split_rows,
        theme::value_style(),
    );
    if let Some(status) = app.planet_overlay.build_unit_status.as_deref() {
        write_clipped(
            buf,
            frame.body_row + 1 + standard_table_body_height(split_rows.len()),
            frame.body_col,
            frame.body_width,
            status,
            theme::error_style(),
        );
    }
}

fn draw_build_list(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let Some(view) = app.planet_build_view() else {
        return;
    };
    let entries = app.planet_build_list_entries();
    let table_cells = if entries.is_empty() {
        vec![vec![
            "No build orders are queued.".to_string(),
            String::new(),
            String::new(),
        ]]
    } else {
        entries
            .iter()
            .map(|entry| {
                vec![
                    build_kind_count_label(entry.kind, entry.queue_qty).to_string(),
                    entry.points.to_string(),
                    entry.queue_qty.to_string(),
                ]
            })
            .collect::<Vec<_>>()
    };
    const BUILD_LIST_COLUMNS: [TableColumn<'static>; 3] = [
        TableColumn::left("Unit", 24),
        TableColumn::right("Points", 6),
        TableColumn::right("Queue", 5),
    ];
    let columns = resolve_table_columns(
        &BUILD_LIST_COLUMNS,
        &table_cells,
        max_overlay_body_width(_map_frame),
        false,
        TableWidthMode::Compact,
    );
    let table_width = table_render_width(&columns);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        "QUEUED BUILDS",
        table_width,
        1 + standard_table_body_height(table_cells.len()),
        OverlaySizePolicy::default(),
        TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "? <Q> -> ",
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    assert_overlay_body_write_fits(
        frame,
        "QUEUED BUILDS",
        table_width,
        1 + standard_table_body_height(table_cells.len()),
    );
    write_build_points_line(buf, frame, view.points_left, table_width);
    let table_col =
        frame.body_col + centered_table_start_col(frame.body_width, &BUILD_LIST_COLUMNS);
    let _ = write_table_window_with_theme_at(
        buf,
        frame.body_row + 1,
        table_col,
        &columns,
        &table_cells,
        0,
        table_cells.len(),
        theme::table_theme(),
        None,
        0,
        None,
    );
}

fn build_list_popup_rect(app: &DashApp) -> Rect {
    let entries = app.planet_build_list_entries();
    let table_cells = if entries.is_empty() {
        vec![vec![
            "No build orders are queued.".to_string(),
            String::new(),
            String::new(),
        ]]
    } else {
        entries
            .iter()
            .map(|entry| {
                vec![
                    build_kind_count_label(entry.kind, entry.queue_qty).to_string(),
                    entry.points.to_string(),
                    entry.queue_qty.to_string(),
                ]
            })
            .collect::<Vec<_>>()
    };
    const BUILD_LIST_COLUMNS: [TableColumn<'static>; 3] = [
        TableColumn::left("Unit", 24),
        TableColumn::right("Points", 6),
        TableColumn::right("Queue", 5),
    ];
    let columns = resolve_table_columns(
        &BUILD_LIST_COLUMNS,
        &table_cells,
        80,
        false,
        TableWidthMode::Compact,
    );
    let table_width = table_render_width(&columns);
    overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        "QUEUED BUILDS",
        table_width,
        1 + standard_table_body_height(table_cells.len()),
        OverlaySizePolicy::default(),
        TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "? <Q> -> ",
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    )
}

fn draw_build_quantity(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let view = app.planet_build_view();
    let Some(kind) = app.planet_overlay.build_selected_kind else {
        draw_build_specify(buf, app, map_frame);
        return;
    };
    let Some(unit) = BUILD_UNITS.iter().copied().find(|unit| unit.kind == kind) else {
        draw_build_specify(buf, app, map_frame);
        return;
    };
    let max_qty = app.planet_build_max_quantity_for(kind).unwrap_or(0);
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries);
    let table_width = build_specify_table_width();
    let status_rows = usize::from(app.planet_overlay.build_quantity_status.is_some());
    let default_qty = max_qty.to_string();
    let prompt = format!(
        "How many new {} to build (0 - {}) ",
        unit.singular_label, max_qty
    );
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        "BUILD QUANTITY",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
        OverlaySizePolicy::default(),
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &prompt,
            default: &default_qty,
            input: &app.planet_overlay.build_quantity_input,
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    assert_overlay_body_write_fits(
        frame,
        "BUILD QUANTITY",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
    );

    let Some(view) = view else {
        write_clipped(
            buf,
            frame.body_row,
            frame.body_col,
            frame.body_width,
            "No owned planets available for building.",
            theme::dim_style(),
        );
        return;
    };

    write_build_points_line(buf, frame, view.points_left, table_width);
    let table_col =
        frame.body_col + centered_table_start_col(frame.body_width, &build_specify_all_columns());
    let _ = write_split_table_at(
        buf,
        frame.body_row + 1,
        table_col,
        &BUILD_HALF_COLUMNS,
        &BUILD_HALF_COLUMNS,
        &split_rows,
        theme::value_style(),
    );
    if let Some(status) = app.planet_overlay.build_quantity_status.as_deref() {
        write_clipped(
            buf,
            frame.body_row + 1 + standard_table_body_height(split_rows.len()),
            frame.body_col,
            frame.body_width,
            status,
            theme::error_style(),
        );
    }
}

fn build_specify_popup_rect(app: &DashApp) -> Rect {
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries);
    let table_width = build_specify_table_width();
    let status_rows = usize::from(app.planet_overlay.build_unit_status.is_some());
    let max_unit_num = app.planet_build_max_selectable_unit_number();
    let prompt = format!("Unit number or 0 if done (0 - {}) ", max_unit_num);
    overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        "SPECIFY BUILD ORDERS",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
        OverlaySizePolicy::default(),
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &prompt,
            default: "0",
            input: &app.planet_overlay.build_unit_input,
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    )
}

fn build_quantity_popup_rect(app: &DashApp) -> Rect {
    let Some(kind) = app.planet_overlay.build_selected_kind else {
        return build_specify_popup_rect(app);
    };
    let Some(unit) = BUILD_UNITS.iter().copied().find(|unit| unit.kind == kind) else {
        return build_specify_popup_rect(app);
    };
    let max_qty = app.planet_build_max_quantity_for(kind).unwrap_or(0);
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries);
    let table_width = build_specify_table_width();
    let status_rows = usize::from(app.planet_overlay.build_quantity_status.is_some());
    let default_qty = max_qty.to_string();
    let prompt = format!(
        "How many new {} to build (0 - {}) ",
        unit.singular_label, max_qty
    );
    overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        "BUILD QUANTITY",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
        OverlaySizePolicy::default(),
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &prompt,
            default: &default_qty,
            input: &app.planet_overlay.build_quantity_input,
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    )
}

const BUILD_HALF_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::left("NO.", 4),
    TableColumn::left("UNIT TYPE", 19),
    TableColumn::right("COST", 4),
    TableColumn::right("QTY.", 5),
];

fn build_specify_all_columns() -> [TableColumn<'static>; 8] {
    [
        BUILD_HALF_COLUMNS[0],
        BUILD_HALF_COLUMNS[1],
        BUILD_HALF_COLUMNS[2],
        BUILD_HALF_COLUMNS[3],
        BUILD_HALF_COLUMNS[0],
        BUILD_HALF_COLUMNS[1],
        BUILD_HALF_COLUMNS[2],
        BUILD_HALF_COLUMNS[3],
    ]
}

fn build_specify_table_width() -> usize {
    table_render_width(&build_specify_all_columns())
}

fn write_build_points_line(
    buf: &mut PlayfieldBuffer,
    frame: crate::overlays::frame::OverlayFrame,
    points_left: u32,
    table_width: usize,
) {
    let table_col =
        frame.body_col + centered_table_start_col(frame.body_width, &build_specify_all_columns());
    let points_left_label = format!("BUDGET: {}", points_left);
    let points_left_col = table_col + table_width - TABLE_TEXT_INSET - points_left_label.len();
    write_clipped(
        buf,
        frame.body_row,
        points_left_col,
        frame
            .body_width
            .saturating_sub(points_left_col.saturating_sub(frame.body_col)),
        &points_left_label,
        theme::title_style(),
    );
}

fn build_specify_split_rows(entries: &[nc_engine::PlanetBuildSpecifyEntry]) -> Vec<SplitTableRow> {
    let left_units = [0usize, 1, 2, 3, 4];
    let right_units = [5usize, 6, 7, 8];

    (0..left_units.len())
        .map(|idx| {
            let left = entries[left_units[idx]];
            let right = right_units
                .get(idx)
                .and_then(|entry_idx| entries.get(*entry_idx).copied());
            SplitTableRow {
                left_cells: build_specify_cells(left),
                right_cells: right.map(build_specify_cells).unwrap_or_else(|| {
                    vec![String::new(), String::new(), String::new(), String::new()]
                }),
            }
        })
        .collect()
}

fn build_specify_cells(entry: nc_engine::PlanetBuildSpecifyEntry) -> Vec<String> {
    vec![
        if entry.selectable {
            format!("<{:02}>", entry.number)
        } else {
            String::new()
        },
        entry.label.to_string(),
        format!("{:02}", entry.cost),
        format!("({})", entry.queued_qty),
    ]
}

pub(crate) fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    table_rows(app)
        .into_iter()
        .map(|row| vec![format_sector_coords_table(row.coords)])
        .collect()
}

pub(crate) fn table_rows(app: &DashApp) -> Vec<PlanetOverlayRow> {
    let starbase_coords = app
        .game_data
        .bases
        .records
        .iter()
        .filter(|base| {
            base.owner_empire_raw() == app.player_record_index_1_based as u8
                && base.active_flag_raw() != 0
        })
        .map(|base| base.coords_raw())
        .collect::<std::collections::BTreeSet<_>>();

    let mut rows = app
        .game_data
        .empire_planet_economy_rows(app.player_record_index_1_based)
        .iter()
        .filter_map(|row| {
            app.game_data
                .planets
                .records
                .get(row.planet_record_index_1_based.saturating_sub(1))
                .map(|planet| {
                    format_planet_row_cells(
                        &app.game_data,
                        row,
                        planet,
                        starbase_coords.contains(&row.coords),
                    )
                })
        })
        .collect::<Vec<_>>();

    rows.retain(|row| match app.planet_overlay.filter {
        PlanetOverlayFilter::All => true,
        PlanetOverlayFilter::Range { anchor, radius } => {
            distance_sq(anchor, row.coords) <= u32::from(radius) * u32::from(radius)
        }
        PlanetOverlayFilter::Starbase => row.has_starbase,
    });
    if let Some(clause) = &app.planet_overlay.filter_clause {
        rows.retain(|row| planet_row_matches_clause(row, clause));
    }

    rows.sort_by(|left, right| match app.planet_overlay.sort {
        PlanetOverlaySort::Location => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.coords.cmp(&right.coords),
        ),
        PlanetOverlaySort::PlanetName => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.planet_name.cmp(&right.planet_name),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::MaxProduction => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.max_prod.cmp(&right.max_prod),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::CurrentProduction => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.current_prod.cmp(&right.current_prod),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Treasury => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.treasury.cmp(&right.treasury),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Budget => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.budget.cmp(&right.budget),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Revenue => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.revenue.cmp(&right.revenue),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Growth => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.growth.cmp(&right.growth),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::BuildQueue => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.build_queue.cmp(&right.build_queue),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Stardock => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.docked.cmp(&right.docked),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Starbase => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.has_starbase.cmp(&right.has_starbase),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Armies => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.armies.cmp(&right.armies),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Batteries => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.batteries.cmp(&right.batteries),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
    });

    rows
}

fn clamp_scroll(scroll: usize, selected: usize, max_rows: usize, total_rows: usize) -> usize {
    if max_rows == 0 || total_rows <= max_rows {
        return 0;
    }
    if selected < scroll {
        return selected;
    }
    if selected >= scroll + max_rows {
        return selected + 1 - max_rows;
    }
    scroll.min(total_rows.saturating_sub(max_rows))
}

fn format_planet_row_cells(
    game_data: &nc_data::CoreGameData,
    row: &EmpirePlanetEconomyRow,
    planet: &PlanetRecord,
    has_starbase: bool,
) -> PlanetOverlayRow {
    let coords = planet.coords_raw();
    let (stored, budget) = effective_points_left(game_data, row);
    let queue = build_queue_total(planet);
    let docked = docked_total(planet);
    let growth = row.yearly_growth_delta as i16;

    PlanetOverlayRow {
        planet_record_index_1_based: row.planet_record_index_1_based,
        coords,
        planet_name: row.planet_name.clone(),
        current_prod: row.present_production,
        max_prod: row.potential_production,
        treasury: stored,
        budget,
        revenue: row.yearly_tax_revenue as i16,
        growth,
        build_queue: queue,
        has_starbase,
        docked,
        armies: planet.army_count_raw(),
        batteries: planet.ground_batteries_raw(),
        cells: vec![
            format_sector_coords_table(coords),
            truncate(&row.planet_name, 13),
            row.potential_production.to_string(),
            row.present_production.to_string(),
            stored.to_string(),
            budget.to_string(),
            row.yearly_tax_revenue.to_string(),
            format!("{growth:+}"),
            queue.to_string(),
            docked.to_string(),
            u8::from(has_starbase).to_string(),
            planet.army_count_raw().to_string(),
            planet.ground_batteries_raw().to_string(),
        ],
    }
}

fn effective_points_left(
    game_data: &nc_data::CoreGameData,
    row: &EmpirePlanetEconomyRow,
) -> (u32, u32) {
    planet_build_view(game_data, row)
        .map(|view| (view.treasury_left, view.points_left))
        .unwrap_or_else(|_| {
            (
                row.stored_production_points,
                u32::from(row.build_capacity).min(row.stored_production_points),
            )
        })
}

fn build_queue_total(planet: &PlanetRecord) -> u32 {
    (0..10)
        .map(|slot| {
            let points = u32::from(planet.build_count_raw(slot));
            let kind = ProductionItemKind::from_raw(planet.build_kind_raw(slot));
            let Some(cost) = kind.build_cost() else {
                return 0;
            };
            points / cost
        })
        .sum()
}

fn docked_total(planet: &PlanetRecord) -> u32 {
    (0..STARDOCK_SLOT_COUNT)
        .map(|slot| u32::from(planet.stardock_count_raw(slot)))
        .sum()
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

pub(crate) fn sync_cursor_to_jump_input(app: &mut DashApp) -> bool {
    let rows = selection_rows(app);
    let Some(matched) = table_selection::find_typed_jump(&rows, 0, &app.planet_overlay.jump_input)
    else {
        return false;
    };
    app.planet_overlay.selected = matched.index;
    matched.is_terminal_exact_match
}

fn distance_sq(a: [u8; 2], b: [u8; 2]) -> u32 {
    let dx = i32::from(a[0]) - i32::from(b[0]);
    let dy = i32::from(a[1]) - i32::from(b[1]);
    (dx * dx + dy * dy) as u32
}

fn overlay_title(app: &DashApp) -> String {
    format!(
        "PLANET LIST: {} {} {}",
        sort_label(app.planet_overlay.sort),
        app.planet_overlay.sort_direction.title_label(),
        app.planet_overlay
            .filter_clause
            .as_ref()
            .map(|clause| clause.summary.as_str())
            .unwrap_or(filter_label(app.planet_overlay.filter))
    )
}

const fn sort_label(sort: PlanetOverlaySort) -> &'static str {
    match sort {
        PlanetOverlaySort::Location => "COO",
        PlanetOverlaySort::PlanetName => "PLA",
        PlanetOverlaySort::MaxProduction => "MAX",
        PlanetOverlaySort::CurrentProduction => "CUR",
        PlanetOverlaySort::Treasury => "TRS",
        PlanetOverlaySort::Budget => "BDG",
        PlanetOverlaySort::Revenue => "REV",
        PlanetOverlaySort::Growth => "GRO",
        PlanetOverlaySort::BuildQueue => "BUI",
        PlanetOverlaySort::Stardock => "STA",
        PlanetOverlaySort::Starbase => "SBS",
        PlanetOverlaySort::Armies => "ARS",
        PlanetOverlaySort::Batteries => "GBS",
    }
}

pub(crate) fn filter_columns() -> &'static [TableFilterColumn] {
    FILTER_COLUMNS
}

pub(crate) fn filter_default_value(app: &DashApp, column: TableFilterColumn) -> String {
    let row = table_rows(app).get(app.planet_overlay.selected).cloned();
    let Some(row) = row else {
        return String::new();
    };
    match column.code {
        "coo" => format!("{},{}", row.coords[0], row.coords[1]),
        "pla" => row.cells[1].clone(),
        "max" => row.cells[2].clone(),
        "cur" => row.cells[3].clone(),
        "trs" => row.cells[4].clone(),
        "bdg" => row.cells[5].clone(),
        "rev" => row.cells[6].clone(),
        "gro" => row.cells[7].clone(),
        "bui" => row.cells[8].clone(),
        "sta" => row.cells[9].clone(),
        "sbs" => row.cells[10].clone(),
        "ars" => row.cells[11].clone(),
        "gbs" => row.cells[12].clone(),
        _ => String::new(),
    }
}

pub(crate) fn planet_row_matches_clause(
    row: &PlanetOverlayRow,
    clause: &TableFilterClause,
) -> bool {
    match clause.column.code {
        "coo" => clause.predicate.matches_coord(row.coords),
        "pla" => clause.predicate.matches_text(Some(&row.cells[1])),
        "max" => clause
            .predicate
            .matches_number(row.cells[2].parse::<i64>().ok()),
        "cur" => clause
            .predicate
            .matches_number(row.cells[3].parse::<i64>().ok()),
        "trs" => clause
            .predicate
            .matches_number(row.cells[4].parse::<i64>().ok()),
        "bdg" => clause
            .predicate
            .matches_number(row.cells[5].parse::<i64>().ok()),
        "rev" => clause
            .predicate
            .matches_number(row.cells[6].parse::<i64>().ok()),
        "gro" => clause
            .predicate
            .matches_number(row.cells[7].parse::<i64>().ok()),
        "bui" => clause
            .predicate
            .matches_number(row.cells[8].parse::<i64>().ok()),
        "sta" => clause
            .predicate
            .matches_number(row.cells[9].parse::<i64>().ok()),
        "sbs" => clause
            .predicate
            .matches_number(row.cells[10].parse::<i64>().ok()),
        "ars" => clause
            .predicate
            .matches_number(row.cells[11].parse::<i64>().ok()),
        "gbs" => clause
            .predicate
            .matches_number(row.cells[12].parse::<i64>().ok()),
        _ => true,
    }
}

#[cfg(test)]
fn sort_footer_label(app: &DashApp) -> String {
    format!("SORT {}", app.planet_overlay.sort_direction.label())
}

fn filter_label(filter: crate::app::state::PlanetOverlayFilter) -> &'static str {
    match filter {
        crate::app::state::PlanetOverlayFilter::All => "ALL",
        crate::app::state::PlanetOverlayFilter::Range { .. } => "RNG",
        crate::app::state::PlanetOverlayFilter::Starbase => "SB",
    }
}

fn apply_sort_direction(direction: SortDirection, ordering: Ordering) -> Ordering {
    match direction {
        SortDirection::Asc => ordering,
        SortDirection::Desc => ordering.reverse(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nc_data::{GameStateBuilder, ProductionItemKind};
    use nc_ui::ScreenGeometry;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn browse_hotkeys_match_supported_planet_list_commands() {
        assert_eq!(HOTKEYS, "? F S B D A <Q>");
    }

    #[test]
    fn titles_and_sort_footer_show_direction() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.planet_overlay.sort_direction = SortDirection::Asc;

        assert_eq!(overlay_title(&app), "PLANET LIST: CUR ASCENDING ALL");
        assert_eq!(sort_footer_label(&app), "SORT ASC");
    }

    #[test]
    fn build_specify_rows_keep_all_choices_and_blank_unselectable_tags() {
        let rows = build_specify_split_rows(&[
            nc_engine::PlanetBuildSpecifyEntry {
                number: 1,
                kind: ProductionItemKind::Destroyer,
                label: "Destroyers",
                cost: 5,
                queued_qty: 0,
                selectable: true,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 2,
                kind: ProductionItemKind::Cruiser,
                label: "Cruisers",
                cost: 15,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 3,
                kind: ProductionItemKind::Battleship,
                label: "Battleships",
                cost: 45,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 4,
                kind: ProductionItemKind::Scout,
                label: "Scouts",
                cost: 15,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 5,
                kind: ProductionItemKind::Transport,
                label: "Troop transports",
                cost: 5,
                queued_qty: 0,
                selectable: true,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 6,
                kind: ProductionItemKind::Etac,
                label: "ETACs",
                cost: 20,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 7,
                kind: ProductionItemKind::Starbase,
                label: "Starbases",
                cost: 50,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 9,
                kind: ProductionItemKind::Army,
                label: "Armies",
                cost: 2,
                queued_qty: 3,
                selectable: true,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 10,
                kind: ProductionItemKind::GroundBattery,
                label: "Ground batteries",
                cost: 20,
                queued_qty: 1,
                selectable: false,
            },
        ]);

        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].left_cells[0], "<01>");
        assert_eq!(rows[0].right_cells[0], "");
        assert_eq!(rows[2].right_cells[0], "<09>");
        assert_eq!(rows[2].right_cells[3], "(3)");
        assert_eq!(rows[3].right_cells[0], "");
        assert_eq!(rows[4].left_cells[0], "<05>");
        assert!(rows[4].right_cells.iter().all(|cell| cell.is_empty()));
    }

    #[test]
    fn planet_table_rows_show_treasury_and_budget_after_pending_build_spend() {
        let mut game_data = GameStateBuilder::new()
            .with_player_count(4)
            .build_initialized_baseline()
            .expect("baseline");
        let planet = &mut game_data.planets.records[0];
        planet.set_stored_production_points(165);
        planet.set_build_kind_raw(0, 1);
        planet.set_build_count_raw(0, 5);
        planet.set_build_kind_raw(1, 6);
        planet.set_build_count_raw(1, 40);

        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            game_data,
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );

        let row = table_rows(&app)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == 1)
            .expect("homeworld row");

        assert_eq!(row.cells[4], "120");
        assert_eq!(row.cells[5], "55");
    }
}
