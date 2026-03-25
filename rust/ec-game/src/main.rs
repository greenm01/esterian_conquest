fn main() -> Result<(), Box<dyn std::error::Error>> {
    ec_game::cli::run(std::env::args())
}
