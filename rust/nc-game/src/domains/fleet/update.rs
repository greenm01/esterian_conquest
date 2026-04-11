use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::screen::PlanetTransportMode;

pub fn update(app: &mut App, action: FleetAction) {
    match action {
        FleetAction::OpenMenu => app.open_fleet_menu(),
        FleetAction::OpenList => app.open_fleet_list(),
        FleetAction::OpenReviewPrompt => app.open_fleet_review_prompt(),
        FleetAction::OpenReview => app.open_fleet_review(),
        FleetAction::CloseReview => app.close_fleet_review(),
        FleetAction::OpenListFilterPrompt => app.open_fleet_list_filter_prompt(),
        FleetAction::DismissListFilterPromptNotice => app.dismiss_fleet_list_filter_prompt_notice(),
        FleetAction::OpenListSortPrompt => app.open_fleet_list_sort_prompt(),
        FleetAction::OpenChangePrompt => app.open_fleet_change_prompt(),
        FleetAction::OpenOrder => app.open_fleet_order(),
        FleetAction::OpenGroupOrder => app.open_fleet_group_order(),
        FleetAction::OpenMissionPicker => app.open_fleet_mission_picker(),
        FleetAction::OpenMerge => app.open_fleet_merge(),
        FleetAction::OpenTransfer => app.open_fleet_transfer(),
        FleetAction::OpenDetach => app.open_fleet_detach(),
        FleetAction::OpenEta => app.open_fleet_eta(),
        FleetAction::CloseEta => app.close_fleet_eta(),
        FleetAction::OpenTransportLoad => {
            app.open_fleet_transport_prompt(PlanetTransportMode::Load)
        }
        FleetAction::OpenTransportUnload => {
            app.open_fleet_transport_prompt(PlanetTransportMode::Unload)
        }
        FleetAction::DismissMessage => app.dismiss_fleet_message(),
        FleetAction::MoveList(delta) => app.move_fleet_list(delta),
        FleetAction::MoveReview(delta) => app.move_fleet_review(delta),
        FleetAction::MoveGroupOrder(delta) => app.move_fleet_group_order(delta),
        FleetAction::MoveMissionPicker(delta) => app.move_fleet_mission_picker(delta),
        FleetAction::ToggleGroupOrderSelection => app.toggle_fleet_group_order_selection(),
        FleetAction::AppendListChar(ch) => app.append_fleet_list_char(ch),
        FleetAction::AppendListFilterPromptChar(ch) => app.append_fleet_list_filter_prompt_char(ch),
        FleetAction::AppendMenuPromptChar(ch) => app.append_fleet_menu_prompt_char(ch),
        FleetAction::AppendOrderChar(ch) => app.append_fleet_order_char(ch),
        FleetAction::AppendGroupOrderChar(ch) => app.append_fleet_group_order_char(ch),
        FleetAction::AppendMissionPickerChar(ch) => app.append_fleet_mission_picker_char(ch),
        FleetAction::AppendTransferChar(ch) => app.append_fleet_transfer_char(ch),
        FleetAction::AppendDetachChar(ch) => app.append_fleet_detach_char(ch),
        FleetAction::AppendEtaChar(ch) => app.append_fleet_eta_char(ch),
        FleetAction::BackspaceListInput => app.backspace_fleet_list_input(),
        FleetAction::BackspaceListFilterPromptInput => app.backspace_fleet_list_filter_prompt_input(),
        FleetAction::BackspaceMenuPromptInput => app.backspace_fleet_menu_prompt_input(),
        FleetAction::BackspaceOrderInput => app.backspace_fleet_order_input(),
        FleetAction::BackspaceGroupOrderInput => app.backspace_fleet_group_order_input(),
        FleetAction::BackspaceMissionPickerInput => app.backspace_fleet_mission_picker_input(),
        FleetAction::BackspaceTransferInput => app.backspace_fleet_transfer_input(),
        FleetAction::BackspaceDetachInput => app.backspace_fleet_detach_input(),
        FleetAction::BackspaceEtaInput => app.backspace_fleet_eta_input(),
        FleetAction::CancelMenuPrompt => app.cancel_fleet_menu_prompt(),
        FleetAction::CancelTransfer => app.cancel_fleet_transfer(),
        FleetAction::CancelDetach => app.cancel_fleet_detach(),
        FleetAction::CloseListPrompt => app.close_fleet_list_prompt(),
        FleetAction::ClearTransferSelection => app.clear_fleet_transfer_selection(),
        FleetAction::ClearDetachSelection => app.clear_fleet_detach_selection(),
        FleetAction::CancelOrder => app.cancel_fleet_order(),
        FleetAction::CancelGroupOrder => app.cancel_fleet_group_order(),
        FleetAction::SubmitListFilter(filter) => app.submit_fleet_list_filter(filter),
        FleetAction::SubmitListFilterPrompt => app.submit_fleet_list_filter_prompt(),
        FleetAction::SubmitListSort(sort) => app.submit_fleet_list_sort(sort),
        FleetAction::SubmitListSortPrompt => app.submit_fleet_list_sort_prompt(),
        FleetAction::SubmitMenuPrompt => app.submit_fleet_menu_prompt(),
        FleetAction::SubmitOrder => {
            if let Err(err) = app.submit_fleet_order() {
                app.log_action_error("submit_fleet_order", err.as_ref());
                app.fleet.order_status = Some(
                    "Unable to save this fleet order right now. Please try again.".to_string(),
                );
            }
        }
        FleetAction::SubmitGroupOrder => app.submit_fleet_group_order(),
        FleetAction::SubmitMissionPicker => app.submit_fleet_mission_picker(),
        FleetAction::SubmitTransfer => {
            if let Err(err) = app.submit_fleet_transfer() {
                app.log_action_error("submit_fleet_transfer", err.as_ref());
                app.fleet.transfer_status = Some(
                    "Unable to save this fleet transfer right now. Please try again.".to_string(),
                );
            }
        }
        FleetAction::SubmitDetach => {
            if let Err(err) = app.submit_fleet_detach() {
                app.log_action_error("submit_fleet_detach", err.as_ref());
                app.fleet.detach_status = Some(
                    "Unable to save this fleet detach right now. Please try again.".to_string(),
                );
            }
        }
        FleetAction::SubmitEta => app.submit_fleet_eta(),
    }
}
