use crate::app::state::{DashApp, OwnedPlanetPopupMode};
use crate::buffer::PlayfieldBuffer;
use crate::layout::{self, MapWidgetFrame, dashboard};
use crate::modal::Rect;
use crate::overlays::frame::{
    OverlaySizePolicy, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent,
};
use crate::planet_view::selected_planet_detail;
use crate::table::TableFooter;
use crate::theme;
use nc_data::ProductionItemKind;
use nc_engine::{ArmyTransportMode, build_unit_spec_by_kind, transport_fleet_candidates_for_planet};

pub fn draw(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    map_frame: MapWidgetFrame,
    planet_record_index_1_based: usize,
) {
    let max_body_width = map_frame.outer.width.saturating_sub(6).max(1);
    let (lines, footer) = popup_content(app, max_body_width);
    let body_width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(1);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "INFO ABOUT A PLANET:",
        body_width,
        lines.len(),
        OverlaySizePolicy::default(),
        footer,
        app.popup_position_for(crate::app::state::ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        }),
    );
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

pub fn popup_rect(
    app: &DashApp,
    map_frame: MapWidgetFrame,
    planet_record_index_1_based: usize,
) -> Rect {
    let max_body_width = map_frame.outer.width.saturating_sub(6).max(1);
    let (lines, footer) = popup_content(app, max_body_width);
    let body_width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(1);
    overlay_popup_rect_for_body_in_parent(
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "INFO ABOUT A PLANET:",
        body_width,
        lines.len(),
        OverlaySizePolicy::default(),
        footer,
        app.popup_position_for(crate::app::state::ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        }),
    )
}

fn popup_content<'a>(app: &'a DashApp, max_body_width: usize) -> (Vec<String>, TableFooter<'a>) {
    match app.owned_planet_popup.mode {
        OwnedPlanetPopupMode::Browse => (
            browse_lines(app, max_body_width),
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: "B D A C M L U X ->",
            },
        ),
        OwnedPlanetPopupMode::BuildList => (
            wrap_plain_lines(&build_list_lines(app), max_body_width),
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: "<ESC> ->",
            },
        ),
        OwnedPlanetPopupMode::BuildAbortConfirm => (
            vec![String::from("Abort all queued builds for this planet?")],
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: "Abort queued builds? Y/[N] ->",
            },
        ),
        OwnedPlanetPopupMode::BuildSpecify => (
            wrap_plain_lines(&build_specify_lines(app), max_body_width),
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: "Unit number or 0 if done",
                default: "0",
                input: &app.owned_planet_popup.input,
            },
        ),
        OwnedPlanetPopupMode::BuildQuantity => {
            let default = if app.owned_planet_popup.default.is_empty() {
                "0"
            } else {
                app.owned_planet_popup.default.as_str()
            };
            (
                wrap_plain_lines(&build_quantity_lines(app), max_body_width),
                TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: "Qty",
                    default,
                    input: &app.owned_planet_popup.input,
                },
            )
        }
        OwnedPlanetPopupMode::CommissionSelect => {
            let default = if app.owned_planet_popup.default.is_empty() {
                "01"
            } else {
                app.owned_planet_popup.default.as_str()
            };
            (
                wrap_plain_lines(&commission_lines(app), max_body_width),
                TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: "Commission slot #",
                    default,
                    input: &app.owned_planet_popup.input,
                },
            )
        }
        OwnedPlanetPopupMode::CommissionResult => (
            wrap_plain_lines(&app.owned_planet_popup.report_lines, max_body_width),
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: "<ESC> ->",
            },
        ),
        OwnedPlanetPopupMode::MassCommissionConfirm => (
            wrap_plain_lines(
                &[
                    String::from("Automatically commission all ships and starbases in stardock?"),
                ],
                max_body_width,
            ),
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: "Mass commission? Y/[N] ->",
            },
        ),
        OwnedPlanetPopupMode::MassCommissionReport => (
            wrap_plain_lines(&app.owned_planet_popup.report_lines, max_body_width),
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: "<ESC> ->",
            },
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
            (
                wrap_plain_lines(&transport_fleet_lines(app, mode), max_body_width),
                TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt,
                    default,
                    input: &app.owned_planet_popup.input,
                },
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
            (
                wrap_plain_lines(&transport_quantity_lines(app, mode), max_body_width),
                TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt,
                    default,
                    input: &app.owned_planet_popup.input,
                },
            )
        }
        OwnedPlanetPopupMode::ScorchConfirm1 => (
            wrap_plain_lines(&scorch_lines(app), max_body_width),
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: "Are you sure? Y/[N] ->",
            },
        ),
        OwnedPlanetPopupMode::ScorchConfirm2 => (
            wrap_plain_lines(&scorch_lines(app), max_body_width),
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: "Are you really sure? Y/[N] ->",
            },
        ),
        OwnedPlanetPopupMode::ScorchConfirm3 => (
            wrap_plain_lines(&scorch_lines(app), max_body_width),
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: "Are you sure-sure? Last chance to bail! Y/[N] ->",
            },
        ),
    }
}

fn browse_lines(app: &DashApp, max_body_width: usize) -> Vec<String> {
    let mut lines = selected_planet_detail(app)
        .map(|detail| crate::popups::planet_detail::popup_lines(&detail.popup_lines, max_body_width))
        .unwrap_or_else(|| vec![String::from("No planet selected.")]);
    if let Some(status) = app.owned_planet_popup.status.as_deref() {
        lines.push(String::new());
        lines.extend(wrap_plain_lines(&[status.to_string()], max_body_width));
    }
    lines
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

fn build_specify_lines(app: &DashApp) -> Vec<String> {
    let mut lines = vec![String::from("Available units:")];
    for entry in app.owned_planet_build_entries() {
        lines.push(format!(
            "{:02}  {:<16} cost {:>3}  queued {:>2}{}",
            entry.number,
            entry.label,
            entry.cost,
            entry.queued_qty,
            if entry.selectable { "" } else { "  (full)" }
        ));
    }
    if let Some(status) = app.owned_planet_popup.status.as_deref() {
        lines.push(String::new());
        lines.push(status.to_string());
    }
    lines
}

fn build_quantity_lines(app: &DashApp) -> Vec<String> {
    let kind = app
        .owned_planet_popup
        .build_selected_kind
        .unwrap_or(ProductionItemKind::Destroyer);
    let label = build_item_label(kind);
    let mut lines = vec![
        format!("Selected unit: {label}"),
        format!("Maximum quantity: {}", app.owned_planet_popup.default),
    ];
    if let Some(status) = app.owned_planet_popup.status.as_deref() {
        lines.push(String::new());
        lines.push(status.to_string());
    }
    lines
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
    if let Some(status) = app.owned_planet_popup.status.as_deref() {
        lines.push(String::new());
        lines.push(status.to_string());
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
    if let Some(status) = app.owned_planet_popup.status.as_deref() {
        lines.push(String::new());
        lines.push(status.to_string());
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
    let mut lines = vec![
        format!("Fleet {:02} selected.", fleet_number),
        format!(
            "Available armies to {action}: {}",
            app.owned_planet_popup.transport_available_qty
        ),
    ];
    if let Some(status) = app.owned_planet_popup.status.as_deref() {
        lines.push(String::new());
        lines.push(status.to_string());
    }
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
