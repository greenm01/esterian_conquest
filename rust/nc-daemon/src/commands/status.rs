use std::path::PathBuf;

use crate::config::daemon_config::DaemonConfig;
use crate::status::{collect, render};

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut config_path = None;
    let mut games_root = None;
    let mut json = false;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--config" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --config".into());
                }
                config_path = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            "--root" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --root".into());
                }
                games_root = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            "--json" => {
                json = true;
                i += 1;
            }
            other => return Err(format!("unknown argument: {}", other).into()),
        }
    }

    let config_path = config_path
        .or_else(|| std::env::var("NC_DAEMON_CONFIG").ok().map(PathBuf::from))
        .unwrap_or_else(DaemonConfig::default_config_path);
    let mut config = DaemonConfig::load(&config_path)?;
    if let Some(root) = games_root {
        config.games_root = root;
    }

    let report = collect::collect_status(&config, Some(config_path.as_path()))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render::render_human(&report));
    }

    Ok(())
}

fn print_usage() {
    println!("Usage: nc-daemon status [--config <path>] [--root <path>] [--json]");
}
