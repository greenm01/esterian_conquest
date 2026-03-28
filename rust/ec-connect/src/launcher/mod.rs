mod onboarding;
pub mod render;

use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, poll, read};

use ec_ui::paint::render_to_stdout;
use ec_ui::session::TerminalSession;

use crate::password::wallet_exists;
use crate::wallet::io::wallet_path;
use onboarding::run_first_identity_setup_in_session;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordGateMode {
    UnlockExisting,
    CreateNew,
    ConfirmNew,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordGateState {
    pub mode: PasswordGateMode,
    pub input: String,
    pub staged_password: String,
    pub error_msg: Option<String>,
}

impl PasswordGateState {
    pub fn new(existing_wallet: bool, error_msg: Option<String>) -> Self {
        Self {
            mode: if existing_wallet {
                PasswordGateMode::UnlockExisting
            } else {
                PasswordGateMode::CreateNew
            },
            input: String::new(),
            staged_password: String::new(),
            error_msg,
        }
    }

    pub fn title(&self) -> &'static str {
        match self.mode {
            PasswordGateMode::UnlockExisting => "Unlock Wallet",
            PasswordGateMode::CreateNew => "Create Wallet Password",
            PasswordGateMode::ConfirmNew => "Confirm Wallet Password",
        }
    }

    pub fn lead_line(&self) -> &'static str {
        match self.mode {
            PasswordGateMode::UnlockExisting => "Enter your wallet password.",
            PasswordGateMode::CreateNew => "Enter a new wallet password.",
            PasswordGateMode::ConfirmNew => "Enter it again to confirm.",
        }
    }

    pub fn field_label(&self) -> &'static str {
        match self.mode {
            PasswordGateMode::UnlockExisting => "Password:",
            PasswordGateMode::CreateNew => "New password:",
            PasswordGateMode::ConfirmNew => "Confirm password:",
        }
    }

    pub fn show_warning(&self) -> bool {
        self.mode != PasswordGateMode::UnlockExisting
    }

    pub fn masked_input(&self) -> String {
        "*".repeat(self.input.chars().count())
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    pub fn push_char(&mut self, ch: char) {
        self.input.push(ch);
    }

    pub fn submit(&mut self) -> GateSubmit {
        if self.input.is_empty() {
            self.error_msg = Some("Error: password cannot be empty.".to_string());
            return GateSubmit::Pending;
        }

        match self.mode {
            PasswordGateMode::UnlockExisting => {
                let password = std::mem::take(&mut self.input);
                self.error_msg = None;
                GateSubmit::Accepted(password)
            }
            PasswordGateMode::CreateNew => {
                self.staged_password = std::mem::take(&mut self.input);
                self.mode = PasswordGateMode::ConfirmNew;
                self.error_msg = None;
                GateSubmit::Pending
            }
            PasswordGateMode::ConfirmNew => {
                let confirm = std::mem::take(&mut self.input);
                if confirm != self.staged_password {
                    self.staged_password.clear();
                    self.mode = PasswordGateMode::CreateNew;
                    self.error_msg = Some("Error: passwords do not match. Start over.".to_string());
                    GateSubmit::Pending
                } else {
                    let password = std::mem::take(&mut self.staged_password);
                    self.error_msg = None;
                    GateSubmit::Accepted(password)
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateSubmit {
    Pending,
    Accepted(String),
}

pub fn run_password_gate_in_session(
    session: &mut TerminalSession,
    error_msg: Option<String>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let existing_wallet = wallet_exists(&wallet_path());
    let mut state = PasswordGateState::new(existing_wallet, error_msg);

    loop {
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 25));
        let buffer = render::render_buffer(&state, width, height);
        render_to_stdout(&buffer)?;

        if !poll(Duration::from_millis(250))? {
            continue;
        }

        let Event::Key(key) = read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Esc => return Ok(None),
            KeyCode::Char('q' | 'Q')
                if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
            {
                if state.input.is_empty() {
                    return Ok(None);
                }
                state.push_char(match key.code {
                    KeyCode::Char(ch) => ch,
                    _ => unreachable!(),
                });
            }
            KeyCode::Enter => {
                if let GateSubmit::Accepted(password) = state.submit() {
                    if !existing_wallet && !run_first_identity_setup_in_session(session, &password)?
                    {
                        return Ok(None);
                    }
                    return Ok(Some(password));
                }
            }
            KeyCode::Backspace => state.backspace(),
            KeyCode::Char(ch)
                if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
            {
                state.push_char(ch);
            }
            _ => {}
        }
    }
}

pub fn run_password_gate(
    error_msg: Option<String>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut session = TerminalSession::enter_picker()?;
    let result = run_password_gate_in_session(&mut session, error_msg);
    let _ = session.restore();
    result
}
