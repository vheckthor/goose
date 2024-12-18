use std::{
    cmp::max, collections::HashMap, io::{self, stdout}, panic::{set_hook, take_hook}, vec
};

use ratatui::{
    backend::{Backend, CrosstermBackend}, crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
    }, layout::{self, Layout}, style::{Modifier, Style}, text::{Line, Span, Text}, widgets::{Block, Borders, HighlightSpacing, List, ListState, Padding, Paragraph}, Frame, Terminal
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::profile::{load_profiles, Profile};

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

struct ConfigureState {
    ui_mode: UIMode,
    profile_list_state: ListState,
    profiles: HashMap<String, Profile>,
}

#[derive(Clone)]
struct EditableProfile {
    pub focussed_field: InputField,
    pub name: Input,
    pub model: Input,
    pub temperature: Input,
    pub context_limit: Input,
    pub max_tokens: Input,
    pub estimate_factor: Input,
}

impl EditableProfile {
    fn new(name: &String, profile: &Profile) -> Self {
        let temperature = profile.temperature.map_or_else(Input::default, |temp| Input::default().with_value(temp.to_string()));
        let context_limit = profile.context_limit.map_or_else(Input::default, |limit| Input::default().with_value(limit.to_string()));
        let max_tokens = profile.max_tokens.map_or_else(Input::default, |tokens| Input::default().with_value(tokens.to_string()));
        let estimate_factor = profile.estimate_factor.map_or_else(Input::default, |factor| Input::default().with_value(factor.to_string()));
        Self {
            focussed_field: InputField::Name,
            name: Input::default().with_value(name.to_string()),
            model: Input::default().with_value(profile.model.clone()),
            temperature,
            context_limit,
            max_tokens,
            estimate_factor,
        }
    }
}

#[derive(Clone, PartialEq)]
enum InputField {
    Name,
    // Provider,
    Model,
    Temperature,
    ContextLimit,
    MaxTokens,
    EstimateFactor,
}

enum UIMode {
    ProfileView,
    ProfileEdit,
}

