use std::io;

use crate::commands::{configure::get_required_keys, tui_configure::provider_list};
use goose::key_manager::{get_keyring_secret, KeyRetrievalStrategy};
use ratatui::{crossterm::event::{KeyCode, KeyEvent}, layout::{self, Layout, Rect}, text::Span, widgets::{Block, List, ListState}, Frame};

use super::{main_area::{chunks_for_list_and_view_split, render_left_list}, AppOutcome};

pub struct ProviderUi {
    provider_list_state: ListState,
    providers: Vec<ProviderWithState>,
}

impl ProviderUi {
    pub fn new() -> Self {
        let providers = provider_states(provider_list(), false);
        Self {
            provider_list_state: ListState::default().with_selected(Some(0)),
            providers,
        }
    }

    /// Draw provider ui for the main area
    pub fn render_main_area(&mut self, f: &mut Frame, area: Rect, view_focussed: bool) {
        let provider_names = self.renderable_provider_list();
        let main_area_horizontal_chunks = chunks_for_list_and_view_split(area);

        render_left_list(f, "Providers".to_string(), provider_names.clone(), &mut self.provider_list_state, view_focussed, main_area_horizontal_chunks[0]);

        // Render the right side
        let right_chunks = Layout::default()
            .direction(layout::Direction::Vertical)
            .constraints([layout::Constraint::Length(2), layout::Constraint::Min(1)])
            .split(main_area_horizontal_chunks[1]);

        let selected_provider = self.provider_list_state.selected().map(|i| &self.providers[i]);
        if let Some(provider) = selected_provider {
            let provider_attributes: Vec<String> = provider.attributes.iter().map(|attr| {
                let source = match attr.source {
                    AttributeSource::Env => "Present in Env",
                    AttributeSource::Keyring => "Present in Keyring",
                    AttributeSource::Missing => "Missing",
                    AttributeSource::Pending => "Unknown: Press 'c' to check if present in keyring. You may be prompted by a keyring access request.",
                };
                format!("   {}: {}", attr.name, source)
            }).collect();
            let provider_attributes = List::new(provider_attributes)
                .block(Block::default());
            f.render_widget(provider_attributes, right_chunks[1]);
        }
    }

    pub fn handle_events(&mut self, key: KeyEvent) -> io::Result<AppOutcome> {
        match key.code {
            KeyCode::Esc => {
                return Ok(AppOutcome::UpMenu)
            }
            KeyCode::Char('q') => {
                return Ok(AppOutcome::Exit)
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                // Do stuff
            }
            KeyCode::Char('c') => {
                // TODO: Add some UI feedback that we checked things otherwise its instantaneous.
                let selected_provider = self.provider_list_state.selected().map(|i| &self.providers[i]);
                if let Some(provider) = selected_provider {
                    let provider_name = provider.name.clone();
                    let provider_attributes = provider_state(provider_name.clone(), true);
                    let provider_index = self.providers.iter().position(|p| p.name == provider_name).unwrap();
                    self.providers[provider_index].attributes = provider_attributes;
                }
            }
            KeyCode::Char('t') => {
                // TODO: Test connection to provider.
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

    pub fn action_footer_names(&self) -> Vec<Span> {
        vec![Span::raw("Provider"), Span::raw("[C] Check Configuration"), Span::raw("[T] Test Connection")] // TODO: Add edit config
    }

    fn renderable_provider_list(&self) -> Vec<String> {
        self.providers.iter().map(|provider| {
            if provider.attributes.iter().any(|attr| attr.source == AttributeSource::Pending) {
                format!("{} {}", '?', provider.name)
            } else if provider.attributes.iter().any(|attr| attr.source == AttributeSource::Missing) {
                format!("{} {}", '-', provider.name)
            } else {
                format!("{} {}", '✔', provider.name)
            }
        }).collect()
    }
}

#[derive(PartialEq)]
enum AttributeSource {
    Env,
    Keyring,
    Missing,
    Pending,
}
struct AttributeState {
    name: String, // Attribute name
    source: AttributeSource,
}
struct ProviderWithState {
    name: String, // Provider name
    attributes: Vec<AttributeState>
}

// We don't always check the keyring as it requires granting access to the keychain which might scare people without forewarning.
fn provider_states(provider_names: Vec<String>, also_check_keyring: bool) -> Vec<ProviderWithState> {
    provider_names.into_iter().map(|name| {
        ProviderWithState { 
            name: name.clone(), 
            attributes: provider_state(name, also_check_keyring)
        }
    }).collect()
}

fn provider_state(name: String, also_check_keyring: bool) -> Vec<AttributeState> {
    get_required_keys(&name).into_iter().map(|key| {
        let source = if also_check_keyring && get_keyring_secret(key, KeyRetrievalStrategy::KeyringOnly).is_ok() {
            AttributeSource::Keyring
        } else if get_keyring_secret(key, KeyRetrievalStrategy::EnvironmentOnly).is_ok() {
            AttributeSource::Env
        } else {
            // If we haven't checked the keyring, we don't know if it's missing.
            if also_check_keyring { AttributeSource::Missing } else { AttributeSource::Pending }
        };
        AttributeState {
            name: key.to_string(),
            source
        }
    }).collect()
}