//! kcore appliance console (Ratatui). `--product` selects hypervisor or vault.

pub mod app;
pub mod inventory;
pub mod product;
pub mod theme;
pub mod ui;

mod tty;

use std::io::{self, stdout};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::{Action, AppState};
use crate::inventory::load_snapshot;
use crate::product::Product;

/// Seconds between automatic inventory refreshes.
pub const REFRESH_SECS: u64 = 5;

/// CLI options (also used from tests).
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub dev: bool,
    /// When set, make this path the controlling TTY (dup2 on Unix).
    pub tty: Option<String>,
    /// `None` reads `/etc/kcore-type` (`kcore-vault` selects the backup appliance).
    pub product: Option<Product>,
}

/// Main event loop. Returns `Ok(())` on clean exit (dev only) or `Err` on I/O errors.
pub fn run(opts: Options) -> io::Result<()> {
    if let Some(ref p) = opts.tty {
        tty::attach_tty(p)?;
    }

    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide)?;

    install_panic_hook();

    if !opts.dev {
        let _ = ctrlc::set_handler(|| {
            // production: ignore SIGINT
        });
    }

    let (product, _source) = Product::resolve(opts.product, Path::new(crate::product::TYPE_FILE));

    let (refresh_tx, refresh_rx) = mpsc::channel::<()>();
    let (snapshot_tx, snapshot_rx) = mpsc::channel();
    let _inventory_worker = std::thread::spawn(move || loop {
        let snapshot = load_snapshot(product);
        if snapshot_tx.send(snapshot).is_err() {
            break;
        }

        match refresh_rx.recv_timeout(Duration::from_secs(REFRESH_SECS)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    });

    let mut app = AppState::new(opts.dev, crate::inventory::Snapshot::default());
    // Never print diagnostics onto the managed TTY after attach — they overwrite the TUI.
    if opts.dev && opts.tty.is_none() {
        eprintln!("[kcore-console] dev mode: q quits, Ctrl+C exits (best effort)");
    }

    // Kernel warnings on tty1 scroll the framebuffer and smear the TUI.
    let _quiet_console = opts.tty.as_ref().map(|_| ConsoleQuiet::engage());

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    terminal.clear()?;
    loop {
        while let Ok(snapshot) = snapshot_rx.try_recv() {
            let now = chrono::Local::now().format("%H:%M:%S").to_string();
            app.apply_snapshot(snapshot, now);
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                // SIGWINCH / terminal resize: next loop iteration redraws with new area.
                Event::Resize(_, _) => {}
                Event::Key(key) if key.kind == KeyEventKind::Press => match app.handle_key(key) {
                    Action::Quit => break,
                    Action::Refresh => {
                        let _ = refresh_tx.send(());
                    }
                    Action::None => {}
                },
                _ => {}
            }
        }
    }

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    disable_raw_mode()?;
    Ok(())
}

/// Drops the kernel console loglevel to 1 while the appliance TUI owns the VT.
struct ConsoleQuiet {
    previous: Option<String>,
}

impl ConsoleQuiet {
    fn engage() -> Self {
        let path = "/proc/sys/kernel/printk";
        let previous = std::fs::read_to_string(path).ok();
        if let Some(prev) = &previous {
            let mut parts = prev.split_whitespace();
            if let (Some(_), Some(b), Some(c), Some(d)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            {
                let _ = std::fs::write(path, format!("1 {b} {c} {d}\n"));
            }
        }
        Self { previous }
    }
}

impl Drop for ConsoleQuiet {
    fn drop(&mut self) {
        if let Some(prev) = self.previous.take() {
            let _ = std::fs::write("/proc/sys/kernel/printk", prev);
        }
    }
}

fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stderr(), LeaveAlternateScreen, crossterm::cursor::Show);
        prev(info);
    }));
}
