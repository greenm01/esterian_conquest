//! nc-session — shared launch infrastructure for nc-game and nc-dash.
//!
//! Provides:
//! - `StartupPhase` — generic phase progression for the intro flow
//! - `detect_color_mode` — terminal color depth detection
//! - `LaunchArgs` — parsed CLI arguments shared between frontends

pub mod args;
pub mod launch;
pub mod onboarding;
pub mod startup;
