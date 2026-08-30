use std::process::ExitCode;

fn main() -> ExitCode {
    mdbook_structured::run_with_io(
        std::env::args_os(),
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    )
}
