use nc_data::{EmpirePlanetEconomyRow, GameStateMutationError, ProductionItemKind};
use nc_engine::{
    BUILD_UNITS, BuildUnitSpec, build_unit_spec, build_unit_spec_by_kind, max_quantity,
};

use crate::overlays::planet_list;

use super::state::{DashApp, HelpContext, PlanetOverlayPromptMode};

#[derive(Debug, Clone)]
pub(crate) struct PlanetBuildOverlayView {
    pub row: EmpirePlanetEconomyRow,
    pub committed_points: u32,
    pub available_points: u32,
    pub points_left: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlanetBuildOrderLine {
    pub kind: ProductionItemKind,
    pub points_remaining: u8,
}

impl DashApp {
    pub(crate) fn planet_build_orders(&self) -> Vec<PlanetBuildOrderLine> {
        let Some(planet_record_index_1_based) = self.planet_build_planet_record_index_1_based() else {
            return Vec::new();
        };
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(planet_record_index_1_based.saturating_sub(1))
        else {
            return Vec::new();
        };
        (0..10)
            .filter_map(|slot| {
                let points = record.build_count_raw(slot);
                let kind_raw = record.build_kind_raw(slot);
                if points == 0 || kind_raw == 0 {
                    None
                } else {
                    Some(PlanetBuildOrderLine {
                        kind: ProductionItemKind::from_raw(kind_raw),
                        points_remaining: points,
                    })
                }
            })
            .collect()
    }

    pub(crate) fn planet_build_view(&self) -> Option<PlanetBuildOverlayView> {
        let planet_record_index_1_based = self.planet_build_planet_record_index_1_based()?;
        let row = self
            .game_data
            .empire_planet_economy_rows(self.player_record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet_record_index_1_based)?;
        let committed_points = self.current_build_committed_points(planet_record_index_1_based).ok()?;
        let available_points = u32::from(row.build_capacity)
            .min(row.stored_production_points.min(u32::from(u16::MAX)));
        Some(PlanetBuildOverlayView {
            row,
            committed_points,
            available_points,
            points_left: available_points.saturating_sub(committed_points),
        })
    }

    pub(crate) fn planet_build_max_quantity_for(
        &self,
        kind: ProductionItemKind,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let Some(view) = self.planet_build_view() else {
            return Ok(0);
        };
        let unit = build_unit_spec_by_kind(kind).ok_or("unit spec missing")?;
        let queue_capacity = self
            .game_data
            .planet_additional_build_points_capacity(view.row.planet_record_index_1_based, kind)?;
        let mut max_qty = max_quantity(view.points_left.min(queue_capacity), unit.cost);
        match kind {
            ProductionItemKind::Army => {
                let free = self
                    .game_data
                    .planet_free_army_capacity(view.row.planet_record_index_1_based)?;
                max_qty = max_qty.min(u32::from(free));
            }
            ProductionItemKind::GroundBattery => {
                let free = self
                    .game_data
                    .planet_free_ground_battery_capacity(view.row.planet_record_index_1_based)?;
                max_qty = max_qty.min(u32::from(free));
            }
            _ => {}
        }
        Ok(max_qty)
    }

    pub(crate) fn planet_build_available_units(&self) -> Vec<(BuildUnitSpec, u32)> {
        let Some(view) = self.planet_build_view() else {
            return Vec::new();
        };
        BUILD_UNITS
            .iter()
            .copied()
            .filter_map(|unit| {
                let max_qty = self.planet_build_max_quantity_for(unit.kind).ok()?;
                (view.points_left > 0 && max_qty > 0).then_some((unit, max_qty))
            })
            .collect()
    }

    pub(crate) fn open_planet_build_specify(&mut self) {
        let rows = planet_list::table_rows(self);
        let selected = self
            .planet_overlay
            .selected
            .min(rows.len().saturating_sub(1));
        let Some(row) = rows.get(selected) else {
            return;
        };
        self.planet_overlay.build_planet_record_index_1_based = Some(row.planet_record_index_1_based);
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_unit_notice = None;
        self.planet_overlay.build_selected_kind = None;
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::BuildSpecify;
    }

    pub(crate) fn append_planet_build_unit_char(&mut self, ch: char) {
        if self.planet_overlay.build_unit_input.len() < 2 {
            self.planet_overlay.build_unit_input.push(ch);
            self.planet_overlay.build_unit_status = None;
            self.planet_overlay.build_unit_notice = None;
        }
    }

    pub(crate) fn backspace_planet_build_unit_input(&mut self) {
        self.planet_overlay.build_unit_input.pop();
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_unit_notice = None;
    }

    pub(crate) fn submit_planet_build_unit(&mut self) {
        let raw = self.planet_overlay.build_unit_input.trim();
        let number = if raw.is_empty() {
            0
        } else if let Ok(value) = raw.parse::<u8>() {
            value
        } else {
            self.planet_overlay.build_unit_status = Some("Enter a valid unit number.".to_string());
            return;
        };

        if number == 0 {
            self.close_planet_build_overlay();
            return;
        }

        let Some(unit) = build_unit_spec(number) else {
            self.planet_overlay.build_unit_status = Some("That unit is not available.".to_string());
            return;
        };

        let Ok(max_qty) = self.planet_build_max_quantity_for(unit.kind) else {
            self.planet_overlay.build_unit_status = Some("No points are available to spend.".to_string());
            return;
        };
        if max_qty == 0 {
            self.planet_overlay.build_unit_status = Some(self.planet_build_unavailable_message(unit.kind));
            return;
        }

        self.planet_overlay.build_selected_kind = Some(unit.kind);
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay.build_unit_notice = None;
        self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::BuildQuantity;
    }

