pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }
    Err("nc-daemon settings is not implemented yet".into())
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-daemon settings show --dir <path>");
    println!("  nc-daemon settings set --dir <path> ...");
}
