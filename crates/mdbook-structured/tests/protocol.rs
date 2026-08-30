use std::path::{Path, PathBuf};
use std::str::FromStr;

use assert_cmd::Command;
use mdbook_core::book::{Book, Chapter};
use mdbook_preprocessor::{PreprocessorContext, config::Config};

fn command(args: &[&str]) -> Command {
    let mut command = Command::cargo_bin("mdbook-structured").unwrap();
    command.args(args);
    command
}

fn context(renderer: &str, configuration: &str) -> PreprocessorContext {
    PreprocessorContext::new(
        PathBuf::from("book"),
        Config::from_str(configuration).unwrap(),
        renderer.to_owned(),
    )
}

fn chapter(name: &str, content: &str, source_path: &str, logical_path: &str) -> Chapter {
    let mut chapter = Chapter::new(name, content.to_owned(), source_path, Vec::new());
    chapter.path = Some(PathBuf::from(logical_path));
    chapter
}

fn protocol_input(context: PreprocessorContext, book: Book) -> Vec<u8> {
    serde_json::to_vec(&(context, book)).unwrap()
}

fn html_context(configuration: &str) -> PreprocessorContext {
    context("html", configuration)
}

fn default_configuration() -> &'static str {
    "[book]\nsrc = \"chapters\"\n"
}

fn assert_success(args: &[&str], input: Vec<u8>) -> Book {
    let assertion = command(args)
        .write_stdin(input)
        .assert()
        .success()
        .stderr("");
    let output = assertion.get_output();
    assert!(output.stdout.ends_with(b"\n"));
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    serde_json::from_slice(&output.stdout).expect("stdout must contain exactly one Book")
}

fn assert_failure(args: &[&str], input: Vec<u8>, required_facts: &[&str]) {
    let assertion = command(args).write_stdin(input).assert().failure();
    let output = assertion.get_output();
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    for fact in required_facts {
        assert!(
            stderr.contains(fact),
            "missing stderr fact {fact:?}: {stderr}"
        );
    }
}

#[test]
fn capability_render_supports_html() {
    command(&["render", "supports", "html"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn capability_rewrite_links_supports_html() {
    command(&["rewrite-links", "supports", "html"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn capability_rejects_non_html_renderers_without_reading_stdin() {
    command(&["render", "supports", "pdf"])
        .write_stdin("{")
        .assert()
        .failure()
        .stdout("");
}

#[test]
fn capability_rejects_malformed_command_shapes_without_reading_stdin() {
    for arguments in [
        &[][..],
        &["render", "supports"][..],
        &["render", "supports", "html", "extra"][..],
        &["unknown", "supports", "html"][..],
        &["install"][..],
    ] {
        command(arguments)
            .write_stdin("{")
            .assert()
            .failure()
            .stdout("");
    }
}

#[test]
fn normal_render_transforms_received_structured_chapters() {
    let output = assert_success(
        &["render"],
        protocol_input(
            html_context(default_configuration()),
            Book::new_with_items(vec![
                chapter("Guide", "ordinary Markdown", "guide.md", "guide.md").into(),
                chapter("Config", r#"{"enabled":true}"#, "config.json", "config.md").into(),
            ]),
        ),
    );
    let chapters: Vec<_> = output.chapters().collect();

    assert_eq!(chapters[0].name, "Guide");
    assert_eq!(chapters[0].content, "ordinary Markdown");
    assert_eq!(
        chapters[0].source_path.as_deref(),
        Some(Path::new("guide.md"))
    );
    assert_eq!(chapters[0].path.as_deref(), Some(Path::new("guide.md")));
    assert_eq!(chapters[1].name, "Config");
    assert_eq!(
        chapters[1].source_path.as_deref(),
        Some(Path::new("config.json"))
    );
    assert_eq!(
        chapters[1].path.as_deref(),
        Some(Path::new("config.json.md"))
    );
    assert!(chapters[1].content.contains("<h1>Config</h1>"));
    assert!(chapters[1].content.contains("enabled"));
    assert!(chapters[1].content.contains("true"));
}

#[test]
fn normal_rewrite_links_rewrites_received_post_render_book() {
    let output = assert_success(
        &["rewrite-links"],
        protocol_input(
            html_context(default_configuration()),
            Book::new_with_items(vec![
                chapter("Guide", "[Config](config.json)\n", "guide.md", "guide.md").into(),
                chapter(
                    "Config",
                    "<div class=\"structured-document\"></div>",
                    "config.json",
                    "config.json.md",
                )
                .into(),
            ]),
        ),
    );
    let chapters: Vec<_> = output.chapters().collect();

    assert_eq!(chapters[0].content, "[Config](config.json.html)\n");
    assert_eq!(
        chapters[0].source_path.as_deref(),
        Some(Path::new("guide.md"))
    );
    assert_eq!(chapters[0].path.as_deref(), Some(Path::new("guide.md")));
    assert_eq!(
        chapters[1].source_path.as_deref(),
        Some(Path::new("config.json"))
    );
    assert_eq!(
        chapters[1].path.as_deref(),
        Some(Path::new("config.json.md"))
    );
}

#[test]
fn error_malformed_protocol_retains_protocol_category() {
    assert_failure(&["render"], b"{".to_vec(), &["Protocol"]);
}

#[test]
fn error_trailing_json_retains_protocol_trailing_input_context() {
    let mut input = protocol_input(
        html_context(default_configuration()),
        Book::new_with_items(Vec::new()),
    );
    input.extend_from_slice(b"\nnull");

    assert_failure(&["render"], input, &["Protocol", "trailing-input"]);
}

#[test]
fn error_renderer_guard_retains_unsupported_renderer_and_name() {
    assert_failure(
        &["render"],
        protocol_input(
            context("pdf", default_configuration()),
            Book::new_with_items(Vec::new()),
        ),
        &["UnsupportedRenderer", "pdf"],
    );
}

#[test]
fn error_unknown_render_option_retains_configuration_key() {
    assert_failure(
        &["render"],
        protocol_input(
            html_context(
                "[book]\nsrc = \"chapters\"\n[preprocessor.structured]\nunexpected = true\n",
            ),
            Book::new_with_items(Vec::new()),
        ),
        &["Configuration", "unexpected"],
    );
}

#[test]
fn error_malformed_registered_json_retains_core_parse_location() {
    assert_failure(
        &["render"],
        protocol_input(
            html_context(default_configuration()),
            Book::new_with_items(vec![
                chapter("Broken", "{", "broken.json", "broken.json").into(),
            ]),
        ),
        &["Parse", "broken.json", "line 1", "column 2"],
    );
}

#[test]
fn error_duplicate_decoded_key_retains_core_path_and_location() {
    assert_failure(
        &["render"],
        protocol_input(
            html_context(default_configuration()),
            Book::new_with_items(vec![
                chapter(
                    "Duplicate",
                    "{\n  \"outer\": {\n    \"a\": 1,\n    \"\\u0061\": 2\n  }\n}\n",
                    "duplicate.json",
                    "duplicate.json",
                )
                .into(),
            ]),
        ),
        &[
            "DuplicateDecodedKey",
            "duplicate.json",
            "line 4",
            "column 5",
            "outer",
            "a",
        ],
    );
}
