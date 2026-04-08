use nc_data::{EmpirePlanetEconomyRow, GameStateMutationError, ProductionItemKind};
use nc_engine::{
    build_unit_spec, build_unit_spec_by_kind, planet_build_max_quantity,
    planet_build_max_selectable_unit_number, planet_build_orders, planet_build_specify_entries,
    planet_build_unavailable_message, planet_build_view, production_item_kind_raw,
};

use crate::overlays::planet_list;

use super::state::{DashApp, HelpContext, PlanetOverlayPromptMode};

#[derive(Debug, Clone)]
pub(crate) struct PlanetBuildOverlayView {
    pub row: EmpirePlanetEconomyRow,
    pub points_left: u32,
}

impl DashApp {
    pub(crate) fn planet_build_view(&self) -> Option<PlanetBuildOverlayView> {
        let planet_record_index_1_based = self.planet_build_planet_record_index_1_based()?;
        let row = self
            .game_data
            .empire_planet_economy_rows(self.player_record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet_record_index_1_based)?;
        let view = planet_build_view(&self.game_data, &row).ok()?;
        Some(PlanetBuildOverlayView {
            row,
            points_left: view.points_left,
        })
    }

    pub(crate) fn planet_build_max_quantity_for(
        &self,
        kind: ProductionItemKind,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let Some(view) = self.planet_build_view() else {
            return Ok(0);
        };
        planet_build_max_quantity(&self.game_data, &view.row, kind).map_err(Into::into)
    }

    pub(crate) fn planet_build_specify_entries(&self) -> Vec<nc_engine::PlanetBuildSpecifyEntry> {
        let Some(view) = self.planet_build_view() else {
            return Vec::new();
        };
        planet_build_specify_entries(view.points_left, &planet_build_orders_for_dash(self))
    }

    pub(crate) fn planet_build_max_selectable_unit_number(&self) -> u8 {
        planet_build_max_selectable_unit_number(&self.planet_build_specify_entries())
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
        self.planet_overlay.build_planet_record_index_1_based =
            Some(row.planet_record_index_1_based);
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_selected_kind = None;
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay
            .open_prompt(PlanetOverlayPromptMode::BuildSpecify);
    }

    pub(crate) fn append_planet_build_unit_char(&mut self, ch: char) {
        if self.planet_overlay.build_unit_input.len() < 2 {
            self.planet_overlay.build_unit_input.push(ch);
            self.planet_overlay.build_unit_status = None;
        }
    }

    pub(crate) fn backspace_planet_build_unit_input(&mut self) {
        self.planet_overlay.build_unit_input.pop();
        self.planet_overlay.build_unit_status = None;
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
            self.planet_overlay.build_unit_status = Some("No build budget available.".to_string());
            return;
        };
        if max_qty == 0 {
            self.planet_overlay.build_unit_status =
                Some(self.planet_build_unavailable_message(unit.kind));
            return;
        }

        self.planet_overlay.build_selected_kind = Some(unit.kind);
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay
            .open_prompt(PlanetOverlayPromptMode::BuildQuantity);
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

    pub(crate) fn submit_planet_build_quantity(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.planet_overlay.build_selected_kind else {
            self.planet_overlay.close_prompt();
            return Ok(());
        };
        let Some(unit) = build_unit_spec_by_kind(kind) else {
            self.planet_overlay.close_prompt();
            return Ok(());
        };
        let max_qty = self.planet_build_max_quantity_for(kind)?;
        if max_qty == 0 {
            self.planet_overlay.build_quantity_status =
                Some(self.planet_build_unavailable_message(kind));
            return Ok(());
        }

        let qty = if self.planet_overlay.build_quantity_input.trim().is_empty() {
            max_qty
        } else {
            match self
                .planet_overlay
                .build_quantity_input
                .trim()
                .parse::<u32>()
            {
                Ok(value) => value,
                Err(_) => {
                    self.planet_overlay.build_quantity_status =
                        Some("Enter a valid quantity.".to_string());
                    return Ok(());
                }
            }
        };

        if qty == 0 {
            self.planet_overlay.close_prompt();
            self.planet_overlay.build_quantity_input.clear();
            self.planet_overlay.build_selected_kind = None;
            return Ok(());
        }
        if qty > max_qty {
            self.planet_overlay.build_quantity_status =
                Some(format!("Enter a quantity from 0 to {}.", max_qty));
            return Ok(());
        }

        let Some(planet_record_index_1_based) = self.planet_build_planet_record_index_1_based()
        else {
            self.close_planet_build_overlay();
            return Ok(());
        };

        let needs_stardock = !matches!(
            kind,
            ProductionItemKind::Army | ProductionItemKind::GroundBattery
        );
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
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay.build_selected_kind = None;
        self.planet_overlay.close_prompt();
        Ok(())
    }

    pub(crate) fn cancel_planet_build_quantity(&mut self) {
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.planet_overlay.build_selected_kind = None;
        self.planet_overlay.close_prompt();
    }

    pub(crate) fn close_planet_build_overlay(&mut self) {
        self.planet_overlay.close_prompt();
        self.planet_overlay.build_planet_record_index_1_based = None;
        self.planet_overlay.build_unit_input.clear();
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_selected_kind = None;
        self.planet_overlay.build_quantity_input.clear();
        self.planet_overlay.build_quantity_status = None;
        self.help_context = HelpContext::PlanetList;
    }

    fn planet_build_planet_record_index_1_based(&self) -> Option<usize> {
        self.planet_overlay.build_planet_record_index_1_based
    }

    fn planet_build_unavailable_message(&self, kind: ProductionItemKind) -> String {
        let Some(view) = self.planet_build_view() else {
            return "No build budget available.".to_string();
        };
        planet_build_unavailable_message(view.points_left, kind).to_string()
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

fn planet_build_orders_for_dash(app: &DashApp) -> Vec<nc_engine::PlanetBuildOrderLine> {
    let Some(planet_record_index_1_based) = app.planet_build_planet_record_index_1_based() else {
        return Vec::new();
    };
    let Some(record) = app
        .game_data
        .planets
        .records
        .get(planet_record_index_1_based.saturating_sub(1))
    else {
        return Vec::new();
    };
    planet_build_orders(record)
}
