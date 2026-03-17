use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::screen::PlanetTransportMode;

pub fn update(app: &mut App, action: FleetAction) {
    match action {
        FleetAction::OpenMenu => app.open_fleet_menu(),
        FleetAction::OpenHelp => app.open_fleet_help(),
        FleetAction::ShowExpertModeNotice => app.show_fleet_expert_mode_notice(),
        FleetAction::OpenList(mode) => app.open_fleet_list(mode),
        FleetAction::OpenReviewSelect => app.open_fleet_review_select(),
        FleetAction::OpenReview => app.open_fleet_review(),
        FleetAction::OpenRoeSelect => app.open_fleet_roe_select(),
        FleetAction::OpenOrder => app.open_fleet_order(),
        FleetAction::OpenGroupOrder => app.open_fleet_group_order(),
        FleetAction::OpenMissionPicker => app.open_fleet_mission_picker(),
        FleetAction::OpenMerge => app.open_fleet_merge(),
        FleetAction::OpenTransfer => app.open_fleet_transfer(),
        FleetAction::OpenDetach => app.open_fleet_detach(),
        FleetAction::OpenEta => app.open_fleet_eta(),
        FleetAction::OpenTransportLoad => {
            app.open_fleet_transport_planet_select(PlanetTransportMode::Load)
        }
        FleetAction::OpenTransportUnload => {
            app.open_fleet_transport_planet_select(PlanetTransportMode::Unload)
        }
        FleetAction::MoveList(delta) => app.move_fleet_list(delta),
        FleetAction::MoveReviewSelect(delta) => app.move_fleet_review_select(delta),
        FleetAction::MoveReview(delta) => app.move_fleet_review(delta),
        FleetAction::MoveRoeSelect(delta) => app.move_fleet_roe_select(delta),
        FleetAction::MoveOrderSelect(delta) => app.move_fleet_order_select(delta),
        FleetAction::MoveGroupOrder(delta) => app.move_fleet_group_order(delta),
        FleetAction::MoveMissionPicker(delta) => app.move_fleet_mission_picker(delta),
        FleetAction::MoveMergeSelect(delta) => app.move_fleet_merge_select(delta),
        FleetAction::MoveTransferSelect(delta) => app.move_fleet_transfer_select(delta),
        FleetAction::MoveDetachSelect(delta) => app.move_fleet_detach_select(delta),
        FleetAction::MoveEtaSelect(delta) => app.move_fleet_eta_select(delta),
        FleetAction::ToggleGroupOrderSelection => app.toggle_fleet_group_order_selection(),
        FleetAction::ToggleTransferSelection => app.toggle_fleet_transfer_selection(),
        FleetAction::AppendReviewChar(ch) => app.append_fleet_review_char(ch),
        FleetAction::AppendRoeChar(ch) => app.append_fleet_roe_char(ch),
        FleetAction::AppendOrderChar(ch) => app.append_fleet_order_char(ch),
        FleetAction::AppendGroupOrderChar(ch) => app.append_fleet_group_order_char(ch),
        FleetAction::AppendMissionPickerChar(ch) => app.append_fleet_mission_picker_char(ch),
        FleetAction::AppendMergeChar(ch) => app.append_fleet_merge_char(ch),
        FleetAction::AppendTransferChar(ch) => app.append_fleet_transfer_char(ch),
        FleetAction::AppendDetachChar(ch) => app.append_fleet_detach_char(ch),
        FleetAction::AppendEtaChar(ch) => app.append_fleet_eta_char(ch),
        FleetAction::BackspaceReviewInput => app.backspace_fleet_review_input(),
        FleetAction::BackspaceRoeInput => app.backspace_fleet_roe_input(),
        FleetAction::BackspaceOrderInput => app.backspace_fleet_order_input(),
        FleetAction::BackspaceGroupOrderInput => app.backspace_fleet_group_order_input(),
        FleetAction::BackspaceMissionPickerInput => app.backspace_fleet_mission_picker_input(),
        FleetAction::BackspaceMergeInput => app.backspace_fleet_merge_input(),
        FleetAction::BackspaceTransferInput => app.backspace_fleet_transfer_input(),
        FleetAction::BackspaceDetachInput => app.backspace_fleet_detach_input(),
        FleetAction::BackspaceEtaInput => app.backspace_fleet_eta_input(),
        FleetAction::SubmitReviewSelect => app.submit_fleet_review_select(),
        FleetAction::SubmitRoe => {
            if let Err(err) = app.submit_fleet_roe() {
                eprintln!("submit fleet roe failed: {err}");
            }
        }
        FleetAction::SubmitOrder => {
            if let Err(err) = app.submit_fleet_order() {
                eprintln!("submit fleet order failed: {err}");
            }
        }
        FleetAction::SubmitGroupOrder => app.submit_fleet_group_order(),
        FleetAction::SubmitMissionPicker => app.submit_fleet_mission_picker(),
        FleetAction::SubmitMerge => {
            if let Err(err) = app.submit_fleet_merge() {
                eprintln!("submit fleet merge failed: {err}");
            }
        }
        FleetAction::SubmitTransfer => {
            if let Err(err) = app.submit_fleet_transfer() {
                eprintln!("submit fleet transfer failed: {err}");
            }
        }
        FleetAction::SubmitDetach => {
            if let Err(err) = app.submit_fleet_detach() {
                eprintln!("submit fleet detach failed: {err}");
            }
        }
        FleetAction::SubmitEta => app.submit_fleet_eta(),
    }
}
