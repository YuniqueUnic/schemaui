use std::{
    collections::hash_map::RandomState,
    env,
    hash::BuildHasher,
    io::{self, IsTerminal, Write},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use super::super::options::{ImageCellSize, ImageProtocol};

// Like Crossterm's raw-mode state, this belongs to the active terminal session.
// The panic hook runs before TerminalGuard is dropped, so it needs the same lease.
static KITTY_IMAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub(crate) struct GraphicsOutput {
    pub protocol: ImageProtocol,
    pub is_tmux: bool,
    pub kitty_id: u32,
    cell_size: Option<ImageCellSize>,
}

impl GraphicsOutput {
    pub fn detect(
        requested: ImageProtocol,
        cell_size: Option<ImageCellSize>,
    ) -> Result<Self, String> {
        if requested == ImageProtocol::Source {
            return Err("Image preview is disabled; showing source.".into());
        }
        if !io::stderr().is_terminal() {
            return Err(
                "Native image preview requires a terminal on stderr; showing source.".into(),
            );
        }
        let term = env::var("TERM").unwrap_or_default();
        if term == "dumb" {
            return Err("Native image preview is disabled for TERM=dumb; showing source.".into());
        }
        let program = env::var("TERM_PROGRAM").unwrap_or_default();
        let is_tmux =
            env::var_os("TMUX").is_some() || program == "tmux" || term.starts_with("tmux");
        let is_screen = env::var_os("STY").is_some() || term.starts_with("screen");
        if is_screen && !is_tmux {
            return Err(
                "Native image passthrough through screen is not supported; showing source.".into(),
            );
        }
        let protocol = if requested != ImageProtocol::Auto {
            requested
        } else if is_tmux || is_screen {
            return Err(
                "Native images through tmux/screen were not confirmed; showing source. Passthrough settings were not changed. Select --image-protocol only if your existing setup supports it."
                    .into(),
            );
        } else {
            select_protocol(&program, &term)?
        };
        if is_tmux && protocol == ImageProtocol::Sixel {
            return Err("Sixel passthrough through tmux is not supported; showing source.".into());
        }
        let kitty_id = if protocol == ImageProtocol::Kitty {
            (RandomState::new().hash_one((std::process::id(), Instant::now())) as u32).max(1)
        } else {
            0
        };
        Ok(Self {
            protocol,
            is_tmux,
            kitty_id,
            cell_size,
        })
    }

    pub fn cell_size(self) -> Result<ImageCellSize, String> {
        if let Some(size) = self.cell_size {
            return Ok(size);
        }
        let size = crossterm::terminal::window_size().ok();
        size.and_then(|size| {
            if size.columns == 0
                || size.rows == 0
                || size.width % size.columns != 0
                || size.height % size.rows != 0
            {
                return None;
            }
            ImageCellSize::new(size.width / size.columns, size.height / size.rows)
        })
        .ok_or_else(|| {
            "Terminal cell pixels are unavailable or not exact; showing source. Specify --image-cell-size WIDTHxHEIGHT using the actual cell pixels."
                .into()
        })
    }

    pub fn arm_cleanup(self) {
        if self.protocol == ImageProtocol::Kitty {
            let lease = u64::from(self.kitty_id) | (u64::from(self.is_tmux) << 32);
            KITTY_IMAGE.store(lease, Ordering::SeqCst);
        }
    }
}

fn select_protocol(program: &str, term: &str) -> Result<ImageProtocol, String> {
    let unconfirmed = || {
        "Could not automatically identify a native image protocol; showing source. Use --image-protocol kitty|iterm2|sixel if your terminal supports it."
            .to_string()
    };
    let program_protocol = match program {
        "" => None,
        "ghostty" | "kitty" => Some(ImageProtocol::Kitty),
        "iTerm.app" | "WezTerm" | "rio" => Some(ImageProtocol::Iterm2),
        _ => return Err(unconfirmed()),
    };
    let term_protocol = match term {
        "xterm-kitty" | "xterm-ghostty" => Some(ImageProtocol::Kitty),
        "wezterm" => Some(ImageProtocol::Iterm2),
        "foot" | "foot-extra" | "foot-direct" | "mlterm" | "mlterm-256color" => {
            Some(ImageProtocol::Sixel)
        }
        _ => None,
    };
    let Some(protocol) = program_protocol.or(term_protocol) else {
        return Err(unconfirmed());
    };
    let nonempty = |name| env::var_os(name).is_some_and(|value| !value.is_empty());
    let conflicting = program_protocol.is_some_and(|value| value != protocol)
        || term_protocol.is_some_and(|value| value != protocol)
        || (nonempty("KITTY_WINDOW_ID") && protocol != ImageProtocol::Kitty)
        || (nonempty("ITERM_SESSION_ID") && protocol != ImageProtocol::Iterm2);
    if conflicting {
        return Err(
            "Terminal image hints conflict; showing source. Select --image-protocol explicitly if your terminal supports it."
                .into(),
        );
    }
    Ok(protocol)
}

pub(super) fn clear_image(writer: &mut impl Write) -> io::Result<()> {
    delete_image(writer, KITTY_IMAGE.load(Ordering::SeqCst))
}

pub(super) fn restore() {
    let _ = delete_image(
        &mut io::stderr().lock(),
        KITTY_IMAGE.swap(0, Ordering::SeqCst),
    );
}

fn delete_image(writer: &mut impl Write, lease: u64) -> io::Result<()> {
    if lease == 0 {
        return Ok(());
    }
    let id = lease as u32;
    let command = format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\");
    if lease >> 32 != 0 {
        write!(
            writer,
            "\x1bPtmux;{}\x1b\\",
            command.replace('\x1b', "\x1b\x1b")
        )?;
    } else {
        writer.write_all(command.as_bytes())?;
    }
    writer.flush()
}
