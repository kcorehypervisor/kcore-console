//! App state, page enum, and keyboard / exit policy (dev vs production).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;

use crate::inventory::Snapshot;

/// Top-level TUI pages (tabs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Overview,
    Network,
    Storage,
    Gpu,
    Security,
    Diagnostics,
    Help,
}

impl Page {
    pub const ALL: [Page; 7] = [
        Page::Overview,
        Page::Network,
        Page::Storage,
        Page::Gpu,
        Page::Security,
        Page::Diagnostics,
        Page::Help,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Overview => "Overview",
            Page::Network => "Network",
            Page::Storage => "Storage",
            Page::Gpu => "GPU",
            Page::Security => "Security",
            Page::Diagnostics => "Diagnostics",
            Page::Help => "Help",
        }
    }

    /// Tab-bar label. Diagnostics is shortened so all seven tabs fit an 80-column console.
    pub fn tab_label(self) -> &'static str {
        match self {
            Page::Diagnostics => "Diag",
            other => other.label(),
        }
    }

    /// Number key that jumps straight to this page.
    pub fn hotkey(self) -> char {
        match self {
            Page::Overview => '1',
            Page::Network => '2',
            Page::Storage => '3',
            Page::Gpu => '4',
            Page::Security => '5',
            Page::Diagnostics => '6',
            Page::Help => '7',
        }
    }

    pub fn next(self) -> Page {
        match self {
            Page::Overview => Page::Network,
            Page::Network => Page::Storage,
            Page::Storage => Page::Gpu,
            Page::Gpu => Page::Security,
            Page::Security => Page::Diagnostics,
            Page::Diagnostics => Page::Help,
            Page::Help => Page::Overview,
        }
    }

    pub fn prev(self) -> Page {
        match self {
            Page::Overview => Page::Help,
            Page::Network => Page::Overview,
            Page::Storage => Page::Network,
            Page::Gpu => Page::Storage,
            Page::Security => Page::Gpu,
            Page::Diagnostics => Page::Security,
            Page::Help => Page::Diagnostics,
        }
    }
}

/// What the event loop should do after a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Refresh,
    Quit,
}

/// Rows moved by PageUp / PageDown.
const PAGE_STEP: usize = 10;

/// Per-page selection and scroll state plus the latest inventory snapshot.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub page: Page,
    pub dev: bool,
    pub network: TableState,
    pub storage: TableState,
    pub gpu: TableState,
    pub security: TableState,
    /// Lines scrolled back from the newest system-log line.
    pub log_scroll: usize,
    /// Wall-clock label of the last applied snapshot, `None` before the first one.
    pub last_refresh: Option<String>,
    /// Set after `r` until the next snapshot arrives.
    pub refreshing: bool,
    pub last_msg: Option<String>,
    pub snapshot: Snapshot,
}

impl AppState {
    pub fn new(dev: bool, initial: Snapshot) -> Self {
        let mut s = Self {
            page: Page::default(),
            dev,
            network: TableState::default(),
            storage: TableState::default(),
            gpu: TableState::default(),
            security: TableState::default(),
            log_scroll: 0,
            last_refresh: None,
            refreshing: false,
            last_msg: None,
            snapshot: initial,
        };
        s.clamp_selections();
        s
    }

    /// When `true`, the `q` key exits the TUI. Only enabled in dev mode.
    pub fn allow_exit_on_q(&self) -> bool {
        self.dev
    }

    /// Replace the inventory and keep selections inside the new row counts.
    pub fn apply_snapshot(&mut self, snapshot: Snapshot, refreshed_at: String) {
        self.snapshot = snapshot;
        self.last_refresh = Some(refreshed_at);
        self.refreshing = false;
        self.clamp_selections();
    }

    pub fn network_sel(&self) -> usize {
        self.network.selected().unwrap_or(0)
    }

    pub fn storage_sel(&self) -> usize {
        self.storage.selected().unwrap_or(0)
    }

    pub fn clamp_selections(&mut self) {
        clamp_table(&mut self.network, self.snapshot.nics.len());
        clamp_table(&mut self.storage, self.snapshot.disks.len());
        clamp_table(&mut self.gpu, self.snapshot.gpus.len());
        clamp_table(&mut self.security, self.snapshot.security.len());
        self.log_scroll = self
            .log_scroll
            .min(self.snapshot.logs.len().saturating_sub(1));
    }

