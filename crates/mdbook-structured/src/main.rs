use std::process::ExitCode;

fn main() -> ExitCode {
    let current_dir = match std::env::current_dir() {
        Ok(current_dir) => current_dir,
        Err(error) => {
            eprintln!("Io: failed to resolve current directory: {error}");
            return ExitCode::FAILURE;
        }
    };
    mdbook_structured::run_with_io(
        std::env::args_os(),
        &current_dir,
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    )
}
