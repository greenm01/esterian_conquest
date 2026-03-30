#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

fn main() {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    let result = ec_connect::gui::run();
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    let result = ec_connect::cli::run();

    if let Err(err) = result {
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        {
            ec_connect::gui::show_fatal_error(&err.to_string());
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            eprintln!("error: {err}");
        }
        std::process::exit(1);
    }
}
