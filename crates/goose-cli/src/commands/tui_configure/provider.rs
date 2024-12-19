use crate::commands::tui_configure::provider_list;
use ratatui::{widgets::List, Frame};

pub struct ProviderUi {
}

impl ProviderUi {
    pub fn new() -> Self {
        Self {
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let provider_names = provider_list();
    }
}