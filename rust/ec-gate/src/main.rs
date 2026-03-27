fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(err) = ec_gate::cli::run() {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}
