pub struct EmpireState {
    pub enemies_input: String,
    pub enemies_status: Option<String>,
    pub enemies_scroll_offset: usize,
    pub enemies_cursor: usize,
}

impl Default for EmpireState {
    fn default() -> Self {
        Self {
            enemies_input: String::new(),
            enemies_status: None,
            enemies_scroll_offset: 0,
            enemies_cursor: 0,
        }
    }
}
