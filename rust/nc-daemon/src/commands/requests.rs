pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }
    Err("nc-daemon requests is not implemented yet".into())
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-daemon requests list [--dir <path>]");
    println!("  nc-daemon requests show --dir <path> --request <id>");
    println!("  nc-daemon requests approve --dir <path> --request <id> --player N");
    println!("  nc-daemon requests reject --dir <path> --request <id> --message \"...\"");
}
