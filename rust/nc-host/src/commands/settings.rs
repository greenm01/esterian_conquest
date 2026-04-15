use nc_data::hosted::{
    GameTier, HostedStore, LobbyVisibility, RecruitingMode, get_settings, mark_catalog_dirty,
    update_settings,
};
use std::path::PathBuf;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut game_dir = None;
    let mut subcmd = None;
    let mut recruiting: Option<String> = None;
    let mut lobby_visibility: Option<String> = None;
    let mut host_alias: Option<String> = None;
    let mut summary: Option<String> = None;
    let mut tier: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--dir" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --dir".into());
                }
                game_dir = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            "--recruiting" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --recruiting".into());
                }
                recruiting = Some(args[i + 1].to_string());
                i += 2;
            }
            "--lobby" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --lobby".into());
                }
                lobby_visibility = Some(args[i + 1].to_string());
                i += 2;
            }
            "--host-alias" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --host-alias".into());
                }
                host_alias = Some(args[i + 1].to_string());
                i += 2;
            }
            "--summary" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --summary".into());
                }
                summary = Some(args[i + 1].to_string());
                i += 2;
            }
            "--tier" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --tier".into());
                }
                tier = Some(args[i + 1].to_string());
                i += 2;
            }
            _ => {
                if subcmd.is_none() {
                    subcmd = Some(args[i]);
                } else {
                    return Err(format!("unexpected argument: {}", args[i]).into());
                }
                i += 1;
            }
        }
    }

    let game_dir = game_dir.ok_or("missing --dir argument")?;
    let db_path = game_dir.join("hosted.db");
    let store = HostedStore::open(&db_path)?;

    let game_id = game_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid game directory name")?;

    match subcmd {
        Some("show") => run_show(&store, game_id),
        Some("set") => run_set(
            &store,
            game_id,
            recruiting,
            lobby_visibility,
            host_alias,
            summary,
            tier,
        ),
        Some(cmd) => Err(format!("unknown settings subcommand: {}", cmd).into()),
        None => {
            print_usage();
            Ok(())
        }
    }
}

fn run_show(store: &HostedStore, game_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let settings = get_settings(store.connection(), game_id)?;

    println!("Settings for game '{}':", game_id);
    println!();
    println!("  Tier:             {}", settings.game_tier.as_str());
    println!("  Lobby visibility: {}", settings.lobby_visibility.as_str());
    println!("  Recruiting:       {}", settings.recruiting.as_str());
    println!(
        "  Maintenance:     {}",
        if settings.maintenance_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    if let Some(alias) = settings.host_alias {
        println!("  Host alias:      {}", alias);
    }
    if let Some(summary) = settings.summary {
        println!("  Summary:         {}", summary);
    }
    println!(
        "  Interval:         {} minutes",
        settings.maintenance_interval_minutes
    );

    Ok(())
}

fn run_set(
    store: &HostedStore,
    game_id: &str,
    recruiting: Option<String>,
    lobby_visibility: Option<String>,
    host_alias: Option<String>,
    summary: Option<String>,
    tier: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = get_settings(store.connection(), game_id)?;

    let mut dirty = false;

    if let Some(r) = recruiting {
        settings.recruiting = RecruitingMode::from_str(&r).ok_or("invalid recruiting mode")?;
        dirty = true;
    }

    if let Some(v) = lobby_visibility {
        settings.lobby_visibility =
            LobbyVisibility::from_str(&v).ok_or("invalid lobby visibility")?;
        dirty = true;
    }

    if host_alias.is_some() {
        settings.host_alias = host_alias;
        dirty = true;
    }

    if summary.is_some() {
        settings.summary = summary;
        dirty = true;
    }

    if let Some(t) = tier {
        settings.game_tier = GameTier::from_str(&t).ok_or("invalid tier: use 'sandbox' or 'league'")?;
        dirty = true;
    }

    update_settings(store.connection(), game_id, &settings)?;

    if dirty {
        mark_catalog_dirty(store.connection(), game_id)?;
        println!("Settings updated. Catalog will be republished on next cycle.");
    } else {
        println!("Settings updated.");
    }

    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-host settings show --dir <path>");
    println!("  nc-host settings set --dir <path> [options]");
    println!();
    println!("Options:");
    println!("  --recruiting <mode>   none|new_players|replacement_players");
    println!("  --lobby <visibility>  public|private");
    println!("  --tier <tier>         sandbox|league");
    println!("  --host-alias <name>   Host display name");
    println!("  --summary <text>      Lobby description");
}