    /// Keyboard dispatch. Pure state transition; the loop acts on the returned [`Action`].
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') if self.dev => Action::Quit,
                _ => Action::None,
            };
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') if self.allow_exit_on_q() => Action::Quit,
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refreshing = true;
                Action::Refresh
            }
            KeyCode::Tab | KeyCode::Right => {
                self.page = self.page.next();
                Action::None
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.page = self.page.prev();
                Action::None
            }
            KeyCode::Char(c @ '1'..='7') => {
                self.page = Page::ALL
                    .into_iter()
                    .find(|p| p.hotkey() == c)
                    .unwrap_or_default();
                Action::None
            }
            // `h` stays "open help": it predates the vim-style row keys.
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::F(1) => {
                self.page = Page::Help;
                Action::None
            }
            KeyCode::Esc => {
                self.page = Page::Overview;
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.step(1);
                Action::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.step(-1);
                Action::None
            }
            KeyCode::PageDown => {
                self.step(PAGE_STEP as isize);
                Action::None
            }
            KeyCode::PageUp => {
                self.step(-(PAGE_STEP as isize));
                Action::None
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.jump(true);
                Action::None
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.jump(false);
                Action::None
            }
            _ => Action::None,
        }
    }

    /// Move the current page's cursor by `delta` rows. Single steps wrap; pages clamp.
    fn step(&mut self, delta: isize) {
        match self.page {
            Page::Network => step_table(&mut self.network, self.snapshot.nics.len(), delta),
            Page::Storage => step_table(&mut self.storage, self.snapshot.disks.len(), delta),
            Page::Gpu => step_table(&mut self.gpu, self.snapshot.gpus.len(), delta),
            Page::Security => step_table(&mut self.security, self.snapshot.security.len(), delta),
            Page::Overview => {
                let max = self.snapshot.logs.len().saturating_sub(1);
                // Up (negative) scrolls back in time, which increases the offset.
                let next = self.log_scroll as isize - delta;
                self.log_scroll = next.clamp(0, max as isize) as usize;
            }
            Page::Diagnostics | Page::Help => {}
        }
    }

    fn jump(&mut self, first: bool) {
        match self.page {
            Page::Network => jump_table(&mut self.network, self.snapshot.nics.len(), first),
            Page::Storage => jump_table(&mut self.storage, self.snapshot.disks.len(), first),
            Page::Gpu => jump_table(&mut self.gpu, self.snapshot.gpus.len(), first),
            Page::Security => jump_table(&mut self.security, self.snapshot.security.len(), first),
            Page::Overview => {
                self.log_scroll = if first {
                    self.snapshot.logs.len().saturating_sub(1)
                } else {
                    0
                };
            }
            Page::Diagnostics | Page::Help => {}
        }
    }
}

fn clamp_table(t: &mut TableState, len: usize) {
    if len == 0 {
        t.select(None);
    } else {
        let cur = t.selected().unwrap_or(0).min(len - 1);
        t.select(Some(cur));
    }
}

fn step_table(t: &mut TableState, len: usize, delta: isize) {
    if len == 0 {
        t.select(None);
        return;
    }
    let cur = t.selected().unwrap_or(0) as isize;
    let n = len as isize;
    let next = if delta.abs() == 1 {
        (cur + delta).rem_euclid(n)
    } else {
        (cur + delta).clamp(0, n - 1)
    };
    t.select(Some(next as usize));
}

