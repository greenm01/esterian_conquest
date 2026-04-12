pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }
    Err("nc-daemon games is not implemented yet".into())
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-daemon games list");
    println!("  nc-daemon games status [--dir <path>]");
}
