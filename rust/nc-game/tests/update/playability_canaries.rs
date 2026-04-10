use crate::support::*;
use nc_game::reports::ReportsPreview;

#[test]
fn structured_combat_report_round_trips_through_reports_screen_and_delete() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    state.queued_mail.clear();
    set_runtime_report_blocks(
        &mut state,
        length_prefixed_report_block(&[
            "From your 3rd Fleet, located in System(9,5):",
            "Bombardment report",
            "",
            "Target world: planet \"red\"",
            "Our forces: 2BB, 4CA, 6DD",
            "World defenses: 10 ground batteries and 34 armies",
            "",
            "We have concluded our bombing run and are awaiting new orders.",
            "Enemy losses: 10 ground batteries",
            "<end of transmission>",
        ]),
    );
    state.game_data.player.records[0].raw[0x30] = 0;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let runtime = latest_runtime_state(&fixture_dir);
    let preview = ReportsPreview::from_block_rows(
        &runtime.game_data,
        1,
        &runtime.report_block_rows,
        &runtime.queued_mail,
    );
    assert_eq!(preview.result_blocks.len(), 1);
    assert!(
        preview
            .results_lines
            .iter()
            .any(|line| line.contains("Bombardment report"))
    );
    assert!(
        preview
            .results_lines
            .iter()
            .any(|line| line.contains("Target world: planet \"red\""))
    );
    assert!(preview.results_lines.iter().any(|line| line.is_empty()));

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SetInboxTypeFilterReports)
        ),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reports screen should render structured combat preview");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Type: Reports"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Bombardment report"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Target world: planet \"red\""))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Enemy losses: 10 ground batteries"))
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenInboxDeleteConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDeleteInboxItem)
        ),
        AppOutcome::Continue
    );

    let reloaded = latest_runtime_state(&fixture_dir);
    assert!(
        reloaded.report_block_rows.is_empty()
            || reloaded
                .report_block_rows
                .iter()
                .all(|row| row.recipient_deleted)
    );

    let refreshed = ReportsPreview::from_block_rows(
        &reloaded.game_data,
        1,
        &reloaded.report_block_rows,
        &reloaded.queued_mail,
    );
    assert!(refreshed.result_blocks.is_empty());
}
