fn main() {
    if let Err(err) = ec_gate::cli::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
