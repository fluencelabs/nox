use std::io;

use crate::contract_status::app::App;
use crate::contract_status::app::Node;
use crate::contract_status::status::Status;
use crate::contract_status::ui::events::Event;
use crate::contract_status::ui::events::Events;
use crate::contract_status::ui::TabsState;
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

// row selection
enum MoveSelection {
    Up,
    Down,
}
// tab selection
enum SwitchTab {
    Next,
    Previous,
}
// render loop action, depends on user input. see `handle_input`
enum LoopAction {
    Exit,
    Continue,
}
// what tab is currently displayed. calculated from tab index, see `State::current_tab`
enum CurrentTab {
    Nodes,
    Apps,
}

struct Config {
    selected_style: Style,
    normal_style: Style,
}

impl Config {
    fn default() -> Config {
        Config {
            selected_style: Style::default().fg(Color::Yellow).modifier(Modifier::Bold),
            normal_style: Style::default().fg(Color::White),
        }
    }
}

struct State<'a> {
    pub tabs: TabsState<'a>,
    pub nodes: TabTable<'a, Node>,
    pub apps: TabTable<'a, App>,
    pub config: Config,
}

impl<'a> State<'a> {
    fn current_tab(&self) -> CurrentTab {
        match self.tabs.index {
            0 => CurrentTab::Nodes,
            _ => CurrentTab::Apps,
        }
    }

    // change selected row
    fn move_selection(&mut self, mv: MoveSelection) {
        let tab = self.current_tab();

        // take number of rows in current table
        let items = match tab {
            CurrentTab::Nodes => self.nodes.elements.len(),
            CurrentTab::Apps => self.apps.elements.len(),
        };

        // if table is empty -- nothing to move
        if items == 0 {
            return;
        }

        // mutable reference to selected position
        let selected = match tab {
            CurrentTab::Nodes => &mut self.nodes.selected,
            CurrentTab::Apps => &mut self.apps.selected,
        };

        match mv {
            // 1 step down, with wrap to top
            MoveSelection::Down => *selected = (*selected + 1) % items,
            // 1 step up if not at the top
            MoveSelection::Up if *selected > 0 => *selected = *selected - 1,
            // jump from top to bottom
            MoveSelection::Up => *selected = items - 1,
        };
    }

    fn switch_tab(&mut self, sw: SwitchTab) {
        match sw {
            SwitchTab::Next => self.tabs.next(),
            SwitchTab::Previous => self.tabs.previous(),
        }
    }
}

// represents the main table
struct TabTable<'a, T: ToColumns> {
    pub header: Vec<&'a str>,
    pub widths: Vec<u16>,
    pub selected: usize,
    pub elements: Vec<T>,
}

impl<'a> TabTable<'a, Node> {
    pub fn nodes(nodes: Vec<Node>) -> TabTable<'a, Node> {
        let header = vec![
            // 2 spaces needed to provide left margin
            "  Tendermint Key",
            "NodeID",
            "IP",
            "Next port",
            "Owner",
            "Private",
        ];
        let widths = vec![30, 25, 20, 10, 25, 5];

        TabTable {
            header,
            widths,
            selected: 0,
            elements: nodes,
        }
    }
}

impl<'a> TabTable<'a, App> {
    pub fn apps(apps: Vec<App>) -> TabTable<'a, App> {
        // 2 spaces needed to provide left margin
        let header = vec!["  App ID", "Size", "Owner"];
        let widths = vec![25, 5, 25];
        TabTable {
            header,
            widths,
            selected: 0,
            elements: apps,
        }
    }
}

// represents row data convertible to table column
trait ToColumns: Sized + Clone {
    fn columns(self) -> Vec<String>;
}

impl ToColumns for Node {
    fn columns(self) -> Vec<String> {
        // 2 spaces needed to provide left margin
        let tendermint_key = format!("  {}", self.tendermint_key);
        let node_id = format!("{:#x}", self.id);
        let next_port = self.next_port.to_string();
        let ip_addr = self.ip_addr;
        let owner = format!("{:#x}", self.owner);
        let is_private = if self.is_private { "yes" } else { "no" };
        vec![
            tendermint_key,
            node_id,
            ip_addr,
            next_port,
            owner,
            is_private.to_string(),
        ]
    }
}

impl ToColumns for App {
    fn columns(self) -> Vec<String> {
        // 2 spaces needed to provide left margin
        let app_id = format!("  {:#x}", self.app_id);
        let cluster_size = self.cluster_size.to_string();
        let owner = format!("{:#x}", self.owner);

        vec![app_id, cluster_size, owner]
    }
}

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
        tabs: TabsState::new(vec!["Nodes", "Apps"]),
        nodes: TabTable::nodes(status.nodes.clone()),
        apps: TabTable::apps(status.apps.clone()),
        config: Config::default(),
    };

    let events = Events::new();

    loop {
        terminal.draw(|mut f| {
            let size = f.size();
            let rects = get_layout(size);

            draw_main_box(&mut f);
            draw_tabs(&mut f, &state, rects[0]);

            match state.tabs.index {
                0 => draw_table(&mut f, &state.nodes, &state.config, rects[1]),
                _ => draw_table(&mut f, &state.apps, &state.config, rects[1]),
            };
        })?;

        match handle_input(&mut state, &events)? {
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
        .titles(&state.tabs.titles)
        .select(state.tabs.index)
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
fn handle_input<'a>(state: &mut State<'a>, events: &Events) -> Result<LoopAction, Error> {
    match events.next()? {
        Event::Input(key) => match key {
            Key::Char('q') => return Ok(LoopAction::Exit),
            Key::Down => state.move_selection(MoveSelection::Down),
            Key::Up => state.move_selection(MoveSelection::Up),
            Key::Right => state.switch_tab(SwitchTab::Next),
            Key::Left => state.switch_tab(SwitchTab::Previous),
            _ => {}
        },
        _ => {}
    };

    Ok(LoopAction::Continue)
}
