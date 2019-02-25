use crate::contract_status::app::App;
use crate::contract_status::app::Node;
use tui::style::Color;
use tui::style::Modifier;
use tui::style::Style;

pub mod async_keys;
pub mod rich_status;

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

impl std::fmt::Display for CurrentTab {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            CurrentTab::Nodes => "Nodes",
            CurrentTab::Apps => "Apps",
        };
        write!(f, "{}", s)
    }
}

impl CurrentTab {
    fn names() -> Vec<String> {
        vec![CurrentTab::Nodes.to_string(), CurrentTab::Apps.to_string()]
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
            "  NodeID",
            "IP",
            "Api port",
            "Owner",
            "Private",
        ];
        let widths = vec![70, 15, 10, 45, 5];

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
        let node_id = format!("  {:#x}", self.validator_key);
        let api_port = self.api_port.to_string();
        let ip_addr = format!("{}", self.ip_addr);
        let owner = format!("{:#x}", self.owner);
        let is_private = if self.is_private { "yes" } else { "no" };
        vec![node_id, ip_addr, api_port, owner, is_private.to_string()]
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
    pub tab: CurrentTab,
    pub nodes: TabTable<'a, Node>,
    pub apps: TabTable<'a, App>,
    pub config: Config,
}

impl<'a> State<'a> {
    // change selected row
    fn move_selection(&mut self, mv: MoveSelection) {
        // take number of rows in current table
        let items = match self.tab {
            CurrentTab::Nodes => self.nodes.elements.len(),
            CurrentTab::Apps => self.apps.elements.len(),
        };

        // if table is empty -- nothing to move
        if items == 0 {
            return;
        }

        // mutable reference to selected position
        let selected = match self.tab {
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

    fn switch_tab(&mut self, _sw: SwitchTab) {
        match self.tab {
            CurrentTab::Nodes => self.tab = CurrentTab::Apps,
            CurrentTab::Apps => self.tab = CurrentTab::Nodes,
        }
    }

    fn selected_tab(&self) -> usize {
        match self.tab {
            CurrentTab::Nodes => 0,
            CurrentTab::Apps => 1,
        }
    }
}
