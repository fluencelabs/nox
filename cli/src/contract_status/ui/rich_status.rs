use std::io;

use crate::contract_status::app::{App, Node};
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
    pub nodes: TabTable<'a>,
    pub apps: TabTable<'a>,
    pub config: &'a Config,
}

impl<'a> State<'a> {
    pub fn current_table(&'a mut self) -> &'a mut TabTable {
        match self.tabs.index {
            0 => &mut self.nodes,
            _ => &mut self.apps,
        }
    }
}

struct TabTable<'a> {
    pub header: Vec<&'a str>,
    pub widths: Vec<u16>,
    pub selected: usize,
}

pub fn node_columns(node: Node) -> Vec<String> {
    let tendermint_key = format!("  {}", node.tendermint_key);
    let node_id = format!("{:#x}", node.id);
    let next_port = node.next_port.to_string();
    let ip_addr = node.ip_addr;
    let owner = format!("{:#x}", node.owner);
    let is_private = if node.is_private { "yes" } else { "no" };
    vec![
        tendermint_key,
        node_id,
        ip_addr,
        next_port,
        owner,
        is_private.to_string(),
    ]
}

pub fn apps_columns(app: App) -> Vec<String> {
    //    pub app_id: H256,
    //    pub storage_hash: H256,
    //    pub storage_receipt: H256,
    //    pub cluster_size: u8,
    //    pub owner: Address,
    //    pub pin_to_nodes: Option<Vec<H256>>,
    //    pub cluster: Option<Cluster>,
    let app_id = format!("  {:#x}", app.app_id);
    let cluster_size = app.cluster_size.to_string();
    let owner = format!("{:#x}", app.owner);

    vec![app_id, cluster_size, owner]
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

    let header = vec![
        "  Tendermint Key",
        "NodeID",
        "IP",
        "Next port",
        "Owner",
        "Private",
    ];
    let widths = vec![30, 25, 20, 10, 25, 5];
    let nodes_table = TabTable {
        header,
        widths,
        selected: 0,
    };

    let header = vec!["  App ID", "Size", "Owner"];
    let widths = vec![25, 5, 25];
    let apps_table = TabTable {
        header,
        widths,
        selected: 0,
    };

    let mut state = State {
        tabs: TabsState::new(vec!["Nodes", "Apps"]),
        nodes: nodes_table,
        apps: apps_table,
        config: &Config::default(),
    };

    loop {
        terminal.draw(|mut f| {
            let size = f.size();
            let rects = get_layout(size);

            draw_main_box(&mut f);
            draw_tabs(&mut f, &state, rects[0]);

            match &state.tabs.index {
                0 => draw_nodes_table(&mut f, &state, status, rects[1]),
                1 => draw_apps_table(&mut f, &state, status, rects[1]),
                _ => (),
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

fn draw_nodes_table<'a, B>(f: &mut Frame<B>, state: &State, status: &Status, area: Rect)
where
    B: Backend + 'a,
{
    let rows = status.nodes().iter().cloned().enumerate().map(|(i, node)| {
        let cols = node_columns(node);
        if i == state.nodes.selected {
            Row::StyledData(cols.into_iter(), state.config.selected_style)
        } else {
            Row::StyledData(cols.into_iter(), state.config.normal_style)
        }
    });

    Table::new(state.nodes.header.iter().cloned(), rows)
        .block(Block::default().borders(Borders::ALL))
        .widths(state.nodes.widths.as_slice())
        .column_spacing(5)
        .render(f, area);
}

fn draw_apps_table<'a, B>(f: &mut Frame<B>, state: &State, status: &Status, area: Rect)
where
    B: Backend + 'a,
{
    let rows = status.apps().iter().cloned().enumerate().map(|(i, app)| {
        let cols = apps_columns(app);
        if i == state.apps.selected {
            Row::StyledData(cols.into_iter(), state.config.selected_style)
        } else {
            Row::StyledData(cols.into_iter(), state.config.normal_style)
        }
    });

    Table::new(state.apps.header.iter().cloned(), rows)
        .block(Block::default().borders(Borders::ALL))
        .widths(state.apps.widths.as_slice())
        .column_spacing(5)
        .render(f, area);
}

fn handle_input<'a>(
    state: &'a mut State<'a>,
    events: &Events,
    status: &Status,
) -> Result<bool, Error> {
    match events.next()? {
        Event::Input(key) => match key {
            Key::Char('q') => {
                return Ok(true);
            }
            Key::Down => {
                let table = state.current_table();
                table.selected += 1;
                if table.selected > status.nodes().len() - 1 {
                    table.selected = 0;
                }
            }
            Key::Up => {
                let table = state.current_table();
                if table.selected > 0 {
                    table.selected -= 1;
                } else {
                    table.selected = status.nodes().len() - 1;
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
