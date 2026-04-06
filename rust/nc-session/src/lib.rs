//! nc-session — shared launch infrastructure for nc-game and nc-dash.
//!
//! Provides:
//! - `StartupPhase` — generic phase progression for the intro flow
//! - `SessionLeaseGuard` — session heartbeat and cleanup
//! - `detect_color_mode` — terminal color depth detection
//! - `LaunchArgs` — parsed CLI arguments shared between frontends

pub mod args;
pub mod lease;
pub mod startup;
