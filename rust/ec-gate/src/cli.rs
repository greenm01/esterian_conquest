use std::env;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let cmd = args.next();

    match cmd.as_deref() {
        None | Some("--help" | "-h" | "help") => {
            print_usage();
            Ok(())
        }
        Some("--version") => {
            println!("ec-gate {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("init") => {
            eprintln!("ec-gate init: not yet implemented");
            Ok(())
        }
        Some("serve") => {
            eprintln!("ec-gate serve: not yet implemented");
            Ok(())
        }
        Some(other) => Err(format!("unknown command: {other}").into()),
    }
}

fn print_usage() {
    println!(
        "\
ec-gate — Esterian Conquest Nostr auth daemon

Usage:
  ec-gate init                         Generate daemon identity
  ec-gate serve                        Start the auth daemon

Options:
  --version                            Print version
  --help                               Print this help"
    );
}
