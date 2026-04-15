use nc_data::{
    AutoCommissionEntry, AutoCommissionFleetEntry, AutoCommissionStarbaseEntry, CommissionResult,
    EmpirePlanetEconomyRow, PlanetRecord, ProductionItemKind,
};
use nc_engine::{
    ArmyTransportMode, PlanetCommissionSlotEntry, build_unit_spec, build_unit_spec_by_kind,
    planet_build_list_entries, planet_build_max_quantity, planet_build_orders,
    planet_build_specify_entries, planet_build_unavailable_message, planet_build_view,
    planet_commission_slot_entries, production_item_kind_raw,
    resolve_planet_transport_fleet_selection, transport_fleet_candidates_for_planet,
};

use super::state::{ActiveMouseGesture, ActivePopup, DashApp, OwnedPlanetPopupMode};

impl DashApp {
    pub(crate) fn open_owned_planet_popup(&mut self, planet_record_index_1_based: usize) {
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.owned_planet_popup.reset();
        self.popup = ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        };
    }

    pub(crate) fn close_owned_planet_popup(&mut self) {
        self.popup = ActivePopup::None;
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.owned_planet_popup.reset();
    }

    pub(crate) fn owned_planet_popup_record_index_1_based(&self) -> Option<usize> {
        match self.popup {
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based,
            } => Some(planet_record_index_1_based),
            _ => None,
        }
    }

    pub(crate) fn set_owned_planet_popup_mode(&mut self, mode: OwnedPlanetPopupMode) {
        self.owned_planet_popup.mode = mode;
        self.owned_planet_popup.input.clear();
        self.owned_planet_popup.default.clear();
        self.owned_planet_popup.status = None;
        self.owned_planet_popup.build_selected_kind = None;
        self.owned_planet_popup
            .transport_selected_fleet_record_index_1_based = None;
        self.owned_planet_popup.transport_selected_fleet_number = None;
        self.owned_planet_popup.transport_available_qty = 0;
        if !matches!(
            mode,
            OwnedPlanetPopupMode::CommissionResult | OwnedPlanetPopupMode::MassCommissionReport
        ) {
            self.owned_planet_popup.report_lines.clear();
        }
    }

    pub(crate) fn owned_planet_record(&self) -> Option<&PlanetRecord> {
        let record = self.owned_planet_popup_record_index_1_based()?;
        self.game_data.planets.records.get(record.saturating_sub(1))
    }

    pub(crate) fn owned_planet_row(&self) -> Option<EmpirePlanetEconomyRow> {
        let record = self.owned_planet_popup_record_index_1_based()?;
        self.game_data
            .empire_planet_economy_rows(self.player_record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == record)
    }

    pub(crate) fn owned_planet_build_orders(&self) -> Vec<nc_engine::PlanetBuildOrderLine> {
        self.owned_planet_record()
            .map(planet_build_orders)
            .unwrap_or_default()
    }

    pub(crate) fn owned_planet_build_entries(&self) -> Vec<nc_engine::PlanetBuildSpecifyEntry> {
        let Some(view) = self.owned_planet_build_view() else {
            return Vec::new();
        };
        planet_build_specify_entries(view.points_left, &self.owned_planet_build_orders())
    }

    pub(crate) fn owned_planet_build_budget(&self) -> u32 {
        self.owned_planet_build_view()
            .map(|view| view.points_left)
            .unwrap_or_default()
    }

    pub(crate) fn owned_planet_build_view(
        &self,
    ) -> Option<super::planet_build::PlanetBuildOverlayView> {
        let record = self.owned_planet_popup_record_index_1_based()?;
        let row = self
            .game_data
            .empire_planet_economy_rows(self.player_record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == record)?;
        let view = planet_build_view(&self.game_data, &row).ok()?;
        Some(super::planet_build::PlanetBuildOverlayView {
            row,
            points_left: view.points_left,
        })
    }

    pub(crate) fn owned_planet_build_list_entries(&self) -> Vec<nc_engine::PlanetBuildListEntry> {
        self.owned_planet_record()
            .map(planet_build_list_entries)
            .unwrap_or_default()
    }

    pub(crate) fn open_owned_planet_build_specify(&mut self) {
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::BuildSpecify);
        if !self.owned_planet_can_afford_any_build() {
            self.owned_planet_popup.status = Some("No build budget remains.".to_string());
            self.owned_planet_popup.mode = OwnedPlanetPopupMode::Browse;
            return;
        }
    }

    pub(crate) fn open_owned_planet_build_list(&mut self) {
        if self.owned_planet_build_list_entries().is_empty() {
            self.show_owned_planet_status("No build orders are queued.");
            return;
        }
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::BuildList);
    }

    pub(crate) fn open_owned_planet_build_abort_confirm(&mut self) {
        if self.owned_planet_build_list_entries().is_empty() {
            self.show_owned_planet_status("No build orders are queued.");
            return;
        }
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::BuildAbortConfirm);
    }

    pub(crate) fn append_owned_planet_input_char(&mut self, ch: char) {
        self.owned_planet_popup.input.push(ch);
        self.owned_planet_popup.status = None;
    }

    pub(crate) fn backspace_owned_planet_input(&mut self) {
        self.owned_planet_popup.input.pop();
        self.owned_planet_popup.status = None;
    }

    pub(crate) fn submit_owned_planet_build_unit(&mut self) {
        let raw = self.owned_planet_popup.input.trim();
        let number = if raw.is_empty() {
            0
        } else if let Ok(value) = raw.parse::<u8>() {
            value
        } else {
            self.owned_planet_popup.status = Some("Enter a valid unit number.".to_string());
            return;
        };
        if number == 0 {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            return;
        }
        let Some(unit) = build_unit_spec(number) else {
            self.owned_planet_popup.status = Some("That unit is not available.".to_string());
            return;
        };
        let Ok(max_qty) = self.owned_planet_build_max_quantity_for(unit.kind) else {
            self.owned_planet_popup.status = Some("No build budget available.".to_string());
            return;
        };
        if max_qty == 0 {
            self.owned_planet_popup.status =
                Some(self.owned_planet_build_unavailable_message(unit.kind));
            return;
        }
        self.owned_planet_popup.mode = OwnedPlanetPopupMode::BuildQuantity;
        self.owned_planet_popup.input.clear();
        self.owned_planet_popup.default = max_qty.to_string();
        self.owned_planet_popup.status = None;
        self.owned_planet_popup.build_selected_kind = Some(unit.kind);
    }

    pub(crate) fn submit_owned_planet_build_quantity(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.owned_planet_popup.build_selected_kind else {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            return Ok(());
        };
        let Some(unit) = build_unit_spec_by_kind(kind) else {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            return Ok(());
        };
        let max_qty = self.owned_planet_build_max_quantity_for(kind)?;
        if max_qty == 0 {
            self.owned_planet_popup.status =
                Some(self.owned_planet_build_unavailable_message(kind));
            return Ok(());
        }
        let qty = if self.owned_planet_popup.input.trim().is_empty() {
            max_qty
        } else {
            match self.owned_planet_popup.input.trim().parse::<u32>() {
                Ok(value) => value,
                Err(_) => {
                    self.owned_planet_popup.status = Some("Enter a valid quantity.".to_string());
                    return Ok(());
                }
            }
        };
        if qty == 0 {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            return Ok(());
        }
        if qty > max_qty {
            self.owned_planet_popup.status = Some(format!("Enter a quantity from 0 to {max_qty}."));
            return Ok(());
        }
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        let needs_stardock = !matches!(
            kind,
            ProductionItemKind::Army | ProductionItemKind::GroundBattery
        );
        if needs_stardock
            && self
                .game_data
                .planet_additional_build_points_capacity(record, kind)?
                == 0
        {
            self.owned_planet_popup.status =
                Some("Stardock is full — commission ships first to free space.".to_string());
            return Ok(());
        }
        let points = qty.saturating_mul(unit.cost);
        self.game_data
            .append_planet_build_order(record, points, production_item_kind_raw(kind))?;
        if let Ok(points_remaining_raw) = u8::try_from(points) {
            self.stage_hosted_planet_build(
                record,
                points_remaining_raw,
                production_item_kind_raw(kind),
            );
        }
        self.save_and_refresh_runtime()?;
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
        self.show_owned_planet_status(format!("Queued {} {}.", qty, unit.label));
        Ok(())
    }

    pub(crate) fn confirm_owned_planet_build_abort(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        self.game_data.clear_planet_build_queue(record)?;
        self.stage_hosted_planet_clear_build_queue(record);
        self.save_and_refresh_runtime()?;
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
        self.show_owned_planet_status("Build orders aborted.");
        Ok(())
    }

    pub(crate) fn open_owned_planet_commission_select(&mut self) {
        if self.owned_planet_commission_entries().is_empty() {
            self.show_owned_planet_status("No ships or starbases are waiting in stardock.");
            return;
        }
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::CommissionSelect);
        if let Some(entry) = self.owned_planet_commission_entries().first() {
            self.owned_planet_popup.default = format!("{:02}", entry.slot_0_based + 1);
        }
    }

    pub(crate) fn submit_owned_planet_commission(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = if self.owned_planet_popup.input.trim().is_empty() {
            self.owned_planet_popup.default.trim()
        } else {
            self.owned_planet_popup.input.trim()
        };
        let slot = raw
            .parse::<usize>()
            .map_err(|_| "Enter a valid slot number.")?;
        let Some(slot_0_based) = slot.checked_sub(1) else {
            return Err("Slot numbers are 1-based.".into());
        };
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        let result = self.game_data.commission_planet_stardock_slot(
            self.player_record_index_1_based,
            record,
            slot_0_based,
        )?;
        self.stage_hosted_planet_commission(record, slot_0_based);
        self.save_and_refresh_runtime()?;
        self.owned_planet_popup.report_lines = vec![self.format_commission_result_line(result)];
        self.owned_planet_popup.mode = OwnedPlanetPopupMode::CommissionResult;
        self.owned_planet_popup.input.clear();
        self.owned_planet_popup.default.clear();
        self.owned_planet_popup.status = None;
        Ok(())
    }

    pub(crate) fn open_owned_planet_mass_commission_confirm(&mut self) {
        if self.owned_planet_commission_entries().is_empty() {
            self.show_owned_planet_status("No ships or starbases are waiting in stardock.");
            return;
        }
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::MassCommissionConfirm);
    }

    pub(crate) fn confirm_owned_planet_mass_commission(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = self
            .game_data
            .auto_commission_all_stardock_units(self.player_record_index_1_based)?;
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        self.stage_hosted_planet_auto_commission(record);
        self.save_and_refresh_runtime()?;
        let lines = self.format_auto_commission_report_lines(&report);
        if lines.is_empty() {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            self.show_owned_planet_status("Nothing was available to commission.");
            return Ok(());
        }
        self.owned_planet_popup.report_lines = lines;
        self.owned_planet_popup.mode = OwnedPlanetPopupMode::MassCommissionReport;
        Ok(())
    }

    pub(crate) fn open_owned_planet_transport_fleet_select(&mut self, mode: ArmyTransportMode) {
        let Some(planet) = self.owned_planet_row() else {
            self.close_owned_planet_popup();
            return;
        };
        let candidates = transport_fleet_candidates_for_planet(
            &self.game_data,
            self.player_record_index_1_based as u8,
            mode,
            &planet,
        )
        .into_iter()
        .filter(|fleet| fleet.available_qty > 0)
        .collect::<Vec<_>>();
        let Some(first) = candidates.first() else {
            self.show_owned_planet_status(match mode {
                ArmyTransportMode::Load => "No fleets at this planet can take more armies.",
                ArmyTransportMode::Unload => "No fleets at this planet have loaded armies.",
            });
            return;
        };
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::TransportFleetSelect { mode });
        self.owned_planet_popup.default = first.fleet_number.to_string();
    }

    pub(crate) fn submit_owned_planet_transport_fleet(
        &mut self,
        mode: ArmyTransportMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = if self.owned_planet_popup.input.trim().is_empty() {
            self.owned_planet_popup.default.trim()
        } else {
            self.owned_planet_popup.input.trim()
        };
        let fleet_number = raw
            .parse::<u16>()
            .map_err(|_| "Enter one of your fleet numbers.")?;
        let owned_rows = self
            .game_data
            .empire_planet_economy_rows(self.player_record_index_1_based);
        let (fleet, _planet) = resolve_planet_transport_fleet_selection(
            &self.game_data,
            self.player_record_index_1_based as u8,
            mode,
            fleet_number,
            &owned_rows,
        )
        .map_err(|err| err.message())?;
        self.owned_planet_popup.mode = OwnedPlanetPopupMode::TransportQuantity { mode };
        self.owned_planet_popup.input.clear();
        self.owned_planet_popup.default = fleet.available_qty.to_string();
        self.owned_planet_popup.status = None;
        self.owned_planet_popup
            .transport_selected_fleet_record_index_1_based = Some(fleet.fleet_record_index_1_based);
        self.owned_planet_popup.transport_selected_fleet_number = Some(fleet.fleet_number);
        self.owned_planet_popup.transport_available_qty = fleet.available_qty;
        Ok(())
    }

    pub(crate) fn submit_owned_planet_transport_quantity(
        &mut self,
        mode: ArmyTransportMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(planet_record_index_1_based) = self.owned_planet_popup_record_index_1_based()
        else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        let Some(fleet_record_index_1_based) = self
            .owned_planet_popup
            .transport_selected_fleet_record_index_1_based
        else {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            return Ok(());
        };
        let qty = if self.owned_planet_popup.input.trim().is_empty() {
            self.owned_planet_popup.transport_available_qty
        } else {
            self.owned_planet_popup
                .input
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter a valid quantity.")?
        };
        if qty == 0 {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            return Ok(());
        }
        if qty > self.owned_planet_popup.transport_available_qty {
            return Err(format!(
                "Enter a quantity from 0 to {}.",
                self.owned_planet_popup.transport_available_qty
            )
            .into());
        }
        match mode {
            ArmyTransportMode::Load => {
                self.game_data.load_planet_armies_onto_fleet(
                    self.player_record_index_1_based,
                    planet_record_index_1_based,
                    fleet_record_index_1_based,
                    qty,
                )?;
                self.stage_hosted_fleet_load_armies(
                    fleet_record_index_1_based,
                    planet_record_index_1_based,
                    qty,
                );
            }
            ArmyTransportMode::Unload => {
                self.game_data.unload_fleet_armies_to_planet(
                    self.player_record_index_1_based,
                    planet_record_index_1_based,
                    fleet_record_index_1_based,
                    qty,
                )?;
                self.stage_hosted_fleet_unload_armies(
                    fleet_record_index_1_based,
                    planet_record_index_1_based,
                    qty,
                );
            }
        }
        self.save_and_refresh_runtime()?;
        let fleet_number = self
            .owned_planet_popup
            .transport_selected_fleet_number
            .unwrap_or_default();
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
        self.show_owned_planet_status(match mode {
            ArmyTransportMode::Load => {
                format!("Loaded {qty} armies onto Fleet {fleet_number:02}.")
            }
            ArmyTransportMode::Unload => {
                format!("Unloaded {qty} armies from Fleet {fleet_number:02}.")
            }
        });
        Ok(())
    }

    pub(crate) fn open_owned_planet_scorch_confirm(&mut self) {
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return;
        };
        if self.planet_scorch_orders.contains(&record) {
            self.show_owned_planet_status("Planet is already scorched.");
            return;
        }
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::ScorchConfirm1);
    }

    pub(crate) fn submit_owned_planet_scorch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        match self.owned_planet_popup.mode {
            OwnedPlanetPopupMode::ScorchConfirm1 => {
                self.owned_planet_popup.mode = OwnedPlanetPopupMode::ScorchConfirm2;
            }
            OwnedPlanetPopupMode::ScorchConfirm2 => {
                self.owned_planet_popup.mode = OwnedPlanetPopupMode::ScorchConfirm3;
            }
            OwnedPlanetPopupMode::ScorchConfirm3 => {
                let planet_name = self
                    .game_data
                    .planets
                    .records
                    .get(record.saturating_sub(1))
                    .map(PlanetRecord::status_or_name_summary)
                    .unwrap_or_else(|| "Planet".to_string());
                self.game_data.scorch_planet_surface(record)?;
                self.planet_scorch_orders.insert(record);
                self.stage_hosted_planet_scorch(record);
                self.save_and_refresh_runtime()?;
                self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
                self.show_owned_planet_status(format!("Planet \"{planet_name}\" is scorched!"));
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn owned_planet_commission_entries(&self) -> Vec<PlanetCommissionSlotEntry> {
        self.owned_planet_record()
            .map(planet_commission_slot_entries)
            .unwrap_or_default()
    }

    pub(crate) fn show_owned_planet_status(&mut self, message: impl Into<String>) {
        self.owned_planet_popup.mode = OwnedPlanetPopupMode::Browse;
        self.owned_planet_popup.status = Some(message.into());
        self.owned_planet_popup.input.clear();
        self.owned_planet_popup.default.clear();
        self.owned_planet_popup.build_selected_kind = None;
    }

    fn owned_planet_build_max_quantity_for(
        &self,
        kind: ProductionItemKind,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let Some(view) = self.owned_planet_build_view() else {
            return Ok(0);
        };
        planet_build_max_quantity(&self.game_data, &view.row, kind).map_err(Into::into)
    }

    fn owned_planet_can_afford_any_build(&self) -> bool {
        !self
            .owned_planet_build_entries()
            .into_iter()
            .all(|entry| !entry.selectable)
    }

    fn owned_planet_build_unavailable_message(&self, kind: ProductionItemKind) -> String {
        let Some(view) = self.owned_planet_build_view() else {
            return "No build budget available.".to_string();
        };
        planet_build_unavailable_message(view.points_left, kind).to_string()
    }

    fn format_commission_result_line(&self, result: CommissionResult) -> String {
        match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => {
                let fleet_number = self
                    .game_data
                    .fleets
                    .records
                    .get(fleet_record_index_1_based.saturating_sub(1))
                    .map(|fleet| fleet.local_slot_word_raw())
                    .unwrap_or_default();
                format!("Commissioned Fleet #{fleet_number:02}.")
            }
            CommissionResult::Starbase {
                base_record_index_1_based,
            } => {
                let base_number = self
                    .game_data
                    .bases
                    .records
                    .get(base_record_index_1_based.saturating_sub(1))
                    .map(|base| base.local_slot_raw())
                    .unwrap_or_default();
                format!("Commissioned Starbase #{base_number:02}.")
            }
        }
    }

    fn format_auto_commission_report_lines(
        &self,
        report: &nc_data::AutoCommissionReport,
    ) -> Vec<String> {
        let max_fleet_number = self
            .game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == self.player_record_index_1_based)
            .map(|fleet| fleet.local_slot_word_raw())
            .max()
            .unwrap_or(1);
        let mut lines = Vec::new();
        for (idx, entry) in report.entries.iter().enumerate() {
            if idx > 0 {
                lines.push(String::new());
            }
            lines.push(match entry {
                AutoCommissionEntry::Fleet(entry) => {
                    format_auto_commission_fleet_entry(entry, max_fleet_number)
                }
                AutoCommissionEntry::Starbase(entry) => {
                    format_auto_commission_starbase_entry(entry)
                }
            });
        }
        lines
    }
}

