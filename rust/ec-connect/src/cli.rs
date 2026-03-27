use std::env;

use crate::identity::{
    cmd_id_import, cmd_id_list, cmd_id_new, cmd_id_secret, cmd_id_show, cmd_id_switch,
};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let cmd = args.next();

    match cmd.as_deref() {
        None => {
            // Picker mode: show ratatui game list
            eprintln!("ec-connect: picker mode not yet implemented");
            Ok(())
        }
        Some("--help" | "-h" | "help") => {
            print_usage();
            Ok(())
        }
        Some("--version") => {
            println!("ec-connect {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("id") => {
            let sub = args.next();
            match sub.as_deref() {
                None => cmd_id_show(),
                Some("--secret") => cmd_id_secret(),
                Some("list") => cmd_id_list(),
                Some("new") => cmd_id_new(),
                Some("import") => cmd_id_import(),
                Some("switch") => {
                    let n = args.next().ok_or("usage: ec-connect id switch <N>")?;
                    cmd_id_switch(&n)
                }
                Some(other) => Err(format!("unknown id subcommand: {other}").into()),
            }
        }
        Some("--join") => {
            let code = args.next().ok_or("--join requires an invite code")?;
            eprintln!("ec-connect --join {code}: not yet implemented");
            Ok(())
        }
        Some(server) => {
            // Direct mode: connect to server bookmark or hostname
            eprintln!("ec-connect {server}: direct mode not yet implemented");
            Ok(())
        }
    }
}

fn print_usage() {
    println!(
        "\
ec-connect — Esterian Conquest multiplayer client

Usage:
  ec-connect                           Picker mode (game list)
  ec-connect <SERVER>                  Direct mode (connect to server)
  ec-connect --join <INVITE-CODE>      Join a new game

Identity:
  ec-connect id                        Show active identity (npub)
  ec-connect id --secret               Show npub + nsec (for backup)
  ec-connect id list                   List all wallet identities
  ec-connect id new                    Generate a new keypair
  ec-connect id import                 Import an existing nsec
  ec-connect id switch <N>             Switch active identity

Options:
  --version                            Print version
  --help                               Print this help"
    );
}
