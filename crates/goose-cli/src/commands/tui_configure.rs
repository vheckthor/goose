use std::{
    cmp::max, collections::HashMap, hash::Hash, io::{self, stdout}, panic::{set_hook, take_hook}, vec
};

use main_area::{chunks_for_list_and_view_split, render_left_list};
use ratatui::{
    backend::{Backend, CrosstermBackend}, crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
    }, layout::{self, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span, Text}, widgets::{Block, Borders, Clear, HighlightSpacing, List, ListState, Paragraph}, Frame, Terminal
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::profile::{self, load_profiles, Profile};

mod provider;
mod main_area;

pub async fn handle_tui_configure() -> io::Result<()> {
    init_panic_hook();
    let mut tui = init_tui()?;

    run(tui).await
}

fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // intentionally ignore errors here since we're already in a panic
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

fn init_tui() -> io::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Restore the terminal to its original state in order to not have side effects on the terminal after the program exits.
fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

struct ProfileUiState {
    profile_ui_mode: ProfileUIMode,
    profile_list_state: ListState,
    profiles: HashMap<String, Profile>,
}

#[derive(Clone)]
struct EditableProfile {
    pub focussed_field: InputField,
    pub name: Input,
    pub provider: String,
    pub model: Input,
    pub temperature: Input,
    pub context_limit: Input,
    pub max_tokens: Input,
    pub estimate_factor: Input,
    pub errors: HashMap<InputField, Option<String>>,
    pub provider_drowdown_open: bool,
    pub provider_list_state: ListState,
    pub edited: bool,
}

impl EditableProfile {
    fn new(name: &String, profile: &Profile) -> Self {
        let temperature = profile.temperature.map_or_else(Input::default, |temp| Input::default().with_value(temp.to_string()));
        let context_limit = profile.context_limit.map_or_else(Input::default, |limit| Input::default().with_value(limit.to_string()));
        let max_tokens = profile.max_tokens.map_or_else(Input::default, |tokens| Input::default().with_value(tokens.to_string()));
        let estimate_factor = profile.estimate_factor.map_or_else(Input::default, |factor| Input::default().with_value(factor.to_string()));

        let mut it = Self {
            focussed_field: InputField::Name,
            name: Input::default().with_value(name.to_string()),
            provider: profile.provider.clone(),
            model: Input::default().with_value(profile.model.clone()),
            temperature,
            context_limit,
            max_tokens,
            estimate_factor,
            errors: HashMap::new(),
            provider_drowdown_open: false,
            provider_list_state: ListState::default(),
            edited: false,
        };

        it.validate();
        it
    }

    fn validate(&mut self) {
        let mut errors: HashMap<InputField, Option<String>> = HashMap::new();
        if self.name.value().is_empty() {
            errors.insert(InputField::Name, Some("Required".to_string()));
        } else {
            errors.insert(InputField::Name, None);
        }

        if self.model.value().is_empty() {
            errors.insert(InputField::Model, Some("Required".to_string()));
        } else {
            errors.insert(InputField::Model, None);
        }

        self.errors = errors;
    }

