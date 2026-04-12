pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }
    Err("nc-daemon new-game is not implemented yet".into())
}

fn print_usage() {
    println!("Usage: nc-daemon new-game <dir> [--players N] [--name \"Name\"] [--seed N]");
}
