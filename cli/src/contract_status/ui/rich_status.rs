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

struct TabTable<'a, T: Clone + Sized> {
    pub header: Vec<&'a str>,
    pub widths: Vec<u16>,
    pub selected: usize,
    pub elements: Vec<T>,
}

impl<'a> TabTable<'a, Node> {
    pub fn nodes(nodes: Vec<Node>) -> TabTable<'a, Node> {
        let header = vec![
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

trait ToColumns: Sized + Clone {
    fn columns(self) -> Vec<String>;
}

impl ToColumns for Node {
    fn columns(self) -> Vec<String> {
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
        let app_id = format!("  {:#x}", self.app_id);
        let cluster_size = self.cluster_size.to_string();
        let owner = format!("{:#x}", self.owner);

        vec![app_id, cluster_size, owner]
    }
}

pub fn draw(status: &Status) -> Result<(), Error> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    let nodes = status.nodes.clone();
    let mut state = State {
        tabs: TabsState::new(vec!["Nodes", "Apps"]),
        nodes: TabTable::nodes(nodes),
        apps: TabTable::apps(status.apps.clone()),
        config: Config::default(),
    };

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

        let exit = handle_input(&mut state, &events, &status)?;
        if exit {
            break;
        }
    }

    Ok(())
}

fn get_layout(terminal_size: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(5)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(terminal_size)
}

fn draw_main_box<'a, B: Backend + 'a>(f: &mut Frame<B>) {
    Block::default().style(Style::default()).render(f, f.size());
}

fn draw_tabs<'a, B: Backend + 'a>(f: &mut Frame<B>, state: &State, area: Rect) {
    Tabs::default()
        .block(Block::default().borders(Borders::ALL).title("Tabs"))
        .titles(&state.tabs.titles)
        .select(state.tabs.index)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::Yellow))
        .render(f, area);
}

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

fn handle_input<'a>(
    state: &mut State<'a>,
    events: &Events,
    status: &Status,
) -> Result<bool, Error> {
    match events.next()? {
        Event::Input(key) => match key {
            Key::Char('q') => {
                return Ok(true);
            }
            Key::Down => {
                state.nodes.selected += 1;
                if state.nodes.selected > status.nodes().len() - 1 {
                    state.nodes.selected = 0;
                }
            }
            Key::Up => {
                if state.nodes.selected > 0 {
                    state.nodes.selected -= 1;
                } else {
                    state.nodes.selected = status.nodes().len() - 1;
                }
            }
            Key::Right => state.tabs.next(),
            Key::Left => state.tabs.previous(),
            _ => {}
        },
        _ => {}
    };

    Ok(false)
}
