use crate::domains::startup::screens::startup::StartupReviewMode;

pub struct StartupState {
    pub splash_page: usize,
    pub intro_page: usize,
    pub results_block: usize,
    pub results_page: usize,
    pub results_mode: StartupReviewMode,
    pub results_nonstop: bool,
    pub messages_block: usize,
    pub messages_page: usize,
    pub messages_mode: StartupReviewMode,
    pub messages_nonstop: bool,
    pub results_deleted_any: bool,
    pub messages_deleted_any: bool,
    pub first_time_intro_page: usize,
    pub first_time_rename_preloaded_empire: bool,
}

impl Default for StartupState {
    fn default() -> Self {
        Self {
            splash_page: 0,
            intro_page: 0,
            results_block: 0,
            results_page: 0,
            results_mode: StartupReviewMode::ViewPrompt,
            results_nonstop: false,
            messages_block: 0,
            messages_page: 0,
            messages_mode: StartupReviewMode::ViewPrompt,
            messages_nonstop: false,
            results_deleted_any: false,
            messages_deleted_any: false,
            first_time_intro_page: 0,
            first_time_rename_preloaded_empire: false,
        }
    }
}
