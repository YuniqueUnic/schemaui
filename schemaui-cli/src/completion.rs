use std::path::Path;

use clap_complete::{Shell, generate};
use color_eyre::eyre::Result;

use crate::cli::{CompletionCommand, CompletionShell, command_info};

pub fn render_script(shell: CompletionShell) -> String {
    let command_name = command_name();
    let mut command = command_info();
    let mut bytes = Vec::new();

    match shell {
        CompletionShell::Bash => generate(Shell::Bash, &mut command, command_name, &mut bytes),
        CompletionShell::Zsh => generate(Shell::Zsh, &mut command, command_name, &mut bytes),
        CompletionShell::Fish => generate(Shell::Fish, &mut command, command_name, &mut bytes),
        CompletionShell::PowerShell => {
            generate(Shell::PowerShell, &mut command, command_name, &mut bytes)
        }
    }

    String::from_utf8(bytes).expect("completion script should be utf-8")
}

pub fn run_cli(args: CompletionCommand) -> Result<()> {
    print!("{}", render_script(args.shell));
    Ok(())
}

fn command_name() -> String {
    std::env::args()
        .next()
        .and_then(|arg0| {
            Path::new(&arg0)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "schemaui".to_string())
}
