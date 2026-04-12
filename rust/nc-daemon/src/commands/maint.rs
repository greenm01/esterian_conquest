pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }
    Err("nc-daemon maint is not implemented yet".into())
}

fn print_usage() {
    println!("Usage: nc-daemon maint <dir> [turns]");
}
