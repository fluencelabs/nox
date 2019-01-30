use std::io;

use crate::contract_status::status::Status;
use crate::contract_status::ui::async_keys::{AsyncKey, AsyncKeys};
use crate::contract_status::ui::Config;
use crate::contract_status::ui::CurrentTab;
use crate::contract_status::ui::LoopAction;
use crate::contract_status::ui::MoveSelection;
use crate::contract_status::ui::State;
use crate::contract_status::ui::SwitchTab;
use crate::contract_status::ui::TabTable;
use crate::contract_status::ui::ToColumns;
use failure::Error;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::Backend;
use tui::backend::TermionBackend;
use tui::layout::Direction;
use tui::layout::Rect;
use tui::layout::{Constraint, Layout};
use tui::style::{Color, Modifier, Style};
use tui::terminal::Frame;
use tui::widgets::Tabs;
use tui::widgets::{Block, Borders, Row, Table, Widget};
use tui::Terminal;

pub fn draw(status: &Status) -> Result<(), Error> {
    // terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // fill initial state
    let mut state = State {
        tab: CurrentTab::Nodes,
        nodes: TabTable::nodes(status.nodes.clone()),
        apps: TabTable::apps(status.apps.clone()),
        config: Config::default(),
    };

    let keys = AsyncKeys::new();

    loop {
        terminal.draw(|mut f| {
            let size = f.size();
            let rects = get_layout(size);

            draw_main_box(&mut f);
            draw_tabs(&mut f, &state, rects[0]);

            match state.tab {
                CurrentTab::Nodes => draw_table(&mut f, &state.nodes, &state.config, rects[1]),
                CurrentTab::Apps => draw_table(&mut f, &state.apps, &state.config, rects[1]),
            };
        })?;

        match handle_input(&keys, &mut state)? {
            LoopAction::Exit => break,
            LoopAction::Continue => (),
        }
    }

    Ok(())
}

// Builds layout and splits it in 2 rectangles: 1 for tabs, 1 for table
fn get_layout(terminal_size: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(5)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(terminal_size)
}

// Draw big rectangular frame
fn draw_main_box<'a, B: Backend + 'a>(f: &mut Frame<B>) {
    Block::default()
        .title("press 'q' for exit")
        .title_style(
            Style::default()
                .fg(Color::LightBlue)
                .modifier(Modifier::Bold),
        )
        .render(f, f.size());
}

// Draw tab selector
fn draw_tabs<'a, B: Backend + 'a>(f: &mut Frame<B>, state: &State, area: Rect) {
    Tabs::default()
        .block(Block::default().borders(Borders::ALL))
        .titles(&CurrentTab::names())
        .select(state.selected_tab())
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::Yellow))
        .render(f, area);
}

// Draw specified table and fill it with rows
fn draw_table<'a, B, T>(f: &mut Frame<B>, table: &TabTable<'a, T>, config: &Config, area: Rect)
where
    B: Backend + 'a,
    T: ToColumns,
{
    let rows = table
        .elements
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, e): (usize, T)| {
            let cols = e.columns();
            if i == table.selected {
                Row::StyledData(cols.into_iter(), config.selected_style)
            } else {
                Row::StyledData(cols.into_iter(), config.normal_style)
            }
        });

    Table::new(table.header.iter().cloned(), rows)
        .block(Block::default().borders(Borders::ALL))
        .widths(table.widths.as_slice())
        .column_spacing(5)
        .render(f, area);
}

// Handle user input. Arrow keys switch row or tabs, 'q' exits.
fn handle_input(keys: &AsyncKeys, state: &mut State) -> Result<LoopAction, Error> {
    match keys.next() {
        Ok(AsyncKey::Input(key)) => match key {
            Key::Char('q') => return Ok(LoopAction::Exit),
            Key::Down => state.move_selection(MoveSelection::Down),
            Key::Up => state.move_selection(MoveSelection::Up),
            Key::Right => state.switch_tab(SwitchTab::Next),
            Key::Left => state.switch_tab(SwitchTab::Previous),
            _ => (),
        },
        _ => (),
    };

    Ok(LoopAction::Continue)
}
