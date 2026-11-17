use std::{
    io::{self, IsTerminal, Stderr},
    ops::{Deref, DerefMut},
    sync::Once,
};

use anyhow::{Context, Result};
use crossterm::{
    cursor::Show,
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

#[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
pub(crate) mod graphics;

static PANIC_HOOK: Once = Once::new();

pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stderr>>,
}

impl TerminalGuard {
    pub(crate) fn clear_image(&mut self) -> Result<()> {
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        graphics::clear_image(self.terminal.backend_mut())
            .context("failed to clear terminal image")?;
        Ok(())
    }

    pub fn new() -> Result<Self> {
        anyhow::ensure!(
            io::stderr().is_terminal(),
            "TUI requires an interactive terminal on stderr"
        );
        enable_raw_mode().context("failed to enable raw mode")?;
        install_panic_hook();
        let initialized = (|| {
            let mut stderr = io::stderr();
            execute!(stderr, EnterAlternateScreen, EnableBracketedPaste)
                .context("failed to enter alternate screen")?;
            Terminal::new(CrosstermBackend::new(stderr)).context("failed to initialize terminal")
        })();
        match initialized {
            Ok(terminal) => Ok(Self { terminal }),
            Err(error) => {
                restore_terminal();
                Err(error)
            }
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        restore_terminal();
    }
}

impl Deref for TerminalGuard {
    type Target = Terminal<CrosstermBackend<Stderr>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for TerminalGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            restore_terminal();
            previous(panic_info);
        }));
    });
}

fn restore_terminal() {
    #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
    graphics::restore();
    let _ = disable_raw_mode();
    let mut stderr = io::stderr();
    let _ = execute!(stderr, DisableBracketedPaste, LeaveAlternateScreen, Show);
}
