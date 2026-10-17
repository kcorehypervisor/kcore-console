//! kcore appliance TUI (Ratatui / crossterm).

use clap::Parser;

use kcore_console::product::Product;

/// kcore local appliance console: read-only status, no local shell.
#[derive(Parser, Debug)]
#[command(
    name = "kcore-console",
    version,
    about = "kcore appliance console (hypervisor or vault)",
    long_about = None
)]
struct Cli {
    /// Development: allow q / Ctrl+C to exit; print extra diagnostics to stderr.
    #[arg(long)]
    dev: bool,
    /// Attach to this TTY (dup2 stdin/out/err). Typical: /dev/tty1 under systemd.
    #[arg(long, value_name = "PATH")]
    tty: Option<String>,
    /// Appliance this screen describes. Omit to read `/etc/kcore-type`
    /// (`kcore-vault` selects the backup appliance; anything else is a hypervisor node).
    #[arg(long, value_enum, env = "KCORE_CONSOLE_PRODUCT")]
    product: Option<Product>,
}

fn main() {
    let cli = Cli::parse();
    let opts = kcore_console::Options {
        dev: cli.dev,
        tty: cli.tty,
        product: cli.product,
    };
    if let Err(e) = kcore_console::run(opts) {
        eprintln!("kcore-console: {e}");
        std::process::exit(1);
    }
}
