use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use mdbook_core::book::Book;
use mdbook_preprocessor::PreprocessorContext;
use mdbook_structured_core::PathSegment;
use toml_edit::{Array, DocumentMut, Item, Table, Value};

use crate::cli::Phase;
use crate::{
    AppDiagnostic, AppDiagnosticKind, HtmlPreprocessorContext, RewriteOptions, install_assets,
    render_book, rewrite_book_links,
};

const CSS_FILE: &str = "mdbook-structured.css";
const JAVASCRIPT_FILE: &str = "mdbook-structured.js";

struct BookRelativeRegistrationPrefix(Box<str>);

struct BookRelativeRegistrationPath(Box<str>);

struct ResolvedInstallDestination {
    filesystem_path: PathBuf,
    registration_prefix: BookRelativeRegistrationPrefix,
}

impl ResolvedInstallDestination {
    fn resolve(current_dir: &Path, directory: Option<&Path>) -> Result<Self, AppDiagnostic> {
        let book_root = current_dir.canonicalize().map_err(|error| {
            configuration_error(format!(
                "failed to resolve the current book root {}: {error}",
                current_dir.display()
            ))
        })?;
        let unresolved_destination = match directory {
            Some(directory) if directory.is_absolute() => directory.to_path_buf(),
            Some(directory) => book_root.join(directory),
            None => book_root.clone(),
        };
        let filesystem_path = unresolved_destination.canonicalize().map_err(|error| {
            configuration_error(format!(
                "failed to resolve install destination {}: {error}",
                unresolved_destination.display()
            ))
        })?;
        let stripped = filesystem_path.strip_prefix(&book_root).map_err(|_| {
            configuration_error(format!(
                "install destination {} is outside the current book root {}",
                filesystem_path.display(),
                book_root.display()
            ))
        })?;
        let registration_prefix = BookRelativeRegistrationPrefix::from_stripped_path(stripped)?;

        Ok(Self {
            filesystem_path,
            registration_prefix,
        })
    }
}

impl BookRelativeRegistrationPrefix {
    fn from_stripped_path(path: &Path) -> Result<Self, AppDiagnostic> {
        let mut components = Vec::new();
        for component in path.components() {
            let Component::Normal(component) = component else {
                return Err(configuration_error(format!(
                    "install registration path contains a non-normal component: {}",
                    path.display()
                )));
            };
            let component = component.to_str().ok_or_else(|| {
                configuration_error("install registration path is not valid Unicode".to_owned())
            })?;
            components.push(component);
        }
        Ok(Self(components.join("/").into_boxed_str()))
    }

    fn join_asset(&self, file_name: &'static str) -> BookRelativeRegistrationPath {
        if self.0.is_empty() {
            BookRelativeRegistrationPath(file_name.into())
        } else {
            BookRelativeRegistrationPath(format!("{}/{file_name}", self.0).into_boxed_str())
        }
    }
}

impl BookRelativeRegistrationPath {
    fn as_str(&self) -> &str {
        &self.0
    }
}

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

pub(crate) fn run_install(
    current_dir: &Path,
    directory: Option<&Path>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let result = (|| -> Result<(), AppDiagnostic> {
        let destination = ResolvedInstallDestination::resolve(current_dir, directory)?;
        install_assets(&destination.filesystem_path)?;
        let css = destination.registration_prefix.join_asset(CSS_FILE);
        let javascript = destination.registration_prefix.join_asset(JAVASCRIPT_FILE);
        write_install_instructions(stdout, &css, &javascript)
    })();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => write_app_diagnostic(stderr, &error),
    }
}

fn write_install_instructions(
    stdout: &mut dyn Write,
    css: &BookRelativeRegistrationPath,
    javascript: &BookRelativeRegistrationPath,
) -> Result<(), AppDiagnostic> {
    let mut document = DocumentMut::new();
    let mut output = Table::new();
    let mut html = Table::new();
    html["additional-css"] = Item::Value(Value::Array(single_value_array(css.as_str())));
    html["additional-js"] = Item::Value(Value::Array(single_value_array(javascript.as_str())));
    output["html"] = Item::Table(html);
    document["output"] = Item::Table(output);

    let instructions = format!(
        "# append these paths to existing arrays; do not replace existing entries\n{document}"
    );
    stdout.write_all(instructions.as_bytes()).map_err(|error| {
        AppDiagnostic::from_kind(
            AppDiagnosticKind::Io { path: None },
            format!("failed to write install instructions: {error}"),
        )
    })
}

fn single_value_array(value: &str) -> Array {
    let mut array = Array::new();
    array.push(value);
    array
}

fn configuration_error(detail: String) -> AppDiagnostic {
    AppDiagnostic::from_kind(AppDiagnosticKind::Configuration { key: None }, detail)
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