impl ConfigureState {
    fn new() -> Self {
        let mut state= Self { 
            ui_mode: UIMode::ProfileView, 
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
}

struct App {
    ui_state: ConfigureState,
    edit_profile: Option<EditableProfile>,
}

impl App {
    fn new() -> Self {
        Self {
            ui_state: ConfigureState::new(),
            edit_profile: None,
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let profile_list_names: Vec<String> = profile_list_names(&self.ui_state.profiles);
        let has_profiles: bool = profile_list_names.len() > 0;

        // Fit all the profile items and enough room to display their details including systems, just using dummy 14 for now.
        let profile_view_height = max(14, self.ui_state.profiles.len() + 4) as u16;

        let vertical_chunks = Layout::default()
            .direction(layout::Direction::Vertical)
            // header, context title (Profiles), main display, footer
            .constraints([layout::Constraint::Length(1), layout::Constraint::Length(3), layout::Constraint::Length(profile_view_height), layout::Constraint::Min(1), layout::Constraint::Min(1)])
            .split(f.area());

        let main_area = vertical_chunks[2];
        let footer_area = vertical_chunks[3];

        render_header(f, vertical_chunks[0]);

        let profiles_title = Span::styled("Profiles", Style::default().add_modifier(Modifier::UNDERLINED));
        let systems_title = Span::styled("Systems (todo)", Style::default()); // Update modifier when selected
        f.render_widget(Text::from(vec!
            [Line::from(""),
            Line::from(vec![Span::raw("   "), profiles_title, Span::raw("   "), systems_title]),
            Line::from(vec![Span::raw("─".repeat((f.area().width as usize).saturating_sub(24)))]),
            Line::from(""),
            ]
        ), vertical_chunks[1]);

        // Main area
        let main_area_horizontal_chunks = Layout::default()
            .direction(layout::Direction::Horizontal)
            .constraints([layout::Constraint::Length(33), layout::Constraint::Min(1)])
            .split(main_area);

        // Main - Profile list area
        let profile_list_chunks = Layout::default()
            .direction(layout::Direction::Vertical)
            .constraints([layout::Constraint::Length(2), layout::Constraint::Min(1)])
            .split(main_area_horizontal_chunks[0]);

        let profile_list_header = Paragraph::new(vec![
            Line::from(vec![Span::styled("   Profiles List", Style::default().add_modifier(Modifier::ITALIC))]),
            Line::from(vec!["".into()])
        ]);
        f.render_widget(profile_list_header, profile_list_chunks[0]);

        
        let profile_list = List::new(profile_list_names.clone())
            .block(Block::default().borders(Borders::NONE))
            .highlight_symbol(" > ")
            .highlight_spacing(HighlightSpacing::Always);
        f.render_stateful_widget(profile_list, profile_list_chunks[1], &mut self.ui_state.profile_list_state);

        // Main - Profile details area
        match self.ui_state.ui_mode {
            UIMode::ProfileView => {
                if has_profiles {
                    let (selected_profile_name, selected_profile) = selected_profile(&self.ui_state, &profile_list_names).unwrap();
                    let profile_view = Paragraph::new(vec![
                        Line::from(vec![Span::styled("    Profile Details", Style::default().add_modifier(Modifier::ITALIC))]),
                        Line::from(vec!["".into()]),
                        Line::from(vec!["    Name:         ".into(), selected_profile_name.clone().into()]),
                        Line::from(vec!["    Provider:     ".into(), selected_profile.provider.clone().into()]),
                        Line::from(vec!["    Model:        ".into(), selected_profile.model.clone().into()]),
                    ]).block(Block::default().borders(Borders::LEFT));
                    f.render_widget(profile_view, main_area_horizontal_chunks[1]);
                } else {
                    let profile_view = Paragraph::new(vec![
                        Line::from(vec![Span::styled("    Profile Details", Style::default().add_modifier(Modifier::ITALIC))]),
                        Line::from(vec!["".into()]),
                        Line::from(vec!["    Create a New Profile".into()]),
                    ]).block(Block::default().borders(Borders::LEFT));
                    f.render_widget(profile_view, main_area_horizontal_chunks[1]);
                }
            },
            UIMode::ProfileEdit => {
                let edit_section_chunks = Layout::default()
                    .direction(layout::Direction::Vertical)
                    .constraints([layout::Constraint::Length(2), layout::Constraint::Min(1), layout::Constraint::Min(1), layout::Constraint::Min(1), layout::Constraint::Min(1), layout::Constraint::Min(1), layout::Constraint::Min(1)])
                    .split(main_area_horizontal_chunks[1]);

                // TODO: Implement editing profile
                let edit_header = Paragraph::new(vec![
                    Line::from(vec![Span::styled("    Edit Profile", Style::default().add_modifier(Modifier::ITALIC))]),
                    Line::from(vec!["".into()]),
                ]).block(Block::default().borders(Borders::LEFT));

                f.render_widget(edit_header, edit_section_chunks[0]);


                // TODO: Render editable fields
                let edit_profile = self.edit_profile.as_ref().unwrap();
                render_editable_profile_row(f, "Name", &edit_profile.name, edit_section_chunks[1], edit_profile.focussed_field == InputField::Name);
                render_editable_profile_row(f, "Model", &edit_profile.model, edit_section_chunks[2], edit_profile.focussed_field == InputField::Model);
                render_editable_profile_row(f, "Temperature", &edit_profile.temperature, edit_section_chunks[3], edit_profile.focussed_field == InputField::Temperature);
                render_editable_profile_row(f, "Context Limit", &edit_profile.context_limit, edit_section_chunks[4], edit_profile.focussed_field == InputField::ContextLimit);
                render_editable_profile_row(f, "Max Tokens", &edit_profile.max_tokens, edit_section_chunks[5], edit_profile.focussed_field == InputField::MaxTokens);
                render_editable_profile_row(f, "Estimate Factor", &edit_profile.estimate_factor, edit_section_chunks[6], edit_profile.focussed_field == InputField::EstimateFactor);
            }
        }

        // Footer
        render_footer(f, footer_area);
    }

    fn handle_events(&mut self) -> io::Result<AppOutcome> {
        if let Event::Key(key) = event::read()? {
            match key {
                KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, kind: KeyEventKind::Press, state: KeyEventState::NONE } => {
                    return Ok(AppOutcome::Exit);
                }
                _ => {}
            }
            match self.ui_state.ui_mode {
                UIMode::ProfileView => {
                    match key.code {
                        KeyCode::Char('q') => {
                            return Ok(AppOutcome::Exit);
                        }
                        KeyCode::Char('e') => {
                            if has_profiles(&self.ui_state.profiles) {
                                self.ui_state.ui_mode = UIMode::ProfileEdit;
                                let profile_names = profile_list_names(&self.ui_state.profiles);
                                let (name, profile) = selected_profile(&self.ui_state, &profile_names).unwrap();
                                self.edit_profile = Some(EditableProfile::new(name, profile));
                            }
                        }
                        KeyCode::Down => {
                            self.ui_state.profile_list_state.select_next();
                        }
                        KeyCode::Up => {
                            self.ui_state.profile_list_state.select_previous();
                        }
                        _ => {}
                    }
                }
                UIMode::ProfileEdit => {
                    match key.code {
                        KeyCode::Esc => {
                            self.ui_state.ui_mode = UIMode::ProfileView;
                            self.edit_profile = None;
                        }
                        KeyCode::Enter => { // Change to save key
                            if let Some(edit_profile) = self.edit_profile.as_mut() {
                                match edit_profile.focussed_field {
                                    InputField::Name => {
                                        let profile_names = profile_list_names(&self.ui_state.profiles);
                                        let (name, _) = selected_profile(&self.ui_state, &profile_names).unwrap();
                                        let name_clone = name.clone();
                                        if edit_profile.name.value() != name_clone {
                                            self.ui_state.profiles.remove(&name_clone);
                                        }
                                    },
                                    _ => {}
                                }
                                // TODO: Update all the other fields and save the profiles.
                                self.ui_state.profiles.insert(edit_profile.name.value().to_string(), Profile {
                                    provider: "todo".to_string(),
                                    model: "todo".to_string(),
                                    additional_systems: vec![],
                                    temperature: None,
                                    context_limit: None,
                                    max_tokens: None,
                                    estimate_factor: None,
                                });
                                self.ui_state.ui_mode = UIMode::ProfileView;
                            }
                        }
                        // Handle up/down arrow keys
                        // Add cancel key
                        _ => {
                            if let Some(edit_profile) = self.edit_profile.as_mut() {
                                match edit_profile.focussed_field {
                                    //TODO: validations
                                    InputField::Name => {
                                        edit_profile.name.handle_event(&Event::Key(key));
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
        Ok(AppOutcome::Continue)
    }
}

async fn run(mut tui: Terminal<impl Backend>) -> io::Result<()> {
    let mut app = App::new();
    loop {
        tui.draw(|f| app.draw(f) )?;
        match app.handle_events() {
            Ok(AppOutcome::Continue) => continue,
            Ok(AppOutcome::Exit) => break,
            Err(_) => break,
        }
    }
    restore_tui()?;
    Ok(())
}

fn selected_profile<'a>(ui_state: &'a ConfigureState, profile_list_names: &'a Vec<String>) -> Option<(&'a String, &'a Profile)> {
    let target_profile_name = profile_list_names.get(ui_state.profile_list_state.selected().unwrap_or(0)).unwrap();
    Some(ui_state.profiles.iter().find(|(name, _)| target_profile_name == *name).map(|(name, profile)| (name, profile)).unwrap())
}

fn render_header(f: &mut Frame, header_area: layout::Rect) {
    let header = Line::from(vec![
        Span::raw("─".repeat(10)),
        Span::styled(" Configure Goose ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("─".repeat((f.area().width as usize).saturating_sub(24))),
    ]);
    f.render_widget(header, header_area);
}

fn render_footer(f: &mut Frame, footer_area: layout::Rect) {
    let footer = Text::from(vec![Line::from(""),
        Line::from(vec![
            Span::raw("─".repeat(10)),
            Span::styled(" Actions ", Style::default()),
            Span::raw("─".repeat((f.area().width as usize).saturating_sub(24))),
        ]),
        Line::from(vec![
            Span::raw(" ".repeat(3)),
            Span::styled("Profile:   ", Style::default()),
            Span::styled("[N] New", Style::default()),
            Span::raw(" ".repeat(3)),
            Span::styled("[E] Edit", Style::default()),
            Span::raw(" ".repeat((f.area().width as usize).saturating_sub(24))),
        ]),
        Line::from(vec![
            Span::raw(" ".repeat(3)),
            Span::styled("App:       ", Style::default()),
            Span::styled("[Q] Quit", Style::default()),
            Span::raw(" ".repeat((f.area().width as usize).saturating_sub(24))),
        ])
    ]);
    f.render_widget(Paragraph::new(footer).block(Block::new()), footer_area);
}

fn has_profiles(profiles: &HashMap<String, Profile>) -> bool {
    profiles.len() > 0
}

fn profile_list_names(profiles: &HashMap<String, Profile>) -> Vec<String> {
    let mut strs: Vec<String> = profiles.iter().map(|(name, _)| name.clone()).collect();
    strs.sort();
    strs
}

fn render_editable_profile_row(f: &mut Frame, label: &str, input: &Input, area: layout::Rect, focussed: bool) {
    let scroll = input.visual_scroll(50 as usize);
    let line = Line::from(vec!["     ".into(), label.into(), "       ".into(), input.value().into()]);
    let pre_input_width = (line.clone().width() - input.value().len()) as u16;
    let label = Paragraph::new(line)
        .scroll((0, scroll as u16))
        .block(Block::default().borders(Borders::NONE).padding(Padding::ZERO));
    f.render_widget(label, area);
    

    if focussed {
        f.set_cursor_position((area.x + pre_input_width + ((input.visual_cursor()).max(scroll) - scroll) as u16, area.y ));
    }
}