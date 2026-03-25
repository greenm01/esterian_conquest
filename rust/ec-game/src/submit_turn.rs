use std::path::{Path, PathBuf};

use ec_data::{TurnSubmission, TurnSubmissionReport};

pub fn run_submit_turn_args(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut player_record_index_1_based = None;
    let mut file = None;
    let mut check_only = false;
    let mut idx = 0;

    while idx < args.len() {
        match args[idx].as_str() {
            "--check" => {
                check_only = true;
                idx += 1;
            }
            "--dir" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("submit-turn requires a path after --dir".into());
                };
                dir = Some(PathBuf::from(value));
                idx += 2;
            }
            "--player" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("submit-turn requires a value after --player".into());
                };
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| format!("player record index must be >= 1, got '{value}'"))?;
                if parsed == 0 {
                    return Err("player record index must be >= 1".into());
                }
                player_record_index_1_based = Some(parsed);
                idx += 2;
            }
            "--file" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("submit-turn requires a path after --file".into());
                };
                file = Some(PathBuf::from(value));
                idx += 2;
            }
            "--help" | "-h" => {
                print_submit_turn_usage();
                return Ok(());
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

    let report = TurnSubmission::submit_kdl_file_to_campaign_dir(
        &dir,
        player_record_index_1_based,
        &file,
        check_only,
    )?;
    let verb = if check_only { "Validated" } else { "Applied" };
    print_submit_turn_report(verb, &file, &report, check_only);
    Ok(())
}

pub fn print_submit_turn_usage() {
    println!("Usage:");
    println!(
        "  ec-game submit-turn [--check] --dir <game_dir> --player <record> --file <turn.kdl>"
    );
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
