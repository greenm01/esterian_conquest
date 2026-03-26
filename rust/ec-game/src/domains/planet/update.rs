use crate::app::state::App;
use crate::domains::planet::PlanetAction;

pub fn update(app: &mut App, action: PlanetAction) {
    match action {
        PlanetAction::OpenMenu => app.open_planet_menu(),
        PlanetAction::OpenHelp => app.open_planet_help(),
        PlanetAction::OpenAutoCommissionPrompt => app.open_planet_auto_commission_prompt(),
        PlanetAction::CloseAutoCommissionPrompt => app.close_planet_auto_commission_prompt(),
        PlanetAction::AdvanceAutoCommissionReport => app.advance_planet_auto_commission_report(),
        PlanetAction::OpenScorchPrompt => app.open_planet_scorch_prompt(),
        PlanetAction::SubmitScorchPrompt => {
            if let Err(err) = app.submit_planet_scorch_prompt() {
                app.planet.scorch_prompt_status = Some(err);
            }
        }
        PlanetAction::AppendScorchPromptChar(ch) => app.append_planet_scorch_prompt_char(ch),
        PlanetAction::BackspaceScorchPromptInput => app.backspace_planet_scorch_prompt_input(),
        PlanetAction::CancelScorchPrompt => app.cancel_planet_scorch_prompt(),
        PlanetAction::OpenCommissionMenu => app.open_planet_commission_menu(),
        PlanetAction::OpenCommissionPlanet => app.open_planet_commission_planet(),
        PlanetAction::CloseCommissionPlanet => app.close_planet_commission_planet(),
        PlanetAction::MoveCommissionDraftRow(delta) => {
            if let Err(err) = app.move_planet_commission_draft_row(delta) {
                eprintln!("move commission draft row failed: {err}");
            }
        }
        PlanetAction::AppendCommissionDraftChar(ch) => app.append_planet_commission_draft_char(ch),
        PlanetAction::BackspaceCommissionDraftInput => {
            app.backspace_planet_commission_draft_input()
        }
        PlanetAction::SubmitCommissionDraft => {
            if let Err(err) = app.submit_planet_commission_draft() {
                eprintln!("submit commission draft failed: {err}");
            }
        }
        PlanetAction::CloseCommissionDraft => app.close_planet_commission_draft(),
        PlanetAction::DismissCommissionResult(key_code) => {
            app.dismiss_planet_commission_result(key_code)
        }
        PlanetAction::ClearCommissionDismissKey => app.clear_planet_commission_dismiss_key(),
        PlanetAction::OpenTransportPlanetSelect(mode) => app.open_planet_transport_prompt(mode),
        PlanetAction::SubmitTransportPrompt => app.submit_planet_transport_prompt(),
        PlanetAction::AppendTransportPromptChar(ch) => app.append_planet_transport_prompt_char(ch),
        PlanetAction::BackspaceTransportPromptInput => {
            app.backspace_planet_transport_prompt_input()
        }
        PlanetAction::CancelTransportPrompt => app.cancel_planet_transport_prompt(),
        PlanetAction::OpenBuildMenu => app.open_planet_build_menu(),
        PlanetAction::OpenBuildHelp => app.open_planet_build_help(),
        PlanetAction::OpenCurrentBuildPlanetInfo => app.open_current_build_planet_info(),
        PlanetAction::OpenBuildList => app.open_planet_build_list(),
        PlanetAction::OpenBuildChange => app.open_planet_build_change(),
        PlanetAction::MoveBuildChange(delta) => app.move_planet_build_change_cursor(delta),
        PlanetAction::ConfirmBuildChange => app.confirm_planet_build_change(),
        PlanetAction::OpenBuildAbortPrompt => app.open_planet_build_abort_prompt(),
        PlanetAction::CloseBuildAbortPrompt => app.close_planet_build_abort_prompt(),
        PlanetAction::OpenBuildSpecify => app.open_planet_build_specify(),
        PlanetAction::OpenListSortPrompt(mode) => app.open_planet_list_sort_prompt(mode),
        PlanetAction::SubmitListSort(mode, sort) => app.submit_planet_list_sort(mode, sort),
        PlanetAction::CloseListSortPrompt(mode) => app.close_planet_list_sort_prompt(mode),
        PlanetAction::OpenTaxPrompt => app.open_planet_tax_prompt(),
        PlanetAction::CloseTaxPrompt => app.close_planet_tax_prompt(),
        PlanetAction::OpenDatabase => app.open_planet_database(),
        PlanetAction::OpenDatabaseFilterPrompt => app.open_planet_database_filter_prompt(),
        PlanetAction::SubmitDatabaseFilter(mode) => app.submit_planet_database_filter(mode),
        PlanetAction::ScrollBrief(delta) => app.scroll_planet_brief(delta),
        PlanetAction::MoveBrief(delta) => app.move_planet_brief_cursor(delta),
        PlanetAction::AppendBriefChar(ch) => app.append_planet_brief_char(ch),
        PlanetAction::BackspaceBriefInput => app.backspace_planet_brief_input(),
        PlanetAction::SubmitBriefInput => app.submit_planet_brief_input(),
        PlanetAction::ScrollBuildList(delta) => app.scroll_planet_build_list(delta),
        PlanetAction::MoveBuildList(delta) => app.move_planet_build_list_cursor(delta),
        PlanetAction::DeleteBuildSlotRequest => app.delete_planet_build_slot_request(),
        PlanetAction::AppendDeleteBuildQtyChar(ch) => app.append_delete_build_qty_char(ch),
        PlanetAction::BackspaceDeleteBuildQtyInput => app.backspace_delete_build_qty_input(),
        PlanetAction::SubmitDeleteBuildQty => {
            if let Err(err) = app.submit_delete_build_qty() {
                eprintln!("submit delete planet build quantity failed: {err}");
            }
        }
        PlanetAction::ConfirmDeleteBuildSlot => {
            if let Err(err) = app.confirm_delete_planet_build_slot() {
                eprintln!("confirm delete planet build slot failed: {err}");
            }
        }
        PlanetAction::CancelDeleteBuildSlot => app.cancel_delete_planet_build_slot(),
        PlanetAction::MoveBuild(delta) => app.move_planet_build(delta),
        PlanetAction::MoveCommissionPlanet(delta) => app.move_planet_commission_planet(delta),
        PlanetAction::MoveCommissionRow(delta) => app.move_planet_commission_row(delta),
        PlanetAction::ToggleCommissionSelection => app.toggle_planet_commission_selection(),
        PlanetAction::CommissionStardockSelection => {
            if let Err(err) = app.commission_selected_stardock_row() {
                eprintln!("commission stardock selection failed: {err}");
            }
        }
        PlanetAction::MoveTransportPlanet(delta) => app.move_planet_transport_planet(delta),
        PlanetAction::ConfirmTransportPlanet => app.confirm_planet_transport_planet(),
        PlanetAction::AppendTransportPlanetChar(ch) => app.append_planet_transport_planet_char(ch),
        PlanetAction::BackspaceTransportPlanetInput => {
            app.backspace_planet_transport_planet_input()
        }
        PlanetAction::SubmitTransportPlanet => app.submit_planet_transport_planet(),
        PlanetAction::MoveTransportFleet(delta) => app.move_planet_transport_fleet(delta),
        PlanetAction::ConfirmTransportFleet => app.confirm_planet_transport_fleet(),
        PlanetAction::AppendTransportQtyChar(ch) => app.append_planet_transport_qty_char(ch),
        PlanetAction::BackspaceTransportQty => app.backspace_planet_transport_qty(),
        PlanetAction::SubmitTransportQty => {
            if let Err(err) = app.submit_planet_transport_qty() {
                eprintln!("submit planet transport qty failed: {err}");
            }
        }
        PlanetAction::MoveDatabaseList(delta) => app.move_planet_database_list(delta),
        PlanetAction::AppendDatabaseChar(ch) => app.append_planet_database_char(ch),
        PlanetAction::BackspaceDatabaseInput => app.backspace_planet_database_input(),
        PlanetAction::SubmitDatabaseLookup => app.submit_planet_database_lookup(),
        PlanetAction::AppendTaxChar(ch) => app.append_planet_tax_char(ch),
        PlanetAction::BackspaceTaxInput => app.backspace_planet_tax_input(),
        PlanetAction::SubmitTax => {
            if let Err(err) = app.submit_planet_tax() {
                eprintln!("submit planet tax failed: {err}");
            }
        }
        PlanetAction::AppendBuildUnitChar(ch) => app.append_planet_build_unit_char(ch),
        PlanetAction::BackspaceBuildUnitInput => app.backspace_planet_build_unit_input(),
        PlanetAction::SubmitBuildUnit => app.submit_planet_build_unit(),
        PlanetAction::AppendBuildQuantityChar(ch) => app.append_planet_build_quantity_char(ch),
        PlanetAction::BackspaceBuildQuantityInput => app.backspace_planet_build_quantity_input(),
        PlanetAction::SubmitBuildQuantity => {
            if let Err(err) = app.submit_planet_build_quantity() {
                eprintln!("submit planet build quantity failed: {err}");
            }
        }
        PlanetAction::ConfirmBuildAbort => {
            if let Err(err) = app.abort_current_planet_builds() {
                eprintln!("confirm planet build abort failed: {err}");
            }
        }
        PlanetAction::ConfirmAutoCommission => {
            if let Err(err) = app.confirm_planet_auto_commission() {
                eprintln!("confirm planet auto commission failed: {err}");
            }
        }
        PlanetAction::OpenInfoPrompt(menu) => app.open_planet_info_prompt(menu),
        PlanetAction::CloseInfoPrompt => app.close_planet_info_prompt(),
        PlanetAction::AppendInfoChar(ch) => app.append_planet_info_char(ch),
        PlanetAction::BackspaceInfoInput => app.backspace_planet_info_input(),
        PlanetAction::SubmitInfoPrompt => app.submit_planet_info_prompt(),
    }
}