    fn is_valid(&self) -> bool {
        self.errors.iter().all(|(_, error)| error.is_none())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum InputField {
    Name,
    Provider,
    Model,
    Temperature,
    ContextLimit,
    MaxTokens,
    EstimateFactor,
}

/// Within the profile view, which mode the profile is in.
enum ProfileUIMode {
    ProfileView,
    ProfileEdit,
}

/// Top level which view is in focus.
#[derive(Clone, PartialEq, Eq, Hash)]
enum UIMode {
    Profile,
    Provider,
}

impl ProfileUiState {
    fn new() -> Self {
        let mut state= Self { 
            profile_ui_mode: ProfileUIMode::ProfileView, 
            profile_list_state: ListState::default(), 
            profiles: load_profiles().unwrap(),
        };
        if state.profiles.len() > 0 {
            state.profile_list_state.select_first();
        }
        state
    }
}

enum AppOutcome {
    Continue,
    Exit,
    UpMenu, // Go up a menu level
}

struct App {
    main_menu_focussed: bool,
    ui_mode: UIMode,
    profile_ui_state: ProfileUiState,
    edit_profile: Option<EditableProfile>, // Probably move this into the ProfileUiState.
    provider_ui: provider::ProviderUi,
}

impl App {
    fn new() -> Self {
        Self {
            main_menu_focussed: false,
            ui_mode: UIMode::Profile,
            profile_ui_state: ProfileUiState::new(),
            edit_profile: None,
            provider_ui: provider::ProviderUi::new(),
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        // Fit all the profile items and enough room to display their details including systems, just using dummy 14 for now.
        let profile_view_height = max(14, self.profile_ui_state.profiles.len() + 4) as u16;

        let vertical_chunks = Layout::default()
            .direction(layout::Direction::Vertical)
            // header, context title (Profiles), main display, footer
            .constraints([layout::Constraint::Length(1), layout::Constraint::Length(3), layout::Constraint::Length(profile_view_height), layout::Constraint::Min(1), layout::Constraint::Min(1)])
            .split(f.area());

        let main_area = vertical_chunks[2];
        let footer_area = vertical_chunks[3];

        render_header(f, vertical_chunks[0]);
        self.render_main_menu(f, vertical_chunks[1]);

        // Main area
        match self.ui_mode {
            UIMode::Profile => {
                self.render_profile_main_area(f, main_area);
            }
            UIMode::Provider => {
                self.provider_ui.render_main_area(f, main_area, !self.main_menu_focussed);
            }
        }

        // Footer
        // TODO: Provide the correct actions for the current mode.
        let actions = match self.profile_ui_state.profile_ui_mode {
            ProfileUIMode::ProfileView => vec![Span::raw("Profile"),Span::raw("[N] New"), Span::raw("[E] Edit")],
            ProfileUIMode::ProfileEdit => {
                if self.edit_profile.as_ref().unwrap().edited {
                    vec![Span::raw("Profile"),Span::styled("[Enter] Save", Style::default().add_modifier(Modifier::BOLD)), Span::raw("[Esc] Cancel")]
                } else {
                    vec![Span::raw("Profile"),Span::raw("[Enter] Save"), Span::raw("[Esc] Cancel")]
                }
            }
        };
        render_footer(f, footer_area, &actions);
    }

    fn handle_events(&mut self) -> io::Result<AppOutcome> {
        if let Event::Key(key) = event::read()? {
            match key {
                KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, kind: KeyEventKind::Press, state: KeyEventState::NONE } => {
                    return Ok(AppOutcome::Exit);
                }
                _ => {}
            }
            if self.main_menu_focussed {
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(AppOutcome::Exit);
                    }
                    KeyCode::Left => {
                        match self.ui_mode {
                            UIMode::Profile => {}
                            UIMode::Provider => {
                                self.ui_mode = UIMode::Profile;
                            }
                        }
                    }
                    KeyCode::Right => {
                        match self.ui_mode {
                            UIMode::Profile => {
                                self.ui_mode = UIMode::Provider;
                            }
                            UIMode::Provider => {}
                        }
                    }
                    KeyCode::Down | KeyCode::Enter => {
                        self.main_menu_focussed = false;
                    }
                    _ => {}
                }
            } else {
                match self.ui_mode {
                    UIMode::Provider => {
                        match self.provider_ui.handle_events(key)? {
                            AppOutcome::UpMenu => {
                                self.main_menu_focussed = true;
                                return Ok(AppOutcome::Continue);
                            }
                            o => {
                                return Ok(o);
                            }
                        }
                    }
                    UIMode::Profile => {
                        match self.profile_ui_state.profile_ui_mode {
                            ProfileUIMode::ProfileView => {
                                match key.code {
                                    KeyCode::Char('q') => {
                                        return Ok(AppOutcome::Exit);
                                    }
                                    KeyCode::Esc => {
                                        self.main_menu_focussed = true;
                                    }
                                    KeyCode::Char('e') | KeyCode::Enter => {
                                        if has_profiles(&self.profile_ui_state.profiles) {
                                            self.profile_ui_state.profile_ui_mode = ProfileUIMode::ProfileEdit;
                                            let profile_names = profile_list_names(&self.profile_ui_state.profiles);
                                            let (name, profile) = selected_profile(&self.profile_ui_state, &profile_names).unwrap();
                                            self.edit_profile = Some(EditableProfile::new(name, profile));
                                        }
                                    }
                                    KeyCode::Down => {
                                        self.profile_ui_state.profile_list_state.select_next();
                                    }
                                    KeyCode::Up => {
                                        if self.profile_ui_state.profile_list_state.selected().is_some_and(|v| v == 0) {
                                            self.main_menu_focussed = true;
                                        } else {
                                            self.profile_ui_state.profile_list_state.select_previous();
                                        }
                                        
                                    }
                                    _ => {}
                                }
                            }
                            ProfileUIMode::ProfileEdit => {
                                if let Some(edit_profile) = self.edit_profile.as_mut() {
                                    if edit_profile.focussed_field == InputField::Provider {
                                        if edit_profile.provider_drowdown_open {
                                            match key.code {
                                                KeyCode::Esc => {
                                                    edit_profile.focussed_field = InputField::Model;
                                                    edit_profile.provider_drowdown_open = false;
                                                },
                                                KeyCode::Down => {
                                                    edit_profile.provider_list_state.select_next();
                                                }
                                                KeyCode::Up => {
                                                    edit_profile.provider_list_state.select_previous();
                                                }
                                                KeyCode::Enter => {
                                                    edit_profile.edited = true;
                                                    let selected_provider = edit_profile.provider_list_state.selected().unwrap_or(0);
                                                    let provider = provider_list()[selected_provider].clone();
                                                    edit_profile.provider = provider.to_string();
                                                    edit_profile.focussed_field = InputField::Model;
                                                    edit_profile.provider_drowdown_open = false;
                                                }
                                                _ => {}
                                            }
                                        } else { // provider dropdown not open
                                            match key.code {
                                                KeyCode::Esc => {
                                                    self.profile_ui_state.profile_ui_mode = ProfileUIMode::ProfileView;
                                                    self.edit_profile = None;
                                                },
                                                KeyCode::Down | KeyCode::Tab => {
                                                    edit_profile.focussed_field = next_field(edit_profile.focussed_field.clone());
                                                }
                                                KeyCode::Up | KeyCode::BackTab => {
                                                    edit_profile.focussed_field = prev_field(edit_profile.focussed_field.clone());
                                                }
                                                _ => {
                                                    edit_profile.provider_drowdown_open = true;
                                                    let index = provider_list().iter().position(|provider| provider == &edit_profile.provider).unwrap_or(0);
                                                    edit_profile.provider_list_state.select(Some(index));
                                                }
                                            }
                                        }
                                    } else { // provider field not focussed.
                                        match key.code {
                                            KeyCode::Esc => {
                                                self.profile_ui_state.profile_ui_mode = ProfileUIMode::ProfileView;
                                                self.edit_profile = None;
                                            },
                                            KeyCode::Enter => { // Change to save key
                                                if let Some(edit_profile) = self.edit_profile.as_mut() {
                                                    match edit_profile.focussed_field {
                                                        InputField::Name => {
                                                            let profile_names = profile_list_names(&self.profile_ui_state.profiles);
                                                            let (name, _) = selected_profile(&self.profile_ui_state, &profile_names).unwrap();
                                                            let name_clone = name.clone();
                                                            if edit_profile.name.value() != name_clone {
                                                                self.profile_ui_state.profiles.remove(&name_clone);
                                                            }
                                                            profile::remove_profile(name_clone.as_str()).unwrap();
                                                        },
                                                        _ => {}
                                                    }
                                                    // TODO: Update all the other fields and save the profiles.
                                                    let new_profile = Profile {
                                                        provider: edit_profile.provider.clone(),
                                                        model: edit_profile.model.value().to_string(),
                                                        additional_systems: vec![],
                                                        temperature: edit_profile.temperature.value().parse().ok(),
                                                        context_limit: edit_profile.context_limit.value().parse().ok(),
                                                        max_tokens: edit_profile.max_tokens.value().parse().ok(),
                                                        estimate_factor: edit_profile.estimate_factor.value().parse().ok(),
                                                    };
                                                    self.profile_ui_state.profiles.insert(edit_profile.name.value().to_string(), new_profile.clone());
                                                    profile::save_profile(edit_profile.name.value(), new_profile).unwrap();

                                                    self.profile_ui_state.profile_ui_mode = ProfileUIMode::ProfileView;
                                                    self.edit_profile = None;
                                                }
                                            }
                                            KeyCode::Down | KeyCode::Tab => {
                                                edit_profile.focussed_field = next_field(edit_profile.focussed_field.clone());
                                            }
                                            KeyCode::Up | KeyCode::BackTab => {
                                                edit_profile.focussed_field = prev_field(edit_profile.focussed_field.clone());
                                            }
                                            // Add cancel key
                                            _ => {
                                                edit_profile.edited = true;
                                                if let Some(edit_profile) = self.edit_profile.as_mut() {
                                                    match edit_profile.focussed_field {
                                                        //TODO: validations
                                                        InputField::Name => {
                                                            edit_profile.name.handle_event(&Event::Key(key));
                                                        },
                                                        InputField::Provider => {
                                                            // edit_profile.provider.handle_event(&Event::Key(key));
                                                        },
                                                        InputField::Model => {
                                                            edit_profile.model.handle_event(&Event::Key(key));
                                                        },
                                                        InputField::Temperature => {
                                                            edit_profile.temperature.handle_event(&Event::Key(key));
                                                        },
                                                        InputField::ContextLimit => {
                                                            edit_profile.context_limit.handle_event(&Event::Key(key));
                                                        },
                                                        InputField::MaxTokens => {
                                                            edit_profile.max_tokens.handle_event(&Event::Key(key));
                                                        },
                                                        InputField::EstimateFactor => {
                                                            edit_profile.estimate_factor.handle_event(&Event::Key(key));
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(AppOutcome::Continue)
    }

    fn render_main_menu(&mut self, f: &mut Frame, area: Rect) {
        let profiles_title = Span::styled("Profiles", main_menu_item_style(self.main_menu_focussed, self.ui_mode == UIMode::Profile));
        let providers_title = Span::styled("Providers", main_menu_item_style(self.main_menu_focussed, self.ui_mode == UIMode::Provider));
        let systems_title = Span::styled("Systems (todo)", main_menu_item_style(self.main_menu_focussed, false));
        f.render_widget(Paragraph::new(vec!
            [Line::from(""),
            Line::from(vec![Span::raw("   "), profiles_title, Span::raw("   "), providers_title, Span::raw("   "), systems_title]),
            Line::from(vec![Span::raw("─".repeat((f.area().width as usize).saturating_sub(24)))]),
            Line::from(""),
            ]
        ).block(Block::default().borders(Borders::BOTTOM)), area);
    }

    /// Render the main area of the profile view.
    // TODO: Move this into a Profile struct rather than having it in the App struct.
    fn render_profile_main_area(&mut self, f: &mut Frame, main_area: Rect) {
        let profile_list_names: Vec<String> = profile_list_names(&self.profile_ui_state.profiles);
        let has_profiles: bool = profile_list_names.len() > 0;

        let main_area_horizontal_chunks = chunks_for_list_and_view_split(main_area);
        render_left_list(f, "Profiles".to_string(), profile_list_names.clone(), &mut self.profile_ui_state.profile_list_state, !self.main_menu_focussed, main_area_horizontal_chunks[0]);

        // Main - Profile details area
        match self.profile_ui_state.profile_ui_mode {
            ProfileUIMode::ProfileView => {
                if has_profiles {
                    let (selected_profile_name, selected_profile) = selected_profile(&self.profile_ui_state, &profile_list_names).unwrap();
                    let profile_view = Paragraph::new(vec![
                        Line::from(vec![Span::styled("    Profile Details", Style::default().add_modifier(Modifier::ITALIC))]),
                        Line::from(vec!["".into()]),
                        Line::from(vec!["    Name:             ".into(), selected_profile_name.clone().into()]),
                        Line::from(vec!["    Provider:         ".into(), selected_profile.provider.clone().into()]),
                        Line::from(vec!["    Model:            ".into(), selected_profile.model.clone().into()]),
                        Line::from(vec!["    Temperature:      ".into(), selected_profile.temperature.clone().map_or("".into(), |temp| temp.to_string().into())]),
                        Line::from(vec!["    Context Limit:    ".into(), selected_profile.context_limit.clone().map_or("".into(), |limit| limit.to_string().into())]),
                        Line::from(vec!["    Max Tokens:       ".into(), selected_profile.max_tokens.clone().map_or("".into(), |tokens| tokens.to_string().into())]),
                        Line::from(vec!["    Estimate Factor:  ".into(), selected_profile.estimate_factor.clone().map_or("".into(), |factor| factor.to_string().into())]),
                    ]).block(Block::default().borders(Borders::NONE));
                    f.render_widget(profile_view, main_area_horizontal_chunks[1]);
                } else {
                    let profile_view = Paragraph::new(vec![
                        Line::from(vec![Span::styled("    Profile Details", Style::default().add_modifier(Modifier::ITALIC))]),
                        Line::from(vec!["".into()]),
                        Line::from(vec!["    Create a New Profile".into()]),
                    ]).block(Block::default().borders(Borders::NONE));
                    f.render_widget(profile_view, main_area_horizontal_chunks[1]);
                }
            },
            ProfileUIMode::ProfileEdit => {
                let edit_section_chunks = Layout::default()
                    .direction(layout::Direction::Vertical)
                    .constraints([layout::Constraint::Length(2), layout::Constraint::Min(1)])
                    .split(main_area_horizontal_chunks[1]);

                let edit_header = Paragraph::new(vec![
                    Line::from(vec![Span::styled("    Edit Profile", Style::default().add_modifier(Modifier::ITALIC))]),
                    Line::from(vec!["".into()]),
                ]).block(Block::default().borders(Borders::NONE));

                f.render_widget(edit_header, edit_section_chunks[0]);


                let edit_profile = self.edit_profile.as_ref().unwrap();                
                let input_offset = 22;
                let lines = vec![
                    editable_profile_line("Name", &edit_profile.name, edit_profile.errors.get(&InputField::Name).cloned().flatten(), input_offset, edit_profile.focussed_field == InputField::Name),
                    if edit_profile.focussed_field == InputField::Provider {
                        non_editable_dropdown_profile_line("Provider", &edit_profile.provider, None, input_offset, edit_profile.focussed_field == InputField::Provider)
                    } else {
                        non_editable_profile_line("Provider", &edit_profile.provider, None, input_offset, edit_profile.focussed_field == InputField::Provider)
                    },
                    editable_profile_line("Model", &edit_profile.model, edit_profile.errors.get(&InputField::Model).cloned().flatten(), input_offset, edit_profile.focussed_field == InputField::Model),
                    editable_profile_line("Temperature", &edit_profile.temperature, None, input_offset, edit_profile.focussed_field == InputField::Temperature),
                    editable_profile_line("Context Limit", &edit_profile.context_limit, None, input_offset, edit_profile.focussed_field == InputField::ContextLimit),
                    editable_profile_line("Max Tokens", &edit_profile.max_tokens, None, input_offset, edit_profile.focussed_field == InputField::MaxTokens),
                    editable_profile_line("Estimate Factor", &edit_profile.estimate_factor, None, input_offset, edit_profile.focussed_field == InputField::EstimateFactor),
                ];
                let edit_profile_area_pos = edit_section_chunks[1].as_position();
                // let mut provider_popup: Option<ProviderPopup> = None;
                match edit_profile.focussed_field {
                    InputField::Name => {
                        f.set_cursor_position((edit_profile_area_pos.x + input_offset + edit_profile.name.visual_cursor() as u16, edit_profile_area_pos.y));
                    },
                    InputField::Provider => {
                        f.set_cursor_position((edit_profile_area_pos.x + input_offset + 0, edit_profile_area_pos.y + 1));
                        // provider_popup = Some(ProviderPopup{});
                    },
                    InputField::Model => {
                        f.set_cursor_position((edit_profile_area_pos.x + input_offset + edit_profile.model.visual_cursor() as u16, edit_profile_area_pos.y + 2));
                    },
                    InputField::Temperature => {
                        f.set_cursor_position((edit_profile_area_pos.x + input_offset + edit_profile.temperature.visual_cursor() as u16, edit_profile_area_pos.y + 3));
                    },
                    InputField::ContextLimit => {
                        f.set_cursor_position((edit_profile_area_pos.x + input_offset + edit_profile.context_limit.visual_cursor() as u16, edit_profile_area_pos.y + 4));
                    },
                    InputField::MaxTokens => {
                        f.set_cursor_position((edit_profile_area_pos.x + input_offset + edit_profile.max_tokens.visual_cursor() as u16, edit_profile_area_pos.y + 5));
                    },
                    InputField::EstimateFactor => {
                        f.set_cursor_position((edit_profile_area_pos.x + input_offset + edit_profile.estimate_factor.visual_cursor() as u16, edit_profile_area_pos.y + 6));
                    },
                }
                let edit_profile_form = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::NONE));
                f.render_widget(edit_profile_form, edit_section_chunks[1]);

                if edit_profile.focussed_field == InputField::Provider && edit_profile.provider_drowdown_open  {
                    let target_area = Rect::new(edit_profile_area_pos.x + input_offset + edit_profile.provider.len() as u16 + 2, edit_profile_area_pos.y + 1, 17, 6);
                    f.render_widget(Clear::default(), target_area);
                    let block = Block::new()
                        .borders(Borders::ALL);
                    let provider_list = List::new(provider_list())
                        .highlight_symbol(" > ")
                        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                        .highlight_spacing(HighlightSpacing::Always)
                        .block(block);
                    f.render_stateful_widget(provider_list, target_area, &mut self.edit_profile.as_mut().unwrap().provider_list_state);
                }
            }
        }
    }
}

async fn run(mut tui: Terminal<impl Backend>) -> io::Result<()> {
    let mut app = App::new();
    loop {
        tui.draw(|f| app.draw(f) )?;
        match app.handle_events() {
            Ok(AppOutcome::Continue) | Ok(AppOutcome::UpMenu) => continue,
            Ok(AppOutcome::Exit) => break,
            Err(_) => break,
        }
    }
    restore_tui()?;
    Ok(())
}

fn render_header(f: &mut Frame, header_area: layout::Rect) {
    let title = Line::from(vec![
        Span::raw("─".repeat(10)),
        Span::styled(" Configure Goose ", Style::default().add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Block::default().borders(Borders::TOP).title(title), header_area);
}

// Main menu functions
fn main_menu_item_style(main_menu_focussed: bool, is_selected: bool) -> Style {
    let mut style = Style::default();
    if main_menu_focussed {
        if is_selected {
            style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        } else {
            style = style.add_modifier(Modifier::BOLD);
        }
    } else if is_selected {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    style
}

// Profile functions

fn selected_profile<'a>(profile_ui_state: &'a ProfileUiState, profile_list_names: &'a Vec<String>) -> Option<(&'a String, &'a Profile)> {
    let target_profile_name = profile_list_names.get(profile_ui_state.profile_list_state.selected().unwrap_or(0)).unwrap();
    Some(profile_ui_state.profiles.iter().find(|(name, _)| target_profile_name == *name).map(|(name, profile)| (name, profile)).unwrap())
}


fn render_footer(f: &mut Frame, footer_area: layout::Rect, actions: &Vec<Span>) {
    let actions_prefix = vec![Span::raw(" ".repeat(3)), actions[0].clone().into(), Span::raw(":"), Span::raw(" ".repeat(3))];
    let actions_suffix = actions.iter().skip(1).fold(Vec::new(), |mut acc, action| {
        acc.push(action.clone());
        acc.push(Span::raw(" ".repeat(3)));
        acc
    });
    let footer = Text::from(vec![
        Line::from([actions_prefix, actions_suffix].concat()),
        Line::from(vec![
            Span::raw(" ".repeat(3)),
            Span::styled("App:       ", Style::default()),
            Span::styled("[Ctrl+C] Quit", Style::default()),
        ])
    ]);

    let title_line = Line::from(vec![
        Span::raw("─".repeat(10)),
        Span::styled(" Actions ", Style::default()),
    ]);
    let block = Block::default().borders(Borders::TOP).title(title_line);
    f.render_widget(Paragraph::new(footer).block(block), footer_area);
}

fn has_profiles(profiles: &HashMap<String, Profile>) -> bool {
    profiles.len() > 0
}

fn profile_list_names(profiles: &HashMap<String, Profile>) -> Vec<String> {
    let mut strs: Vec<String> = profiles.iter().map(|(name, _)| name.clone()).collect();
    strs.sort();
    strs
}

fn editable_profile_line<'a>(label: &'a str, input: &'a Input, error: Option<String>, input_offset: u16, focussed: bool) -> Line<'a> {
    let err_span = if let Some(err) = error {
        Span::styled(err.clone(), Style::default().fg(Color::Red))
    } else {
        Span::raw("".to_string())
    };
    let label_span = if focussed {
        Span::styled(label, Style::default().add_modifier(Modifier::BOLD))
    } else {
        Span::raw(label)
    };
    let prefix_spaces = " ".repeat(input_offset as usize - label.len() - 5);
    Line::from(vec!["    ".into(), label_span, ":".into(), prefix_spaces.into(), input.value().into(), "       ".into(), err_span])
}

fn non_editable_profile_line<'a>(label: &'a str, input: &'a str, error: Option<String>, input_offset: u16, focussed: bool) -> Line<'a> {
    let err_span = if let Some(err) = error {
        Span::styled(err.clone(), Style::default().fg(Color::Red))
    } else {
        Span::raw("".to_string())
    };
    let label_span = if focussed {
        Span::styled(label, Style::default().add_modifier(Modifier::BOLD))
    } else {
        Span::raw(label)
    };
    let prefix_spaces = " ".repeat(input_offset as usize - label.len() - 5);
    Line::from(vec!["    ".into(), label_span, ":".into(), prefix_spaces.into(), input.into(), "       ".into(), err_span])
}

fn non_editable_dropdown_profile_line<'a>(label: &'a str, input: &'a str, error: Option<String>, input_offset: u16, focussed: bool) -> Line<'a> {
    let err_span = if let Some(err) = error {
        Span::styled(err.clone(), Style::default().fg(Color::Red))
    } else {
        Span::raw("".to_string())
    };
    let label_span = if focussed {
        Span::styled(label, Style::default().add_modifier(Modifier::BOLD))
    } else {
        Span::raw(label)
    };
    let prefix_spaces = " ".repeat(input_offset as usize - label.len() - 5);
    Line::from(vec!["    ".into(), label_span, ":".into(), prefix_spaces.into(), input.into(), " ▼     ".into(), err_span])
}

fn next_field(current_field: InputField) -> InputField {
    match current_field {
        InputField::Name => InputField::Provider,
        InputField::Provider => InputField::Model,
        InputField::Model => InputField::Temperature,
        InputField::Temperature => InputField::ContextLimit,
        InputField::ContextLimit => InputField::MaxTokens,
        InputField::MaxTokens => InputField::EstimateFactor,
        InputField::EstimateFactor => InputField::Name,
    }
}

fn prev_field(current_field: InputField) -> InputField {
    match current_field {
        InputField::Name => InputField::EstimateFactor,
        InputField::Provider => InputField::Name,
        InputField::Model => InputField::Provider,
        InputField::Temperature => InputField::Model,
        InputField::ContextLimit => InputField::Temperature,
        InputField::MaxTokens => InputField::ContextLimit,
        InputField::EstimateFactor => InputField::MaxTokens,
    }
}

// Shared functions

pub fn provider_list() -> Vec<String> {
    vec!["anthropic".to_string(), "databricks".to_string(), "ollama".to_string(), "openai".to_string()]
}