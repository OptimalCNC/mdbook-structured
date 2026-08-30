use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::protocol::write_diagnostic;

enum Command {
    Run(Phase),
    Supports { phase: Phase, renderer: String },
    Install { directory: Option<PathBuf> },
}

pub(crate) enum Phase {
    Render,
    RewriteLinks,
}

pub fn run_with_io(
    args: impl IntoIterator<Item = OsString>,
    current_dir: &Path,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    match parse_command(args) {
        Ok(Command::Supports { phase, renderer }) if renderer == "html" => {
            let _ = phase;
            ExitCode::SUCCESS
        }
        Ok(Command::Supports { renderer, .. }) => write_diagnostic(
            stderr,
            "UnsupportedRenderer",
            &format!("mdbook-structured supports only the html renderer, not {renderer}"),
        ),
        Ok(Command::Run(phase)) => crate::protocol::run_phase(phase, stdin, stdout, stderr),
        Ok(Command::Install { directory }) => {
            crate::protocol::run_install(current_dir, directory.as_deref(), stdout, stderr)
        }
        Err(message) => write_diagnostic(stderr, "Protocol", &message),
    }
}

fn parse_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut arguments = args.into_iter();
    let _program = arguments.next();
    let arguments: Vec<_> = arguments.collect();

    match arguments.as_slice() {
        [command] if command == "install" => Ok(Command::Install { directory: None }),
        [command, directory] if command == "install" => Ok(Command::Install {
            directory: Some(PathBuf::from(directory)),
        }),
        [phase] => Ok(Command::Run(parse_phase(phase)?)),
        [phase, supports, renderer] if supports == "supports" => Ok(Command::Supports {
            phase: parse_phase(phase)?,
            renderer: renderer
                .to_str()
                .ok_or_else(|| "renderer name is not valid Unicode".to_owned())?
                .to_owned(),
        }),
        _ => Err(
            "expected render or rewrite-links with optional supports <renderer>, or install [DIR]"
                .to_owned(),
        ),
    }
}

fn parse_phase(value: &OsString) -> Result<Phase, String> {
    match value.to_str() {
        Some("render") => Ok(Phase::Render),
        Some("rewrite-links") => Ok(Phase::RewriteLinks),
        Some(other) => Err(format!("unknown preprocessor phase {other}")),
        None => Err("preprocessor phase is not valid Unicode".to_owned()),
    }
}
