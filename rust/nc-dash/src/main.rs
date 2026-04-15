fn main() {
    if let Err(err) = nc_dash::main_entry() {
        eprintln!("nc-dash fatal error: {err}");
        std::process::exit(1);
    }
}
