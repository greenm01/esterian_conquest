//! Key event → Action mapping.

use crate::dashboard::input::{KeyCode, KeyEvent, KeyModifiers};

use crate::dashboard::app::state::{ActiveOverlay, PanelFocus};

/// Actions the dashboard can perform in response to input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    FocusNext,
    FocusPrev,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    Home,
    End,
    MoveCrosshairUp,
    MoveCrosshairDown,
    MoveCrosshairLeft,
    MoveCrosshairRight,
    JumpPlanetBackward,
    JumpPlanetForward,
    ToggleMapViewMode,
    ZoomMapIn,
    ZoomMapOut,
    ResetMapZoom,
    OpenPlanetDetailPopup,
    ToggleAutopilot,
    SetTaxRate,
    OpenOverlay(ActiveOverlay),
    CloseOverlay,
    ClosePopup,
    None,
}

/// Map a key event to an action given the current focus.
pub fn key_to_action(key: KeyEvent, focus: PanelFocus, overlay: ActiveOverlay) -> Action {
    // Global quit.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }

    // Overlay open — only Esc closes it (overlay handles its own keys otherwise).
    if overlay != ActiveOverlay::None {
        if key.code == KeyCode::Esc {
            return Action::CloseOverlay;
        }
        return Action::None; // overlay handles input internally
    }

    match key.code {
        // Panel focus cycling
        KeyCode::Tab => Action::FocusNext,
        KeyCode::BackTab => Action::FocusPrev,

        // Global overlay shortcuts
        KeyCode::Char('p') | KeyCode::Char('P') => Action::OpenOverlay(ActiveOverlay::PlanetList),
        KeyCode::Char('f') | KeyCode::Char('F') => Action::OpenOverlay(ActiveOverlay::FleetList),
        KeyCode::Char('i') | KeyCode::Char('I') => {
            Action::OpenOverlay(ActiveOverlay::IntelDatabase)
        }
        KeyCode::Char('r') | KeyCode::Char('R') => Action::OpenOverlay(ActiveOverlay::Inbox),
        KeyCode::Char('d') | KeyCode::Char('D') => Action::OpenOverlay(ActiveOverlay::Diplomacy),
        KeyCode::Char('s') | KeyCode::Char('S') => Action::OpenOverlay(ActiveOverlay::Settings),
        KeyCode::Char('?') => Action::OpenOverlay(ActiveOverlay::Help),

        // Autopilot toggle
        KeyCode::Char('a') | KeyCode::Char('A') => Action::ToggleAutopilot,

        // Tax rate
        KeyCode::Char('x') | KeyCode::Char('X') => Action::SetTaxRate,

        // Navigation — context-sensitive
        KeyCode::Up | KeyCode::Char('k') => match focus {
            PanelFocus::Map => Action::MoveCrosshairUp,
            _ => Action::ScrollUp,
        },
        KeyCode::Down | KeyCode::Char('j') => match focus {
            PanelFocus::Map => Action::MoveCrosshairDown,
            _ => Action::ScrollDown,
        },
        KeyCode::Left | KeyCode::Char('h') => match focus {
            PanelFocus::Map => Action::MoveCrosshairLeft,
            _ => Action::None,
        },
        KeyCode::Right | KeyCode::Char('l') => match focus {
            PanelFocus::Map => Action::MoveCrosshairRight,
            _ => Action::None,
        },
        KeyCode::Char('[') if focus == PanelFocus::Map => Action::JumpPlanetBackward,
        KeyCode::Char(']') if focus == PanelFocus::Map => Action::JumpPlanetForward,
        KeyCode::Char('v') | KeyCode::Char('V') if focus == PanelFocus::Map => {
            Action::ToggleMapViewMode
        }
        KeyCode::Char('+') | KeyCode::Char('=') if focus == PanelFocus::Map => Action::ZoomMapIn,
        KeyCode::Char('-') if focus == PanelFocus::Map => Action::ZoomMapOut,
        KeyCode::Char('z') | KeyCode::Char('Z') if focus == PanelFocus::Map => Action::ResetMapZoom,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Home => Action::Home,
        KeyCode::End => Action::End,
        KeyCode::Enter if focus == PanelFocus::Map => Action::OpenPlanetDetailPopup,

        _ => Action::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_keys_jump_planets_only_on_map_without_overlay() {
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::None,
            ),
            Action::JumpPlanetBackward
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::None,
            ),
            Action::JumpPlanetForward
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
                PanelFocus::Planets,
                ActiveOverlay::None,
            ),
            Action::None
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::PlanetList,
            ),
            Action::None
        );
    }

    #[test]
    fn enter_opens_planet_detail_only_on_map_without_overlay() {
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::None,
            ),
            Action::OpenPlanetDetailPopup
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                PanelFocus::Diplomacy,
                ActiveOverlay::None,
            ),
            Action::None
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::PlanetList,
            ),
            Action::None
        );
    }

    #[test]
    fn zoom_keys_only_apply_on_map_without_overlay() {
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char('='), KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::None,
            ),
            Action::ZoomMapIn
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::None,
            ),
            Action::ZoomMapOut
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::None,
            ),
            Action::ResetMapZoom
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::None,
            ),
            Action::ToggleMapViewMode
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char('='), KeyModifiers::NONE),
                PanelFocus::Economy,
                ActiveOverlay::None,
            ),
            Action::None
        );
        assert_eq!(
            key_to_action(
                KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE),
                PanelFocus::Map,
                ActiveOverlay::PlanetList,
            ),
            Action::None
        );
    }
}
