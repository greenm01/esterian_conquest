pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }
    Err("nc-daemon seats is not implemented yet".into())
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-daemon seats list --dir <path>");
    println!("  nc-daemon seats reissue --dir <path> --player N");
    println!("  nc-daemon seats reset --dir <path> --player N");
    println!("  nc-daemon seats open --dir <path> --player N");
    println!("  nc-daemon seats close --dir <path> --player N");
}
