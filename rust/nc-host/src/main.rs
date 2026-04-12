fn main() {
    if let Err(err) = nc_host::run_cli(std::env::args()) {
        tracing::error!(error = %err, "nc-host command failed");
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
