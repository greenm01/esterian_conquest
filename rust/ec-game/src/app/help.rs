use crate::screen::help::{MenuHelpTopic, help_lines, menu_help_spec, render_help_popup};
use crate::screen::{PlayfieldBuffer, ScreenId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupHelp {
    pub title: String,
    pub lines: Vec<String>,
}

pub fn render_popup(buffer: &mut PlayfieldBuffer, popup: &PopupHelp) {
    render_help_popup(buffer, &popup.title, &popup.lines);
}

pub fn popup_for_screen(screen: ScreenId, door_mode: bool) -> Option<PopupHelp> {
    match screen {
        ScreenId::MainMenu => Some(from_menu_topic(MenuHelpTopic::Main, door_mode)),
        ScreenId::GeneralMenu => Some(from_menu_topic(MenuHelpTopic::General, door_mode)),
        ScreenId::FleetMenu => Some(from_menu_topic(MenuHelpTopic::Fleet, door_mode)),
        ScreenId::StarbaseMenu => Some(from_menu_topic(MenuHelpTopic::Starbase, door_mode)),
        ScreenId::PlanetMenu => Some(from_menu_topic(MenuHelpTopic::Planet, door_mode)),
        ScreenId::PlanetBuildMenu => Some(from_menu_topic(MenuHelpTopic::Build, door_mode)),
        ScreenId::FirstTimeMenu => Some(from_menu_topic(MenuHelpTopic::FirstTime, door_mode)),

        ScreenId::ThemePicker => Some(table_help(
            "COLOR THEMES",
            &[
                ("J/K", "move selection"),
                ("^U/^D", "page up/down"),
                ("Type", "filter themes by name"),
                ("Backspace", "erase typed filter text"),
                ("Enter", "apply selected theme"),
                ("Q/Esc", "close picker"),
                ("?", "show/hide helper"),
            ],
        )),
        ScreenId::FleetList
        | ScreenId::FleetMissionPicker
        | ScreenId::FleetEta
        | ScreenId::FleetTransfer
        | ScreenId::FleetDetach => Some(table_help(
            "FLEET COMMANDS",
            &[
                ("J/K", "move selection"),
                ("^U/^D", "page up/down"),
                ("Enter", "accept highlighted item"),
                ("Type", "use the prompt value shown on the command line"),
                ("Backspace", "erase typed prompt input"),
                ("Q/Esc", "return"),
                ("?", "show/hide helper"),
            ],
        )),
        ScreenId::StarbaseList | ScreenId::StarbaseReviewSelect => Some(table_help(
            "STARBASE COMMANDS",
            &[
                ("J/K", "move selection"),
                ("^U/^D", "page up/down"),
                ("Enter", "review highlighted starbase"),
                ("Type", "jump to a starbase number when shown"),
                ("Backspace", "erase typed prompt input"),
                ("Q/Esc", "return"),
                ("?", "show/hide helper"),
            ],
        )),
        ScreenId::PlanetBriefList(_, _)
        | ScreenId::PlanetDatabaseList
        | ScreenId::PlanetCommissionPicker
        | ScreenId::PlanetCommissionMenu
        | ScreenId::PlanetCommissionDraft
        | ScreenId::PlanetAutoCommissionReport
        | ScreenId::PlanetTransportPlanetSelect(_)
        | ScreenId::PlanetTransportFleetSelect(_)
        | ScreenId::PlanetBuildList
        | ScreenId::PlanetBuildChange => Some(table_help(
            "PLANET COMMANDS",
            &[
                ("J/K", "move selection"),
                ("^U/^D", "page up/down"),
                ("Enter", "accept highlighted item"),
                ("Type", "use the prompt value shown on the command line"),
                ("Backspace", "erase typed prompt input"),
                ("Q/Esc", "return"),
                ("?", "show/hide helper"),
            ],
        )),
        ScreenId::PlanetListSortPrompt(_)
        | ScreenId::PlanetDatabaseFilterPrompt
        | ScreenId::PlanetTransportQuantityPrompt(_)
        | ScreenId::PlanetBuildSpecify
        | ScreenId::PlanetBuildQuantity
        | ScreenId::PlanetCommissionResult
        | ScreenId::PlanetTransportDone(_)
        | ScreenId::ComposeMessageSubject
        | ScreenId::ComposeMessageBody
        | ScreenId::ComposeMessageDiscardConfirm
        | ScreenId::ComposeMessageSendConfirm
        | ScreenId::ComposeMessageSent => Some(prompt_help(
            "COMMAND HELP",
            &[
                ("Type", "enter the value shown on the command line"),
                ("Backspace", "erase typed input"),
                ("Enter", "accept the current prompt"),
                ("Q/Esc", "cancel or return when available"),
                ("?", "show/hide helper"),
            ],
        )),
        ScreenId::ComposeMessageRecipient | ScreenId::ComposeMessageOutbox => Some(table_help(
            "MESSAGE COMMANDS",
            &[
                ("J/K", "move selection"),
                ("^U/^D", "page up/down"),
                ("D", "open queued outgoing messages"),
                ("Enter", "accept highlighted item"),
                ("Type", "use the prompt value shown on the command line"),
                ("Backspace", "erase typed input"),
                ("Q/Esc", "return"),
                ("?", "show/hide helper"),
            ],
        )),
        ScreenId::Enemies | ScreenId::Rankings(_) => Some(table_help(
            "EMPIRE COMMANDS",
            &[
                ("J/K", "move selection"),
                ("^U/^D", "page up/down"),
                ("Enter", "accept highlighted item when available"),
                ("Type", "use the prompt value shown on the command line"),
                ("Backspace", "erase typed input"),
                ("Q/Esc", "return"),
                ("?", "show/hide helper"),
            ],
        )),
        ScreenId::Reports => Some(prompt_help(
            "INBOX COMMANDS",
            &[
                ("J/K", "move between report or mail items"),
                ("^U/^D", "page through long review text"),
                ("D", "delete the current or remaining item when offered"),
                ("Enter", "advance or accept the current prompt"),
                ("Q/Esc", "return"),
                ("?", "show/hide helper"),
            ],
        )),
        ScreenId::PartialStarmapView | ScreenId::Starmap => Some(prompt_help(
            "MAP COMMANDS",
            &[
                ("HJKL", "move the view cardinally"),
                (
                    "1 2 3 4 6 7 8 9",
                    "move diagonally or by keypad-style direction",
                ),
                ("Q/Esc", "return"),
                ("?", "show/hide helper"),
            ],
        )),
        ScreenId::FirstTimeIntro | ScreenId::Startup(_) => Some(prompt_help(
            "INTRO HELP",
            &[
                ("Enter", "advance the current page"),
                ("Q/Esc", "return when offered"),
                ("?", "show/hide helper"),
            ],
        )),
        _ => None,
    }
}

fn from_menu_topic(topic: MenuHelpTopic, door_mode: bool) -> PopupHelp {
    let spec = menu_help_spec(topic, door_mode);
    PopupHelp {
        title: spec.title.to_string(),
        lines: help_lines(spec.lines),
    }
}

fn table_help(title: &str, rows: &[(&str, &str)]) -> PopupHelp {
    PopupHelp {
        title: title.to_string(),
        lines: ec_ui::modal::format_help_rows(rows.iter().copied()),
    }
}

fn prompt_help(title: &str, rows: &[(&str, &str)]) -> PopupHelp {
    table_help(title, rows)
}
