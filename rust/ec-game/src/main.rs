fn main() {
    if let Err(err) = ec_game::cli::run(std::env::args()) {
        eprintln!("{err}");
        let code = ec_game::error::exit_code_for(err.as_ref()).unwrap_or(1);
        std::process::exit(code);
    }
}
