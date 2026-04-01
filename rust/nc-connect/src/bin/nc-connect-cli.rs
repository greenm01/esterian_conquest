fn main() {
    if let Err(err) = nc_connect::cli::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
