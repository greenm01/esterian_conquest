pub mod action;
mod build;
mod controller;
mod scorch;
pub mod screens;
pub mod state;
mod transport;
pub mod update;

pub use action::PlanetAction;
pub use state::PlanetState;
pub mod views;

#[derive(Clone, Copy)]
pub(crate) enum KnownOwnerLabelStyle {
    Detail,
    Database,
}

pub(crate) fn known_owner_label(
    known_owner_empire_id: Option<u8>,
    known_owner_empire_name: Option<&str>,
    style: KnownOwnerLabelStyle,
) -> String {
    match known_owner_empire_id {
        Some(0) => "Unowned".to_string(),
        Some(id) => match style {
            KnownOwnerLabelStyle::Detail => known_owner_empire_name
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("Empire #{id}")),
            KnownOwnerLabelStyle::Database => format!("#{id}"),
        },
        None => "?".to_string(),
    }
}
