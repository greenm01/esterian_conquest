use super::state::Screen;

pub const MAIN_MENU_RAIL: &str = "? J K ^U ^D N Y I M D R <Space> L <Q>";
pub const KEYCHAIN_MENU_RAIL: &str = "? R <Enter> L <Q>";
pub const GAME_SELECT_RAIL: &str = "? J K ^U ^D <Q>";
pub const RELAY_MENU_RAIL: &str = "? J K ^U ^D A E D S <Enter> <Q>";
pub const RELAY_GAMES_RAIL: &str = "? J K ^U ^D R <Q>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpTopic {
    MainCommand,
    KeychainCommand,
    ConnectCommand,
    SelectGame,
    RelayCommand,
    RelayGames,
    Identity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpRow {
    pub command: &'static str,
    pub description: &'static str,
}

pub struct HelpSpec {
    pub title: &'static str,
    pub rows: &'static [HelpRow],
    pub note: Option<&'static str>,
}

const MAIN_ROWS: &[HelpRow] = &[
    HelpRow {
        command: "J/K",
        description: "move selection",
    },
    HelpRow {
        command: "^U/^D",
        description: "page up/down",
    },
    HelpRow {
        command: "N",
        description: "join by invite code",
    },
    HelpRow {
        command: "Y",
        description: "open keychain screen",
    },
    HelpRow {
        command: "I",
        description: "show identity info",
    },
    HelpRow {
        command: "M",
        description: "download maps / set save location",
    },
    HelpRow {
        command: "D",
        description: "delete selected game",
    },
    HelpRow {
        command: "r",
        description: "open relay manager",
    },
    HelpRow {
        command: "R",
        description: "edit selected game relay",
    },
    HelpRow {
        command: "Space",
        description: "refresh selected game info",
    },
    HelpRow {
        command: "L",
        description: "lock the screen",
    },
    HelpRow {
        command: "?",
        description: "show/hide helper",
    },
    HelpRow {
        command: "Q",
        description: "quit ec-connect",
    },
    HelpRow {
        command: "Esc",
        description: "same as <Q> on this screen",
    },
];

const KEYCHAIN_ROWS: &[HelpRow] = &[
    HelpRow {
        command: "R",
        description: "replace current identity or import an nsec",
    },
    HelpRow {
        command: "Enter",
        description: "show current identity backup details",
    },
    HelpRow {
        command: "L",
        description: "lock the screen",
    },
    HelpRow {
        command: "?",
        description: "show/hide helper",
    },
    HelpRow {
        command: "Q",
        description: "return to main menu",
    },
    HelpRow {
        command: "Esc",
        description: "same as <Q> on this screen",
    },
];

const CONNECT_ROWS: &[HelpRow] = &[
    HelpRow {
        command: "Q",
        description: "cancel this prompt",
    },
    HelpRow {
        command: "?",
        description: "show/hide helper",
    },
    HelpRow {
        command: "Esc",
        description: "same as <Q> on this screen",
    },
];

const GAME_SELECT_ROWS: &[HelpRow] = &[
    HelpRow {
        command: "J/K",
        description: "move selection",
    },
    HelpRow {
        command: "^U/^D",
        description: "page up/down",
    },
    HelpRow {
        command: "Q",
        description: "cancel selection",
    },
    HelpRow {
        command: "?",
        description: "show/hide helper",
    },
    HelpRow {
        command: "Esc",
        description: "same as <Q> on this screen",
    },
];

const RELAY_ROWS: &[HelpRow] = &[
    HelpRow {
        command: "J/K",
        description: "move selection",
    },
    HelpRow {
        command: "^U/^D",
        description: "page up/down",
    },
    HelpRow {
        command: "A",
        description: "add relay",
    },
    HelpRow {
        command: "E",
        description: "edit selected relay",
    },
    HelpRow {
        command: "S",
        description: "set selected relay as default",
    },
    HelpRow {
        command: "D",
        description: "delete selected relay",
    },
    HelpRow {
        command: "Enter",
        description: "show games on selected relay",
    },
    HelpRow {
        command: "?",
        description: "show/hide helper",
    },
    HelpRow {
        command: "Q",
        description: "return to main menu",
    },
    HelpRow {
        command: "Esc",
        description: "same as <Q> on this screen",
    },
];

const RELAY_GAMES_ROWS: &[HelpRow] = &[
    HelpRow {
        command: "J/K",
        description: "move selection",
    },
    HelpRow {
        command: "^U/^D",
        description: "page up/down",
    },
    HelpRow {
        command: "R",
        description: "edit selected game relay",
    },
    HelpRow {
        command: "?",
        description: "show/hide helper",
    },
    HelpRow {
        command: "Q",
        description: "return to relay list",
    },
    HelpRow {
        command: "Esc",
        description: "same as <Q> on this screen",
    },
];

const IDENTITY_ROWS: &[HelpRow] = &[
    HelpRow {
        command: "Q",
        description: "close identity info",
    },
    HelpRow {
        command: "?",
        description: "show/hide helper",
    },
    HelpRow {
        command: "Esc",
        description: "same as <Q> on this screen",
    },
];

impl HelpTopic {
    pub fn for_screen(screen: &Screen) -> Option<Self> {
        match screen {
            Screen::GameList => Some(Self::MainCommand),
            Screen::RelayList => Some(Self::RelayCommand),
            Screen::RelayGames { .. } => Some(Self::RelayGames),
            Screen::KeychainAddPrompt => Some(Self::ConnectCommand),
            Screen::IdentityOverlay => Some(Self::Identity),
            Screen::KeychainList => Some(Self::KeychainCommand),
            Screen::GameSelect { .. } => Some(Self::SelectGame),
            Screen::Locked => None,
        }
    }

    pub fn spec(self) -> HelpSpec {
        match self {
            Self::MainCommand => HelpSpec {
                title: "MAIN COMMAND HELP",
                rows: MAIN_ROWS,
                note: None,
            },
            Self::KeychainCommand => HelpSpec {
                title: "KEYCHAIN COMMAND HELP",
                rows: KEYCHAIN_ROWS,
                note: None,
            },
            Self::ConnectCommand => HelpSpec {
                title: "CONNECT COMMAND HELP",
                rows: CONNECT_ROWS,
                note: Some("Type your text normally, then press Enter to submit."),
            },
            Self::SelectGame => HelpSpec {
                title: "SELECT GAME HELP",
                rows: GAME_SELECT_ROWS,
                note: Some("Press Enter to connect to the selected game."),
            },
            Self::RelayCommand => HelpSpec {
                title: "RELAY COMMAND HELP",
                rows: RELAY_ROWS,
                note: Some("Relay health is cached from recent connect and refresh activity."),
            },
            Self::RelayGames => HelpSpec {
                title: "RELAY GAMES HELP",
                rows: RELAY_GAMES_ROWS,
                note: Some("This view shows games currently assigned to the selected relay."),
            },
            Self::Identity => HelpSpec {
                title: "IDENTITY HELP",
                rows: IDENTITY_ROWS,
                note: None,
            },
        }
    }
}
