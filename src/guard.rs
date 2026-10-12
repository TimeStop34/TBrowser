use ratatui::DefaultTerminal;

pub struct TuiGuard;

impl TuiGuard {
    pub fn new() -> (Self, DefaultTerminal) {
        let terminal = ratatui::init();
        (Self, terminal)
    }
}

impl Drop for TuiGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}