use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::screen::PlanetTransportMode;

pub fn update(app: &mut App, action: FleetAction) {
    match action {
        FleetAction::OpenMenu => app.open_fleet_menu(),
        FleetAction::OpenHelp => app.open_fleet_help(),
        FleetAction::OpenList => app.open_fleet_list(),
        FleetAction::OpenReviewPrompt => app.open_fleet_review_prompt(),
        FleetAction::OpenReview => app.open_fleet_review(),
        FleetAction::CloseReview => app.close_fleet_review(),
        FleetAction::OpenChangePrompt => app.open_fleet_change_prompt(),
        FleetAction::OpenOrder => app.open_fleet_order(),
        FleetAction::OpenGroupOrder => app.open_fleet_group_order(),
        FleetAction::OpenMissionPicker => app.open_fleet_mission_picker(),
        FleetAction::OpenMerge => app.open_fleet_merge(),
        FleetAction::OpenTransfer => app.open_fleet_transfer(),
        FleetAction::OpenDetach => app.open_fleet_detach(),
        FleetAction::OpenEta => app.open_fleet_eta(),
        FleetAction::OpenTransportLoad => app.open_fleet_transport_prompt(PlanetTransportMode::Load),
        FleetAction::OpenTransportUnload => {
            app.open_fleet_transport_prompt(PlanetTransportMode::Unload)
        }
        FleetAction::MoveList(delta) => app.move_fleet_list(delta),
        FleetAction::MoveReview(delta) => app.move_fleet_review(delta),
        FleetAction::MoveGroupOrder(delta) => app.move_fleet_group_order(delta),
        FleetAction::MoveMissionPicker(delta) => app.move_fleet_mission_picker(delta),
        FleetAction::ToggleGroupOrderSelection => app.toggle_fleet_group_order_selection(),
        FleetAction::AppendListChar(ch) => app.append_fleet_list_char(ch),
        FleetAction::AppendMenuPromptChar(ch) => app.append_fleet_menu_prompt_char(ch),
        FleetAction::AppendOrderChar(ch) => app.append_fleet_order_char(ch),
        FleetAction::AppendGroupOrderChar(ch) => app.append_fleet_group_order_char(ch),
        FleetAction::AppendMissionPickerChar(ch) => app.append_fleet_mission_picker_char(ch),
        FleetAction::AppendTransferChar(ch) => app.append_fleet_transfer_char(ch),
        FleetAction::AppendDetachChar(ch) => app.append_fleet_detach_char(ch),
        FleetAction::AppendEtaChar(ch) => app.append_fleet_eta_char(ch),
        FleetAction::BackspaceListInput => app.backspace_fleet_list_input(),
        FleetAction::BackspaceMenuPromptInput => app.backspace_fleet_menu_prompt_input(),
        FleetAction::BackspaceOrderInput => app.backspace_fleet_order_input(),
        FleetAction::BackspaceGroupOrderInput => app.backspace_fleet_group_order_input(),
        FleetAction::BackspaceMissionPickerInput => app.backspace_fleet_mission_picker_input(),
        FleetAction::BackspaceTransferInput => app.backspace_fleet_transfer_input(),
        FleetAction::BackspaceDetachInput => app.backspace_fleet_detach_input(),
        FleetAction::BackspaceEtaInput => app.backspace_fleet_eta_input(),
        FleetAction::CancelMenuPrompt => app.cancel_fleet_menu_prompt(),
        FleetAction::CancelOrder => app.cancel_fleet_order(),
        FleetAction::CancelGroupOrder => app.cancel_fleet_group_order(),
        FleetAction::SubmitMenuPrompt => app.submit_fleet_menu_prompt(),
        FleetAction::SubmitOrder => {
            if let Err(err) = app.submit_fleet_order() {
                eprintln!("submit fleet order failed: {err}");
            }
        }
        FleetAction::SubmitGroupOrder => app.submit_fleet_group_order(),
        FleetAction::SubmitMissionPicker => app.submit_fleet_mission_picker(),
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
