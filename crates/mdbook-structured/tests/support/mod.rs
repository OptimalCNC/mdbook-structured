use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use mdbook_structured::{InstallState, install_assets};
use scraper::{ElementRef, Html, Selector};
use toml_edit::{DocumentMut, value};

pub struct BuiltFixture {
    _temp_book: tempfile::TempDir,
    output_root: PathBuf,
}

impl BuiltFixture {
    pub fn output_root(&self) -> &Path {
        &self.output_root
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservedNodeKind {
    Mapping,
    Sequence,
    String,
    Number,
    Boolean,
    Null,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedContainer {
    pub kind: ObservedNodeKind,
    pub open: bool,
    pub immediate_child_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedLink {
    pub text: String,
    pub href: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedBuiltPage {
    pub heading: String,
    pub root_kind: ObservedNodeKind,
    pub node_kinds: Vec<ObservedNodeKind>,
    pub scalar_texts: Vec<String>,
    pub containers: Vec<ObservedContainer>,
    pub original_summary: String,
    pub original_format: String,
    pub original_source: String,
}

pub fn build_integration_fixture() -> Result<BuiltFixture, Box<dyn Error>> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/integration-book");
    let temp_book = tempfile::tempdir()?;
    copy_tree(&fixture, temp_book.path())?;

    let binary = Path::new(env!("CARGO_BIN_EXE_mdbook-structured"));
    let binary_text = binary
        .to_str()
        .ok_or_else(|| helper_error("mdbook-structured binary path is not UTF-8"))?;
    let quoted_binary = shlex::try_quote(binary_text)?;
    let render_command = format!("{quoted_binary} render");
    let rewrite_command = format!("{quoted_binary} rewrite-links");
    assert_eq!(
        shlex::split(&render_command),
        Some(vec![binary_text.to_owned(), "render".to_owned()]),
    );
    assert_eq!(
        shlex::split(&rewrite_command),
        Some(vec![binary_text.to_owned(), "rewrite-links".to_owned()]),
    );

    let book_toml = temp_book.path().join("book.toml");
    let mut configuration = fs::read_to_string(&book_toml)?.parse::<DocumentMut>()?;
    configuration["preprocessor"]["structured"]["command"] = value(render_command);
    configuration["preprocessor"]["structured-links"]["command"] = value(rewrite_command);
    fs::write(&book_toml, configuration.to_string())?;

    let installed = install_assets(temp_book.path())?;
    if installed.css() != InstallState::Created || installed.javascript() != InstallState::Created {
        return Err(helper_error("fresh fixture assets were not created").into());
    }
    for asset in ["mdbook-structured.css", "mdbook-structured.js"] {
        if !temp_book.path().join(asset).is_file() {
            return Err(helper_error(format!("missing installed asset {asset}")).into());
        }
    }

    let build = Command::new("mdbook")
        .arg("build")
        .arg(temp_book.path())
        .output()?;
    if !build.status.success() {
        return Err(helper_error(format!(
            "mdbook build failed with {}\nstdout:\n{}\nstderr:\n{}",
            build.status,
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr),
        ))
        .into());
    }

    let output_root = temp_book.path().join("book");
    Ok(BuiltFixture {
        _temp_book: temp_book,
        output_root,
    })
}

pub fn observe_built_page(path: &Path) -> Result<ObservedBuiltPage, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let document = Html::parse_document(&source);
    let structured_roots = document
        .select(&selector("section.structured-document"))
        .collect::<Vec<_>>();
    let structured_root = require_one(structured_roots, "structured document root")?;

    let headings = structured_root.select(&selector("h1")).collect::<Vec<_>>();
    let heading = element_text(require_one(headings, "structured page heading")?);

    let model_roots = direct_element_children(structured_root)
        .filter(|element| element.value().attr("data-structured-node").is_some())
        .collect::<Vec<_>>();
    let model_root = require_one(model_roots, "structured model root")?;
    let root_kind = observe_kind(model_root)?;

    let nodes = structured_root
        .select(&selector("[data-structured-node]"))
        .collect::<Vec<_>>();
    let node_kinds = nodes
        .iter()
        .copied()
        .map(observe_kind)
        .collect::<Result<Vec<_>, _>>()?;
    let scalar_texts = nodes
        .iter()
        .filter(|node| {
            matches!(
                node.value().attr("data-structured-node"),
                Some("string" | "number" | "boolean" | "null")
            )
        })
        .map(|node| {
            let values = direct_element_children(*node)
                .filter(|child| child.value().attr("data-structured-value").is_some())
                .collect::<Vec<_>>();
            require_one(values, "structured scalar value").map(element_text)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let containers = structured_root
        .select(&selector("[data-structured-container]"))
        .map(|container| {
            Ok(ObservedContainer {
                kind: observe_kind(container)?,
                open: container.value().attr("open").is_some(),
                immediate_child_count: direct_element_children(container)
                    .filter(|child| child.value().attr("data-structured-node").is_some())
                    .count(),
            })
        })
        .collect::<Result<Vec<_>, io::Error>>()?;

    let originals = structured_root
        .select(&selector("details[data-structured-original-source]"))
        .collect::<Vec<_>>();
    let original = require_one(originals, "original source disclosure")?;
    let summaries = direct_element_children(original)
        .filter(|child| child.value().name() == "summary")
        .collect::<Vec<_>>();
    let original_summary = element_text(require_one(summaries, "original source summary")?);
    let codes = original
        .select(&selector("code[data-structured-format]"))
        .collect::<Vec<_>>();
    let code = require_one(codes, "original source code")?;
    let original_format = code
        .value()
        .attr("data-structured-format")
        .ok_or_else(|| helper_error("original source has no format marker"))?
        .to_owned();
    let original_source = element_text(code);

    Ok(ObservedBuiltPage {
        heading,
        root_kind,
        node_kinds,
        scalar_texts,
        containers,
        original_summary,
        original_format,
        original_source,
    })
}

pub fn observe_page_links(path: &Path) -> Result<Vec<ObservedLink>, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let document = Html::parse_document(&source);
    document
        .select(&selector("a"))
        .map(|anchor| {
            let href = anchor
                .value()
                .attr("href")
                .ok_or_else(|| helper_error("page anchor has no href"))?;
            Ok(ObservedLink {
                text: element_text(anchor),
                href: href.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, io::Error>>()
        .map_err(Into::into)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path)?;
        } else {
            return Err(helper_error(format!(
                "fixture contains a symlink or unsupported file type: {}",
                source_path.display(),
            ))
            .into());
        }
    }
    Ok(())
}

fn observe_kind(element: ElementRef<'_>) -> Result<ObservedNodeKind, io::Error> {
    match element.value().attr("data-structured-node") {
        Some("mapping") => Ok(ObservedNodeKind::Mapping),
        Some("sequence") => Ok(ObservedNodeKind::Sequence),
        Some("string") => Ok(ObservedNodeKind::String),
        Some("number") => Ok(ObservedNodeKind::Number),
        Some("boolean") => Ok(ObservedNodeKind::Boolean),
        Some("null") => Ok(ObservedNodeKind::Null),
        Some(unknown) => Err(helper_error(format!(
            "unknown structured node kind: {unknown}"
        ))),
        None => Err(helper_error("structured node has no kind")),
    }
}

fn direct_element_children(element: ElementRef<'_>) -> impl Iterator<Item = ElementRef<'_>> {
    element.children().filter_map(ElementRef::wrap)
}

fn require_one<'a>(elements: Vec<ElementRef<'a>>, role: &str) -> Result<ElementRef<'a>, io::Error> {
    if elements.len() != 1 {
        return Err(helper_error(format!(
            "expected exactly one {role}, found {}",
            elements.len(),
        )));
    }
    Ok(elements[0])
}

fn selector(source: &str) -> Selector {
    Selector::parse(source).expect("static selector must parse")
}

fn element_text(element: ElementRef<'_>) -> String {
    element.text().collect()
}

fn helper_error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}
