use std::io;

use crate::commands::tui_configure::provider_list;
use ratatui::{crossterm::event::{KeyCode, KeyEvent}, layout::Rect, widgets::{List, ListState}, Frame};

use super::{main_area::{chunks_for_list_and_view_split, render_left_list}, AppOutcome};

pub struct ProviderUi {
    provider_list_state: ListState,
}

impl ProviderUi {
    pub fn new() -> Self {
        Self {
            provider_list_state: ListState::default().with_selected(Some(0)),
        }
    }

    /// Draw provider ui for the main area
    pub fn render_main_area(&mut self, f: &mut Frame, area: Rect, view_focussed: bool) {
        let provider_names = provider_list();
        let main_area_horizontal_chunks = chunks_for_list_and_view_split(area);

        render_left_list(f, "Providers".to_string(), provider_names.clone(), &mut self.provider_list_state, view_focussed, main_area_horizontal_chunks[0]);
    }

    pub fn handle_events(&mut self, key: KeyEvent) -> io::Result<AppOutcome> {
        match key.code {
            KeyCode::Esc => {
                return Ok(AppOutcome::UpMenu)
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                // Do stuff
            }
            KeyCode::Down => {
                self.provider_list_state.select_next();
            }
            KeyCode::Up => {
                if self.provider_list_state.selected().is_some_and(|v| v == 0) {
                    return Ok(AppOutcome::UpMenu);
                } else {
                    self.provider_list_state.select_previous();
                }
                
            }
            _ => {}
        }
        Ok(AppOutcome::Continue)
    }
}