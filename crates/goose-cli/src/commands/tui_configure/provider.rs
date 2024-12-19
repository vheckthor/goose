use crate::commands::tui_configure::provider_list;
use ratatui::{layout::Rect, widgets::{List, ListState}, Frame};

use super::main_area::{chunks_for_list_and_view_split, render_left_list};

pub struct ProviderUi {
    provider_list_state: ListState,
}

impl ProviderUi {
    pub fn new() -> Self {
        Self {
            provider_list_state: ListState::default(),
        }
    }

    /// Draw provider ui for the main area
    pub fn render_main_area(&mut self, f: &mut Frame, area: Rect, view_focussed: bool) {
        let provider_names = provider_list();
        let main_area_horizontal_chunks = chunks_for_list_and_view_split(area);
        render_left_list(f, "Profiles".to_string(), provider_names.clone(), &mut self.provider_list_state, view_focussed, main_area_horizontal_chunks[0]);

    }
}