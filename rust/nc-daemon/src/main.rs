fn main() {
    if let Err(err) = nc_daemon::run_cli(std::env::args()) {
        tracing::error!(error = %err, "nc-daemon command failed");
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
