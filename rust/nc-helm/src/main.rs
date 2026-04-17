fn main() {
    if let Err(err) = nc_helm::main_entry() {
        eprintln!("nc-helm fatal error: {err}");
        std::process::exit(1);
    }
}
