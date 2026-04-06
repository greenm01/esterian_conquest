// StdoutTerminal now lives in nc-ui. Re-export for backward compatibility.
pub use nc_ui::terminal::stdout::{
    StdoutTerminal, ansi256_to_named16, redmean_dist, resolve_color, rgb_to_ansi256,
    rgb_to_named16,
};
