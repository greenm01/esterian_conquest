use std::path::PathBuf;

use ec_data::{CoreGameData, QueuedPlayerMail, ReportBlockRow};
use ec_game::reports::runtime_inbox_items;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_game_data() -> CoreGameData {
    CoreGameData::load(&repo_root().join("fixtures/ecutil-init/v1.5")).expect("load fixture")
}

fn visible_mail(sender: u8, recipient: u8, year: u16, subject: &str) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id: sender,
        recipient_empire_id: recipient,
        year,
        subject: subject.to_string(),
        body: String::new(),
        recipient_deleted: false,
    }
}

fn report_row(block_index: usize, decoded_text: &str) -> ReportBlockRow {
    ReportBlockRow {
        block_index,
        decoded_text: decoded_text.to_string(),
        raw_bytes: None,
        recipient_deleted: false,
    }
}

#[test]
fn report_subjects_are_event_first_and_zero_padded() {
    let game_data = fixture_game_data();
    let items = runtime_inbox_items(
        &game_data,
        1,
        &[
            report_row(0, "Stardate: 03/3003\nFleet contact report"),
            report_row(1, "Stardate: 11/3003\nBombardment complete."),
            report_row(2, "Stardate: 02/3003\nWe were attacked by enemy forces."),
            report_row(3, "Starbase 1 is moving to (08,12)."),
        ],
        &[],
    );

    let subjects = items
        .into_iter()
        .map(|item| item.subject)
        .collect::<Vec<_>>();
    assert!(subjects.contains(&"Scout - Stardate 03/3003".to_string()));
    assert!(subjects.contains(&"Bombard - Stardate 11/3003".to_string()));
    assert!(subjects.contains(&"Combat - Stardate 02/3003".to_string()));
    assert!(subjects.contains(&format!(
        "Starbase - Stardate 00/{}",
        game_data.conquest.game_year()
    )));
}

#[test]
fn runtime_inbox_items_sort_newest_to_oldest() {
    let game_data = fixture_game_data();
    let items = runtime_inbox_items(
        &game_data,
        1,
        &[
            report_row(0, "Stardate: 02/3003\nFleet contact report"),
            report_row(1, "Stardate: 52/3002\nBombardment complete."),
        ],
        &[visible_mail(2, 1, 3003, "Diplomatic")],
    );

    let labels = items
        .iter()
        .map(|item| format!("{} {}", item.year, item.subject))
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "3003 Scout - Stardate 02/3003".to_string(),
            "3003 Diplomatic".to_string(),
            "3002 Bombard - Stardate 52/3002".to_string(),
        ]
    );
}
