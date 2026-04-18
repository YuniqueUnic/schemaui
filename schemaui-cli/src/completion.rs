use std::path::Path;

use argh_complete::Generator;
use color_eyre::eyre::Result;

use crate::cli::{CompletionCommand, CompletionShell, command_info};

pub fn render_script(shell: CompletionShell) -> String {
    let command_name = command_name();
    let info = command_info();

    match shell {
        CompletionShell::Bash => argh_complete::bash::Bash::generate(&command_name, &info),
        CompletionShell::Zsh => argh_complete::zsh::Zsh::generate(&command_name, &info),
        CompletionShell::Fish => argh_complete::fish::Fish::generate(&command_name, &info),
        CompletionShell::Nushell => argh_complete::nushell::Nushell::generate(&command_name, &info),
    }
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
