use std::path::PathBuf;

use crate::config::host_identity;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    match args[0] {
        "init" => run_init(&args[1..]),
        _ => Err(format!("unknown nostr subcommand: {}", args[0]).into()),
    }
}

fn run_init(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let mut identity_path = PathBuf::from("/etc/nc-host/host.nsec");

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--path" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --path".into());
                }
                identity_path = PathBuf::from(args[i + 1]);
                i += 2;
            }
            _ => return Err(format!("unknown flag: {}", args[i]).into()),
        }
    }

    let identity = host_identity::generate_identity()?;
    host_identity::save_identity(&identity_path, &identity)?;

    println!("Generated host identity:");
    println!("  npub: {}", identity.npub);
    println!(
        "  nsec: {} (saved to {})",
        identity.nsec,
        identity_path.display()
    );

    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-host nostr init [--path <path>]");
    println!();
    println!("Options:");
    println!("  --path <path>  Path to save identity (default: /etc/nc-host/host.nsec)");
}
