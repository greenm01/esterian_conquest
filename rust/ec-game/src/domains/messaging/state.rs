pub struct MessagingState {
    pub compose_recipient_input: String,
    pub compose_recipient_status: Option<String>,
    pub compose_recipient_scroll_offset: usize,
    pub compose_recipient_cursor: usize,
    pub compose_recipient_empire: Option<u8>,
    pub compose_subject: String,
    pub compose_subject_status: Option<String>,
    pub compose_body: String,
    pub compose_body_cursor_row: usize,
    pub compose_body_cursor_col: usize,
    pub compose_body_status: Option<String>,
    pub compose_outbox_input: String,
    pub compose_outbox_status: Option<String>,
    pub compose_outbox_scroll_offset: usize,
    pub compose_outbox_cursor: usize,
    pub compose_sent_status: Option<String>,
    pub delete_reviewables_prompt_active: bool,
}

impl Default for MessagingState {
    fn default() -> Self {
        Self {
            compose_recipient_input: String::new(),
            compose_recipient_status: None,
            compose_recipient_scroll_offset: 0,
            compose_recipient_cursor: 0,
            compose_recipient_empire: None,
            compose_subject: String::new(),
            compose_subject_status: None,
            compose_body: String::new(),
            compose_body_cursor_row: 0,
            compose_body_cursor_col: 0,
            compose_body_status: None,
            compose_outbox_input: String::new(),
            compose_outbox_status: None,
            compose_outbox_scroll_offset: 0,
            compose_outbox_cursor: 0,
            compose_sent_status: None,
            delete_reviewables_prompt_active: false,
        }
    }
}
