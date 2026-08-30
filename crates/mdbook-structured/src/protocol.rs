use std::io::{Read, Write};
use std::process::ExitCode;

use mdbook_core::book::Book;
use mdbook_preprocessor::PreprocessorContext;
use mdbook_structured_core::PathSegment;

use crate::cli::Phase;
use crate::{
    AppDiagnostic, AppDiagnosticKind, HtmlPreprocessorContext, RewriteOptions, render_book,
    rewrite_book_links,
};

pub(crate) fn run_phase(
    phase: Phase,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let result = (|| -> Result<(), AppDiagnostic> {
        let (context, book) = decode_input(stdin)?;
        let context = HtmlPreprocessorContext::try_from_context(context)?;
        let book = match phase {
            Phase::Render => render_book(&context, book)?,
            Phase::RewriteLinks => {
                rewrite_book_links(RewriteOptions::from_context(&context)?, book)?
            }
        };
        write_book(stdout, &book)
    })();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => write_app_diagnostic(stderr, &error),
    }
}

fn decode_input(stdin: &mut dyn Read) -> Result<(PreprocessorContext, Book), AppDiagnostic> {
    mdbook_preprocessor::parse_input(stdin).map_err(|error| {
        AppDiagnostic::from_kind(
            AppDiagnosticKind::Protocol,
            format!(
                "failed to decode one preprocessor input value; trailing-input is rejected: {error:#}"
            ),
        )
    })
}

fn write_book(stdout: &mut dyn Write, book: &Book) -> Result<(), AppDiagnostic> {
    let mut encoded = Vec::new();
    serde_json::to_writer(&mut encoded, book).map_err(|error| {
        AppDiagnostic::from_kind(
            AppDiagnosticKind::Protocol,
            format!("failed to serialize preprocessor output: {error}"),
        )
    })?;
    encoded.push(b'\n');
    stdout.write_all(&encoded).map_err(|error| {
        AppDiagnostic::from_kind(
            AppDiagnosticKind::Io { path: None },
            format!("failed to write preprocessor output: {error}"),
        )
    })
}

pub(crate) fn write_diagnostic(stderr: &mut dyn Write, category: &str, detail: &str) -> ExitCode {
    let _ = writeln!(stderr, "{category}: {detail}");
    ExitCode::FAILURE
}

fn write_app_diagnostic(stderr: &mut dyn Write, diagnostic: &AppDiagnostic) -> ExitCode {
    let _ = writeln!(stderr, "{}", format_diagnostic(diagnostic));
    ExitCode::FAILURE
}

fn format_diagnostic(diagnostic: &AppDiagnostic) -> String {
    let Some(core) = diagnostic.core_diagnostic() else {
        return format!("{:?}: {}", diagnostic.kind(), diagnostic.detail());
    };

    let mut facts = vec![
        format!("{:?}", core.category()),
        format!("source {}", core.source_name().display()),
    ];
    if let Some(location) = core.location() {
        facts.push(format!(
            "line {}, column {}",
            location.line(),
            location.column()
        ));
    }
    if let Some(path) = core.structured_path() {
        facts.push(format!(
            "structured path {}",
            format_structured_path(path.segments())
        ));
    }
    facts.push(core.detail().to_owned());
    facts.join("; ")
}

fn format_structured_path(segments: &[PathSegment]) -> String {
    segments
        .iter()
        .map(|segment| match segment {
            PathSegment::Key(key) => key.clone(),
            PathSegment::Index(index) => format!("[{index}]"),
        })
        .collect::<Vec<_>>()
        .join(".")
}