fn format_auto_commission_fleet_entry(
    entry: &AutoCommissionFleetEntry,
    max_fleet_number: u16,
) -> String {
    let width = max_fleet_number.max(1).to_string().len().max(2);
    let mut composition = Vec::new();
    push_auto_commission_ship_code(&mut composition, "DD", entry.destroyers);
    push_auto_commission_ship_code(&mut composition, "CA", entry.cruisers);
    push_auto_commission_ship_code(&mut composition, "BB", entry.battleships);
    push_auto_commission_ship_code(&mut composition, "SC", entry.scouts);
    push_auto_commission_ship_code(&mut composition, "TT", entry.transports);
    push_auto_commission_ship_code(&mut composition, "ET", entry.etacs);
    format!(
        "Fleet {:0width$} commissioned from \"{}\" in sector ({:02},{:02}) with {}.",
        entry.fleet_number,
        entry.planet_name,
        entry.coords[0],
        entry.coords[1],
        composition.join(", "),
        width = width,
    )
}

fn format_auto_commission_starbase_entry(entry: &AutoCommissionStarbaseEntry) -> String {
    format!(
        "Starbase {:02} commissioned to \"{}\" in sector ({:02},{:02}).",
        entry.starbase_number, entry.planet_name, entry.coords[0], entry.coords[1]
    )
}

fn push_auto_commission_ship_code(parts: &mut Vec<String>, code: &str, qty: u32) {
    if qty == 0 {
        return;
    }
    parts.push(format!("{code} {qty:02}"));
}
