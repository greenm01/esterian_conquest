use crate::app::action::Action;
use crate::app::state::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppOutcome {
    Continue,
    Quit,
}

pub fn apply_action(app: &mut App, action: Action) -> AppOutcome {
    match action {
        Action::AdvanceStartup => {
            app.advance_startup();
            AppOutcome::Continue
        }
        Action::OpenStartupIntro => {
            app.open_startup_intro();
            AppOutcome::Continue
        }
        Action::OpenFirstTimeMenu => {
            app.open_first_time_menu();
            AppOutcome::Continue
        }
        Action::OpenFirstTimeHelp => {
            app.open_first_time_help();
            AppOutcome::Continue
        }
        Action::OpenFirstTimeEmpires => {
            app.open_first_time_empires();
            AppOutcome::Continue
        }
        Action::OpenFirstTimeIntro => {
            app.open_first_time_intro();
            AppOutcome::Continue
        }
        Action::OpenFirstTimeJoinName => {
            app.open_first_time_join_name();
            AppOutcome::Continue
        }
        Action::ShowAnsiAlwaysOnNotice => {
            app.show_first_time_ansi_notice();
            AppOutcome::Continue
        }
        Action::AppendFirstTimeInputChar(ch) => {
            app.append_first_time_input_char(ch);
            AppOutcome::Continue
        }
        Action::BackspaceFirstTimeInput => {
            app.backspace_first_time_input();
            AppOutcome::Continue
        }
        Action::SubmitFirstTimeInput => {
            app.submit_first_time_input();
            AppOutcome::Continue
        }
        Action::AcceptFirstTimePrompt => {
            app.accept_first_time_prompt();
            AppOutcome::Continue
        }
        Action::RejectFirstTimePrompt => {
            app.reject_first_time_prompt();
            AppOutcome::Continue
        }
        Action::OpenMainMenu => {
            app.open_main_menu();
            AppOutcome::Continue
        }
        Action::OpenGeneralMenu => {
            app.open_general_menu();
            AppOutcome::Continue
        }
        Action::OpenGeneralHelp => {
            *app.current_screen_mut() = crate::screen::ScreenId::GeneralHelp;
            AppOutcome::Continue
        }
        Action::OpenFleetHelp => {
            app.open_fleet_help();
            AppOutcome::Continue
        }
        Action::OpenFleetMenu => {
            app.open_fleet_menu();
            AppOutcome::Continue
        }
        Action::OpenFleetList(mode) => {
            app.open_fleet_list(mode);
            AppOutcome::Continue
        }
        Action::OpenFleetReview => {
            app.open_fleet_review();
            AppOutcome::Continue
        }
        Action::OpenFleetRoeSelect => {
            app.open_fleet_roe_select();
            AppOutcome::Continue
        }
        Action::OpenFleetDetach => {
            app.open_fleet_detach();
            AppOutcome::Continue
        }
        Action::OpenFleetEta => {
            app.open_fleet_eta();
            AppOutcome::Continue
        }
        Action::OpenPlanetMenu => {
            app.open_planet_menu();
            AppOutcome::Continue
        }
        Action::OpenPlanetHelp => {
            app.open_planet_help();
            AppOutcome::Continue
        }
        Action::OpenPlanetAutoCommissionConfirm => {
            app.open_planet_auto_commission_confirm();
            AppOutcome::Continue
        }
        Action::OpenPlanetCommissionMenu => {
            app.open_planet_commission_menu();
            AppOutcome::Continue
        }
        Action::OpenPlanetTransportPlanetSelect(mode) => {
            app.open_planet_transport_planet_select(mode);
            AppOutcome::Continue
        }
        Action::OpenPlanetBuildMenu => {
            app.open_planet_build_menu();
            AppOutcome::Continue
        }
        Action::OpenPlanetBuildHelp => {
            app.open_planet_build_help();
            AppOutcome::Continue
        }
        Action::OpenPlanetBuildReview => {
            app.open_planet_build_review();
            AppOutcome::Continue
        }
        Action::OpenPlanetBuildList => {
            app.open_planet_build_list();
            AppOutcome::Continue
        }
        Action::OpenPlanetBuildChange => {
            app.open_planet_build_change();
            AppOutcome::Continue
        }
        Action::MovePlanetBuildChange(delta) => {
            app.move_planet_build_change_cursor(delta);
            AppOutcome::Continue
        }
        Action::ConfirmPlanetBuildChange => {
            app.confirm_planet_build_change();
            AppOutcome::Continue
        }
        Action::OpenPlanetBuildAbortConfirm => {
            app.open_planet_build_abort_confirm();
            AppOutcome::Continue
        }
        Action::OpenPlanetBuildSpecify => {
            app.open_planet_build_specify();
            AppOutcome::Continue
        }
        Action::OpenPlanetTaxPrompt => {
            app.open_planet_tax_prompt();
            AppOutcome::Continue
        }
        Action::OpenPlanetDatabase => {
            app.open_planet_database();
            AppOutcome::Continue
        }
        Action::OpenPlanetDatabaseDetail => {
            app.open_planet_database_detail();
            AppOutcome::Continue
        }
        Action::ReturnToCommandMenu => {
            app.return_to_command_menu();
            AppOutcome::Continue
        }
        Action::OpenPlanetListSortPrompt(mode) => {
            app.open_planet_list_sort_prompt(mode);
            AppOutcome::Continue
        }
        Action::SubmitPlanetListSort(mode, sort) => {
            app.submit_planet_list_sort(mode, sort);
            AppOutcome::Continue
        }
        Action::OpenEnemies => {
            app.open_enemies();
            AppOutcome::Continue
        }
        Action::OpenDeleteReviewables => {
            app.open_delete_reviewables();
            AppOutcome::Continue
        }
        Action::OpenComposeMessageRecipient => {
            app.open_compose_message_recipient();
            AppOutcome::Continue
        }
        Action::OpenComposeMessageSubject => {
            app.open_compose_message_subject();
            AppOutcome::Continue
        }
        Action::OpenComposeMessageBody => {
            app.open_compose_message_body();
            AppOutcome::Continue
        }
        Action::OpenComposeMessageOutbox => {
            app.open_compose_message_outbox();
            AppOutcome::Continue
        }
        Action::OpenComposeMessageDiscardConfirm => {
            app.open_compose_message_discard_confirm();
            AppOutcome::Continue
        }
        Action::OpenComposeMessageSendConfirm => {
            app.open_compose_message_send_confirm();
            AppOutcome::Continue
        }
        Action::OpenStarmap => {
            app.open_starmap();
            AppOutcome::Continue
        }
        Action::ToggleAutopilot => match app.toggle_autopilot() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::ScrollEnemies(delta) => {
            app.scroll_enemies(delta);
            AppOutcome::Continue
        }
        Action::MoveEnemies(delta) => {
            app.move_enemies_cursor(delta);
            AppOutcome::Continue
        }
        Action::MoveFleetList(delta) => {
            app.move_fleet_list(delta);
            AppOutcome::Continue
        }
        Action::MoveFleetReview(delta) => {
            app.move_fleet_review(delta);
            AppOutcome::Continue
        }
        Action::MoveFleetRoeSelect(delta) => {
            app.move_fleet_roe_select(delta);
            AppOutcome::Continue
        }
        Action::MoveFleetDetachSelect(delta) => {
            app.move_fleet_detach_select(delta);
            AppOutcome::Continue
        }
        Action::MoveFleetEtaSelect(delta) => {
            app.move_fleet_eta_select(delta);
            AppOutcome::Continue
        }
        Action::ScrollPlanetBrief(delta) => {
            app.scroll_planet_brief(delta);
            AppOutcome::Continue
        }
        Action::MovePlanetBrief(delta) => {
            app.move_planet_brief_cursor(delta);
            AppOutcome::Continue
        }
        Action::ScrollPlanetBuildList(delta) => {
            app.scroll_planet_build_list(delta);
            AppOutcome::Continue
        }
        Action::MovePlanetBuildList(delta) => {
            app.move_planet_build_list_cursor(delta);
            AppOutcome::Continue
        }
        Action::DeletePlanetBuildSlotRequest => {
            app.delete_planet_build_slot_request();
            AppOutcome::Continue
        }
        Action::ConfirmDeletePlanetBuildSlot => match app.confirm_delete_planet_build_slot() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::CancelDeletePlanetBuildSlot => {
            app.cancel_delete_planet_build_slot();
            AppOutcome::Continue
        }
        Action::MovePlanetBuild(delta) => {
            app.move_planet_build(delta);
            AppOutcome::Continue
        }
        Action::MovePlanetCommissionPlanet(delta) => {
            app.move_planet_commission_planet(delta);
            AppOutcome::Continue
        }
        Action::MovePlanetCommissionRow(delta) => {
            app.move_planet_commission_row(delta);
            AppOutcome::Continue
        }
        Action::TogglePlanetCommissionSelection => {
            app.toggle_planet_commission_selection();
            AppOutcome::Continue
        }
        Action::CommissionPlanetStardockSelection => match app.commission_selected_stardock_row() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::MovePlanetTransportPlanet(delta) => {
            app.move_planet_transport_planet(delta);
            AppOutcome::Continue
        }
        Action::ConfirmPlanetTransportPlanet => {
            app.confirm_planet_transport_planet();
            AppOutcome::Continue
        }
        Action::MovePlanetTransportFleet(delta) => {
            app.move_planet_transport_fleet(delta);
            AppOutcome::Continue
        }
        Action::ConfirmPlanetTransportFleet => {
            app.confirm_planet_transport_fleet();
            AppOutcome::Continue
        }
        Action::AppendPlanetTransportQtyChar(ch) => {
            app.append_planet_transport_qty_char(ch);
            AppOutcome::Continue
        }
        Action::BackspacePlanetTransportQty => {
            app.backspace_planet_transport_qty();
            AppOutcome::Continue
        }
        Action::SubmitPlanetTransportQty => match app.submit_planet_transport_qty() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::MovePlanetDetail(delta) => {
            app.move_planet_detail(delta);
            AppOutcome::Continue
        }
        Action::MovePlanetDatabaseList(delta) => {
            app.move_planet_database_list(delta);
            AppOutcome::Continue
        }
        Action::MovePlanetDatabaseDetail(delta) => {
            app.move_planet_database_detail(delta);
            AppOutcome::Continue
        }
        Action::AppendPlanetDatabaseChar(ch) => {
            app.append_planet_database_char(ch);
            AppOutcome::Continue
        }
        Action::BackspacePlanetDatabaseInput => {
            app.backspace_planet_database_input();
            AppOutcome::Continue
        }
        Action::SubmitPlanetDatabaseLookup => {
            app.submit_planet_database_lookup();
            AppOutcome::Continue
        }
        Action::AppendPlanetTaxChar(ch) => {
            app.append_planet_tax_char(ch);
            AppOutcome::Continue
        }
        Action::AppendFleetRoeChar(ch) => {
            app.append_fleet_roe_char(ch);
            AppOutcome::Continue
        }
        Action::AppendFleetDetachChar(ch) => {
            app.append_fleet_detach_char(ch);
            AppOutcome::Continue
        }
        Action::BackspaceFleetRoeInput => {
            app.backspace_fleet_roe_input();
            AppOutcome::Continue
        }
        Action::BackspaceFleetDetachInput => {
            app.backspace_fleet_detach_input();
            AppOutcome::Continue
        }
        Action::SubmitFleetRoe => match app.submit_fleet_roe() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::SubmitFleetDetach => match app.submit_fleet_detach() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::AppendFleetEtaChar(ch) => {
            app.append_fleet_eta_char(ch);
            AppOutcome::Continue
        }
        Action::BackspaceFleetEtaInput => {
            app.backspace_fleet_eta_input();
            AppOutcome::Continue
        }
        Action::SubmitFleetEta => {
            app.submit_fleet_eta();
            AppOutcome::Continue
        }
        Action::BackspacePlanetTaxInput => {
            app.backspace_planet_tax_input();
            AppOutcome::Continue
        }
        Action::SubmitPlanetTax => match app.submit_planet_tax() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::AppendPlanetBuildUnitChar(ch) => {
            app.append_planet_build_unit_char(ch);
            AppOutcome::Continue
        }
        Action::BackspacePlanetBuildUnitInput => {
            app.backspace_planet_build_unit_input();
            AppOutcome::Continue
        }
        Action::SubmitPlanetBuildUnit => {
            app.submit_planet_build_unit();
            AppOutcome::Continue
        }
        Action::AppendPlanetBuildQuantityChar(ch) => {
            app.append_planet_build_quantity_char(ch);
            AppOutcome::Continue
        }
        Action::BackspacePlanetBuildQuantityInput => {
            app.backspace_planet_build_quantity_input();
            AppOutcome::Continue
        }
        Action::SubmitPlanetBuildQuantity => match app.submit_planet_build_quantity() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::ConfirmPlanetBuildAbort => match app.abort_current_planet_builds() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::ConfirmPlanetAutoCommission => match app.confirm_planet_auto_commission() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::ScrollComposeRecipients(delta) => {
            app.scroll_compose_recipients(delta);
            AppOutcome::Continue
        }
        Action::MoveComposeRecipient(delta) => {
            app.move_compose_recipient_cursor(delta);
            AppOutcome::Continue
        }
        Action::ScrollComposeOutbox(delta) => {
            app.scroll_compose_outbox(delta);
            AppOutcome::Continue
        }
        Action::MoveComposeOutbox(delta) => {
            app.move_compose_outbox_cursor(delta);
            AppOutcome::Continue
        }
        Action::BeginStarmapDump => {
            app.begin_starmap_dump();
            AppOutcome::Continue
        }
        Action::AdvanceStarmapPage => {
            app.advance_starmap_page();
            AppOutcome::Continue
        }
        Action::ExportStarmap => match app.export_starmap() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::OpenPartialStarmapPrompt(menu) => {
            app.open_partial_starmap_prompt(menu);
            AppOutcome::Continue
        }
        Action::AppendPartialStarmapChar(ch) => {
            app.append_partial_starmap_char(ch);
            AppOutcome::Continue
        }
        Action::BackspacePartialStarmapInput => {
            app.backspace_partial_starmap_input();
            AppOutcome::Continue
        }
        Action::SubmitPartialStarmapPrompt => {
            app.submit_partial_starmap_prompt();
            AppOutcome::Continue
        }
        Action::MovePartialStarmap(dx, dy) => {
            app.move_partial_starmap(dx, dy);
            AppOutcome::Continue
        }
        Action::AppendEnemiesChar(ch) => {
            app.append_enemies_char(ch);
            AppOutcome::Continue
        }
        Action::BackspaceEnemiesInput => {
            app.backspace_enemies_input();
            AppOutcome::Continue
        }
        Action::SubmitEnemiesInput => match app.submit_enemies_input() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::ConfirmDeleteReviewables => match app.delete_reviewables() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::AppendComposeRecipientChar(ch) => {
            app.append_compose_recipient_char(ch);
            AppOutcome::Continue
        }
        Action::BackspaceComposeRecipient => {
            app.backspace_compose_recipient();
            AppOutcome::Continue
        }
        Action::SubmitComposeRecipient => {
            app.submit_compose_recipient();
            AppOutcome::Continue
        }
        Action::AppendComposeSubjectChar(ch) => {
            app.append_compose_subject_char(ch);
            AppOutcome::Continue
        }
        Action::BackspaceComposeSubject => {
            app.backspace_compose_subject();
            AppOutcome::Continue
        }
        Action::SubmitComposeSubject => {
            app.submit_compose_subject();
            AppOutcome::Continue
        }
        Action::AppendComposeBodyChar(ch) => {
            app.append_compose_body_char(ch);
            AppOutcome::Continue
        }
        Action::BackspaceComposeBody => {
            app.backspace_compose_body();
            AppOutcome::Continue
        }
        Action::DeleteComposeBodyChar => {
            app.delete_compose_body_char();
            AppOutcome::Continue
        }
        Action::InsertComposeNewline => {
            app.insert_compose_newline();
            AppOutcome::Continue
        }
        Action::MoveComposeBodyCursorLeft => {
            app.move_compose_body_cursor_left();
            AppOutcome::Continue
        }
        Action::MoveComposeBodyCursorRight => {
            app.move_compose_body_cursor_right();
            AppOutcome::Continue
        }
        Action::MoveComposeBodyCursorUp => {
            app.move_compose_body_cursor_up();
            AppOutcome::Continue
        }
        Action::MoveComposeBodyCursorDown => {
            app.move_compose_body_cursor_down();
            AppOutcome::Continue
        }
        Action::MoveComposeBodyCursorHome => {
            app.move_compose_body_cursor_home();
            AppOutcome::Continue
        }
        Action::MoveComposeBodyCursorEnd => {
            app.move_compose_body_cursor_end();
            AppOutcome::Continue
        }
        Action::SendComposedMessage => match app.send_composed_message() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::AppendComposeOutboxChar(ch) => {
            app.append_compose_outbox_char(ch);
            AppOutcome::Continue
        }
        Action::BackspaceComposeOutboxInput => {
            app.backspace_compose_outbox_input();
            AppOutcome::Continue
        }
        Action::DeleteQueuedComposeMessage => match app.delete_queued_compose_message() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::ConfirmDiscardComposedMessage => {
            app.confirm_discard_composed_message();
            AppOutcome::Continue
        }
        Action::ConfirmSendComposedMessage => match app.send_composed_message() {
            Ok(()) => AppOutcome::Continue,
            Err(_) => AppOutcome::Continue,
        },
        Action::OpenPlanetInfoPrompt(menu) => {
            app.open_planet_info_prompt(menu);
            AppOutcome::Continue
        }
        Action::AppendPlanetInfoChar(ch) => {
            app.append_planet_info_char(ch);
            AppOutcome::Continue
        }
        Action::BackspacePlanetInfoInput => {
            app.backspace_planet_info_input();
            AppOutcome::Continue
        }
        Action::SubmitPlanetInfoPrompt => {
            app.submit_planet_info_prompt();
            AppOutcome::Continue
        }
        Action::OpenEmpireStatus => {
            app.open_empire_status();
            AppOutcome::Continue
        }
        Action::OpenEmpireProfile => {
            app.open_empire_profile();
            AppOutcome::Continue
        }
        Action::OpenRankingsTable(sort) => {
            app.open_rankings_table(sort);
            AppOutcome::Continue
        }
        Action::OpenReports => {
            app.open_reports();
            AppOutcome::Continue
        }
        Action::Quit => AppOutcome::Quit,
        Action::Noop => AppOutcome::Continue,
    }
}
