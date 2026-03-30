#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

fn main() {
    #[cfg(windows)]
    let result = ec_connect::gui::run();
    #[cfg(not(windows))]
    let result = ec_connect::cli::run();

    if let Err(err) = result {
        #[cfg(windows)]
        ec_connect::gui::show_fatal_error(&err.to_string());
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
