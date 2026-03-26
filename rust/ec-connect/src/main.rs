fn main() {
    if let Err(err) = ec_connect::cli::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
