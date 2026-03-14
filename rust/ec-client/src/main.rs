fn main() -> Result<(), Box<dyn std::error::Error>> {
    ec_client::cli::run(std::env::args())
}
