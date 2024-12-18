use std::{
    cmp::max, io::{self, stdout}, panic::{set_hook, take_hook}, vec
};

use console::Key;
use ratatui::{
    backend::{Backend, CrosstermBackend}, crossterm::{
        event::{self, Event, KeyCode}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
    }, layout::{self, Layout}, style::{Modifier, Style}, text::{Line, Span, Text}, widgets::{Block, Borders, HighlightSpacing, List, ListState, Paragraph}, Frame, Terminal
};

use crate::{main, profile::{load_profiles, Profile}};

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
    profiles: Vec<(String, Profile)>,
    edit_profile: Option<(String, Profile)>,
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
            profiles: load_profiles().unwrap().into_iter().collect(),
            edit_profile: None,
        };
        if state.profiles.len() > 0 {
            state.profile_list_state.select_first();
        }
        state
    }
}

async fn run(mut tui: Terminal<impl Backend>) -> io::Result<()> {
    let mut ui_state = ConfigureState::new();

    loop {
        let mut profile_list_names: Vec<String> = ui_state.profiles.iter().map(|(name, _)| name.clone()).collect();
        let has_profiles = profile_list_names.len() > 0;
        profile_list_names.sort();
        tui.draw(|f| {
            // Fit all the profile items and enough room to display their details including systems, just using dummy 14 for now.
            let profile_view_height = max(14, ui_state.profiles.len() + 4) as u16;

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
            f.render_stateful_widget(profile_list, profile_list_chunks[1], &mut ui_state.profile_list_state);

            // Main - Profile details area
            match ui_state.ui_mode {
                UIMode::ProfileView => {
                    if has_profiles {
                        let (selected_profile_name, selected_profile) = ui_state.profiles.get(ui_state.profile_list_state.selected().unwrap_or(0)).unwrap();
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
                        .constraints([layout::Constraint::Length(2), layout::Constraint::Min(1)])
                        .split(main_area_horizontal_chunks[1]);

                    // TODO: Implement editing profile
                }
            }

            // Footer
            render_footer(f, footer_area);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    // TODO: Add confirmation dialog
                    restore_tui()?;
                    break;
                }
                KeyCode::Char('e') => {
                    if has_profiles {
                        ui_state.ui_mode = UIMode::ProfileEdit;
                        ui_state.edit_profile = Some(ui_state.profiles.get(ui_state.profile_list_state.selected().unwrap_or(0)).unwrap().clone());
                    }
                }
                KeyCode::Down => {
                    ui_state.profile_list_state.select_next();
                }
                KeyCode::Up => {
                    ui_state.profile_list_state.select_previous();

                }
                _ => {}
            }
        }
    }

    Ok(())
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