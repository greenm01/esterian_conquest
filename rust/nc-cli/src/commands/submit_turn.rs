use std::path::{Path, PathBuf};

use nc_data::{TurnSubmission, TurnSubmissionReport};

use crate::support::parse::parse_usize_1_based;
use crate::support::paths::resolve_repo_path;

pub(crate) fn run_submit_turn_args(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut player_record_index_1_based = None;
    let mut file = None;
    let mut check_only = false;
    let mut remaining = args.into_iter();

    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            "--dir" => {
                let Some(value) = remaining.next() else {
                    return Err("submit-turn requires a path after --dir".into());
                };
                dir = Some(resolve_repo_path(&value));
            }
            "--player" => {
                let Some(value) = remaining.next() else {
                    return Err("submit-turn requires a value after --player".into());
                };
                player_record_index_1_based =
                    Some(parse_usize_1_based(&value, "player record index")?);
            }
            "--file" => {
                let Some(value) = remaining.next() else {
                    return Err("submit-turn requires a path after --file".into());
                };
                file = Some(resolve_repo_path(&value));
            }
            other => return Err(format!("unknown submit-turn argument: {other}").into()),
        }
    }

    let Some(dir) = dir else {
        return Err("submit-turn requires --dir <game_dir>".into());
    };
    let Some(player_record_index_1_based) = player_record_index_1_based else {
        return Err("submit-turn requires --player <record>".into());
    };
    let Some(file) = file else {
        return Err("submit-turn requires --file <turn.kdl>".into());
    };

    submit_turn(&dir, player_record_index_1_based, &file, check_only)
}

pub(crate) fn submit_turn(
    dir: &Path,
    player_record_index_1_based: usize,
    file: &Path,
    check_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = TurnSubmission::submit_kdl_file_to_campaign_dir(
        dir,
        player_record_index_1_based,
        file,
        check_only,
    )?;
    let verb = if check_only { "Validated" } else { "Applied" };
    print_submit_turn_report(verb, file, &report, check_only);
    Ok(())
}

fn print_submit_turn_report(
    verb: &str,
    file: &Path,
    report: &TurnSubmissionReport,
    check_only: bool,
) {
    println!("{verb} turn submission from {}.", display_path(file));
    println!("  player={}", report.player_record_index_1_based);
    println!("  year={}", report.year);
    println!("  tax_changed={}", report.tax_changed);
    println!("  diplomacy_updates={}", report.diplomacy_updates);
    println!("  planet_blocks={}", report.planet_blocks);
    println!("  planet_actions={}", report.planet_actions);
    println!("  fleet_blocks={}", report.fleet_blocks);
    println!("  fleet_actions={}", report.fleet_actions);
    println!("  messages_queued={}", report.messages_queued);
    if check_only {
        println!("  mode=check-only");
    }
}

fn display_path(path: &Path) -> String {
    path.to_str()
        .map(str::to_string)
        .unwrap_or_else(|| PathBuf::from(path).display().to_string())
}