fn jump_table(t: &mut TableState, len: usize, first: bool) {
    if len == 0 {
        t.select(None);
    } else if first {
        t.select(Some(0));
    } else {
        t.select(Some(len - 1));
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, AppState, Page, Snapshot};
    use crate::inventory::logs::LogLine;
    use crate::inventory::network::Nic;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn with_rows() -> AppState {
        let snapshot = Snapshot {
            nics: (0..3)
                .map(|i| Nic {
                    name: format!("eth{i}"),
                    ..Default::default()
                })
                .collect(),
            logs: (0..30)
                .map(|i| LogLine {
                    text: format!("line {i}"),
                })
                .collect(),
            ..Snapshot::default()
        };
        AppState::new(true, snapshot)
    }

    #[test]
    fn page_tabs_wrap() {
        assert_eq!(Page::Overview.next(), Page::Network);
        assert_eq!(Page::Help.next(), Page::Overview);
        assert_eq!(Page::Overview.prev(), Page::Help);
    }

    #[test]
    fn production_mode_disallows_q_exit() {
        let mut s = AppState::new(false, Snapshot::default());
        assert!(!s.allow_exit_on_q());
        assert_eq!(s.handle_key(key(KeyCode::Char('q'))), Action::None);
        assert_eq!(
            s.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Action::None
        );
    }

    #[test]
    fn dev_mode_allows_q_exit() {
        let mut s = AppState::new(true, Snapshot::default());
        assert!(s.allow_exit_on_q());
        assert_eq!(s.handle_key(key(KeyCode::Char('q'))), Action::Quit);
        assert_eq!(
            s.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Action::Quit
        );
    }

    #[test]
    fn number_keys_and_tab_switch_pages() {
        let mut s = with_rows();
        s.handle_key(key(KeyCode::Char('3')));
        assert_eq!(s.page, Page::Storage);
        s.handle_key(key(KeyCode::Char('4')));
        assert_eq!(s.page, Page::Gpu);
        s.handle_key(key(KeyCode::Char('5')));
        assert_eq!(s.page, Page::Security);
        s.handle_key(key(KeyCode::Tab));
        assert_eq!(s.page, Page::Diagnostics);
        s.handle_key(key(KeyCode::BackTab));
        assert_eq!(s.page, Page::Security);
        s.handle_key(key(KeyCode::Char('h')));
        assert_eq!(s.page, Page::Help);
        s.handle_key(key(KeyCode::Char('7')));
        assert_eq!(s.page, Page::Help);
        s.handle_key(key(KeyCode::Esc));
        assert_eq!(s.page, Page::Overview);
    }

    #[test]
    fn table_selection_wraps_and_pages_clamp() {
        let mut s = with_rows();
        s.page = Page::Network;
        assert_eq!(s.network_sel(), 0);
        s.handle_key(key(KeyCode::Up));
        assert_eq!(s.network_sel(), 2, "single step wraps");
        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.network_sel(), 0);
        s.handle_key(key(KeyCode::PageDown));
        assert_eq!(s.network_sel(), 2, "page step clamps");
        s.handle_key(key(KeyCode::Home));
        assert_eq!(s.network_sel(), 0);
        s.handle_key(key(KeyCode::End));
        assert_eq!(s.network_sel(), 2);
    }

    #[test]
    fn empty_table_has_no_selection() {
        let mut s = AppState::new(true, Snapshot::default());
        s.page = Page::Storage;
        s.handle_key(key(KeyCode::Down));
        assert_eq!(s.storage.selected(), None);
    }

    #[test]
    fn overview_scrolls_the_log() {
        let mut s = with_rows();
        assert_eq!(s.log_scroll, 0);
        s.handle_key(key(KeyCode::Up));
        assert_eq!(s.log_scroll, 1);
        s.handle_key(key(KeyCode::PageUp));
        assert_eq!(s.log_scroll, 11);
        s.handle_key(key(KeyCode::Char('g')));
        assert_eq!(s.log_scroll, 29);
        s.handle_key(key(KeyCode::Down));
        assert_eq!(s.log_scroll, 28);
        s.handle_key(key(KeyCode::Char('G')));
        assert_eq!(s.log_scroll, 0);
    }

    #[test]
    fn refresh_sets_flag_until_snapshot_arrives() {
        let mut s = with_rows();
        assert_eq!(s.handle_key(key(KeyCode::Char('r'))), Action::Refresh);
        assert!(s.refreshing);
        s.network.select(Some(2));
        s.apply_snapshot(Snapshot::default(), "20:00:00".into());
        assert!(!s.refreshing);
        assert_eq!(s.last_refresh.as_deref(), Some("20:00:00"));
        assert_eq!(s.network.selected(), None, "selection clamps to new rows");
    }
}
