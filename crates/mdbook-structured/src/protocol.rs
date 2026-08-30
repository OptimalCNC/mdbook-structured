use std::io::{Read, Write};
use std::process::ExitCode;

use crate::cli::Phase;

pub(crate) fn run_phase(
    _phase: Phase,
    _stdin: &mut dyn Read,
    _stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    write_diagnostic(
        stderr,
        "Protocol",
        "normal preprocessor dispatch is not implemented",
    )
}

pub(crate) fn write_diagnostic(stderr: &mut dyn Write, category: &str, detail: &str) -> ExitCode {
    let _ = writeln!(stderr, "{category}: {detail}");
    ExitCode::FAILURE
}
