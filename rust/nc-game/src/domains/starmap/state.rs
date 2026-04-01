pub struct StarmapState {
    pub view_x: usize,
    pub view_y: usize,
    pub status: Option<String>,
    pub dump_lines: Vec<String>,
    pub dump_offset: usize,
    pub dump_active: bool,
    pub capture_complete: bool,
    pub partial_status: Option<String>,
    pub partial_center: [u8; 2],
}

impl Default for StarmapState {
    fn default() -> Self {
        Self {
            view_x: 0,
            view_y: 0,
            status: None,
            dump_lines: Vec::new(),
            dump_offset: 0,
            dump_active: false,
            capture_complete: false,
            partial_status: None,
            partial_center: [0, 0],
        }
    }
}