    pub(crate) fn append_planet_build_quantity_char(&mut self, ch: char) {
        if self.planet_overlay.build_quantity_input.len() < 3 {
            self.planet_overlay.build_quantity_input.push(ch);
            self.planet_overlay.build_quantity_status = None;
        }
    }

    pub(crate) fn backspace_planet_build_quantity_input(&mut self) {
        self.planet_overlay.build_quantity_input.pop();
        self.planet_overlay.build_quantity_status = None;
    }

    pub(crate) fn submit_planet_build_quantity(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.planet_overlay.build_selected_kind else {
            self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::BuildSpecify;
            return Ok(());
        };
        let Some(unit) = build_unit_spec_by_kind(kind) else {
            self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::BuildSpecify;
            return Ok(());
        };
        let max_qty = self.planet_build_max_quantity_for(kind)?;
        if max_qty == 0 {
            self.planet_overlay.build_quantity_status = Some(self.planet_build_unavailable_message(kind));
            return Ok(());
        }

        let qty = if self.planet_overlay.build_quantity_input.trim().is_empty() {
            max_qty
        } else {
            match self.planet_overlay.build_quantity_input.trim().parse::<u32>() {
                Ok(value) => value,
                Err(_) => {
                    self.planet_overlay.build_quantity_status = Some("Enter a valid quantity.".to_string());
                    return Ok(());
                }
            }
        };

        if qty == 0 {
            self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::BuildSpecify;
            self.planet_overlay.build_quantity_input.clear();
            return Ok(());
        }
        if qty > max_qty {
            self.planet_overlay.build_quantity_status =
                Some(format!("Enter a quantity from 0 to {}.", max_qty));
            return Ok(());
        }

        let Some(planet_record_index_1_based) = self.planet_build_planet_record_index_1_based() else {
            self.close_planet_build_overlay();
            return Ok(());
        };

        let needs_stardock = !matches!(kind, ProductionItemKind::Army | ProductionItemKind::GroundBattery);
        if needs_stardock {
            let capacity = self
                .game_data
                .planet_additional_build_points_capacity(planet_record_index_1_based, kind)?;
            if capacity == 0 {
                self.planet_overlay.build_quantity_status =
                    Some("Stardock is full — commission ships first to free space.".to_string());
                return Ok(());
            }
        }

        let points = qty.saturating_mul(unit.cost);
        match self.game_data.append_planet_build_order(
            planet_record_index_1_based,
            points,
            production_item_kind_raw(kind),
        ) {
            Ok(()) => {}
            Err(GameStateMutationError::PlanetBuildQueueFull { .. }) => {
                self.planet_overlay.build_quantity_status =
                    Some("Build queue is full for this planet.".to_string());
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        }

        self.save_and_refresh_runtime()?;
        self.reselect_planet_overlay_row(planet_record_index_1_based);
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_unit_notice = Some(format!("Queued {} {}.", qty, unit.label));
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay.build_selected_kind = None;
        self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::BuildSpecify;
        Ok(())
    }

    pub(crate) fn cancel_planet_build_quantity(&mut self) {
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::BuildSpecify;
    }

    pub(crate) fn close_planet_build_overlay(&mut self) {
        self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::None;
        self.planet_overlay.build_planet_record_index_1_based = None;
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_unit_notice = None;
        self.planet_overlay.build_selected_kind = None;
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.help_context = HelpContext::PlanetList;
    }

    fn planet_build_planet_record_index_1_based(&self) -> Option<usize> {
        self.planet_overlay.build_planet_record_index_1_based
    }

    fn current_build_committed_points(
        &self,
        planet_record_index_1_based: usize,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let record = self
            .game_data
            .planets
            .records
            .get(planet_record_index_1_based.saturating_sub(1))
            .ok_or("planet record missing")?;
        Ok((0..10)
            .map(|slot| u32::from(record.build_count_raw(slot)))
            .sum::<u32>())
    }

    fn planet_build_unavailable_message(&self, kind: ProductionItemKind) -> String {
        let Some(view) = self.planet_build_view() else {
            return "No points are available to spend.".to_string();
        };
        if view.points_left == 0 {
            return "No points are available to spend.".to_string();
        }
        match kind {
            ProductionItemKind::Army => "Planet already has the maximum 255 armies.".to_string(),
            ProductionItemKind::GroundBattery => {
                "Planet already has the maximum 255 ground batteries.".to_string()
            }
            _ => "No points are available to spend.".to_string(),
        }
    }

    fn reselect_planet_overlay_row(&mut self, planet_record_index_1_based: usize) {
        if let Some(index) = planet_list::table_rows(self)
            .iter()
            .position(|row| row.planet_record_index_1_based == planet_record_index_1_based)
        {
            self.planet_overlay.selected = index;
        }
    }
}

pub(crate) fn build_order_point_total(orders: &[PlanetBuildOrderLine]) -> u32 {
    orders
        .iter()
        .map(|order| u32::from(order.points_remaining))
        .sum()
}

fn production_item_kind_raw(kind: ProductionItemKind) -> u8 {
    match kind {
        ProductionItemKind::Destroyer => 1,
        ProductionItemKind::Cruiser => 2,
        ProductionItemKind::Battleship => 3,
        ProductionItemKind::Scout => 4,
        ProductionItemKind::Transport => 5,
        ProductionItemKind::Etac => 6,
        ProductionItemKind::Starbase => 7,
        ProductionItemKind::Army => 9,
        ProductionItemKind::GroundBattery => 10,
        ProductionItemKind::Unknown(raw) => raw,
    }
}
