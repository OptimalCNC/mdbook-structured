use std::path::{Path, PathBuf};
use std::str::FromStr;

use assert_cmd::Command;
use mdbook_core::book::{Book, Chapter};
use mdbook_preprocessor::{PreprocessorContext, config::Config};
use mdbook_structured::install_assets;
use toml_edit::DocumentMut;

const BOOK_TOML: &[u8] = b"sentinel = \"unchanged\"\n";
const CSS_FILE: &str = "mdbook-structured.css";
const JAVASCRIPT_FILE: &str = "mdbook-structured.js";

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
    ] {
        command(arguments)
            .write_stdin("{")
            .assert()
            .failure()
            .stdout("");
    }
}

#[test]
fn install_omitted_directory_uses_current_book_root() {
    let temp = book_directory();
    let before = book_toml_bytes(temp.path());

    let assertion = command(&["install"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr("");

    assert!(temp.path().join(CSS_FILE).is_file());
    assert!(temp.path().join(JAVASCRIPT_FILE).is_file());
    assert_registration_paths(&assertion.get_output().stdout, CSS_FILE, JAVASCRIPT_FILE);
    assert_book_toml_unchanged(temp.path(), &before);
}

#[test]
fn install_relative_directory_quotes_are_toml_encoded() {
    let temp = book_directory();
    let destination_name = "theme with \"quote\"";
    let destination = temp.path().join(destination_name);
    std::fs::create_dir(&destination).unwrap();
    let before = book_toml_bytes(temp.path());

    let assertion = command(&["install", destination_name])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr("");

    assert!(destination.join(CSS_FILE).is_file());
    assert!(destination.join(JAVASCRIPT_FILE).is_file());
    assert_registration_paths(
        &assertion.get_output().stdout,
        &format!("{destination_name}/{CSS_FILE}"),
        &format!("{destination_name}/{JAVASCRIPT_FILE}"),
    );
    assert_book_toml_unchanged(temp.path(), &before);
}

#[test]
fn install_absolute_in_root_directory_emits_book_relative_paths() {
    let temp = book_directory();
    let destination = temp.path().join("theme");
    std::fs::create_dir(&destination).unwrap();
    let destination = destination.canonicalize().unwrap();
    let before = book_toml_bytes(temp.path());

    let assertion = Command::cargo_bin("mdbook-structured")
        .unwrap()
        .args(["install"])
        .arg(&destination)
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr("");

    assert!(destination.join(CSS_FILE).is_file());
    assert!(destination.join(JAVASCRIPT_FILE).is_file());
    assert_registration_paths(
        &assertion.get_output().stdout,
        &format!("theme/{CSS_FILE}"),
        &format!("theme/{JAVASCRIPT_FILE}"),
    );
    assert_book_toml_unchanged(temp.path(), &before);
}

#[test]
fn install_absolute_outside_root_is_configuration_error_before_writes() {
    let temp = book_directory();
    let outside = tempfile::tempdir().unwrap();
    let destination = outside.path().canonicalize().unwrap();
    let before = book_toml_bytes(temp.path());

    let assertion = Command::cargo_bin("mdbook-structured")
        .unwrap()
        .args(["install"])
        .arg(&destination)
        .current_dir(temp.path())
        .assert()
        .failure();

    assert!(assertion.get_output().stdout.is_empty());
    assert!(String::from_utf8_lossy(&assertion.get_output().stderr).contains("Configuration"));
    assert!(!destination.join(CSS_FILE).exists());
    assert!(!destination.join(JAVASCRIPT_FILE).exists());
    assert_book_toml_unchanged(temp.path(), &before);
}

#[test]
fn install_javascript_conflict_preserves_canonical_css_and_prints_no_instructions() {
    let temp = book_directory();
    let before_install = book_toml_bytes(temp.path());
    install_assets(temp.path()).unwrap();
    assert_book_toml_unchanged(temp.path(), &before_install);
    let css_path = temp.path().join(CSS_FILE);
    let canonical_css = std::fs::read(&css_path).unwrap();
    let javascript_path = temp.path().join(JAVASCRIPT_FILE);
    std::fs::write(&javascript_path, b"different js\n").unwrap();
    let before_command = book_toml_bytes(temp.path());

    let assertion = command(&["install"])
        .current_dir(temp.path())
        .assert()
        .failure();

    let output = assertion.get_output();
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("InstallConflict"));
    assert!(stderr.contains(javascript_path.to_str().unwrap()));
    assert_eq!(std::fs::read(css_path).unwrap(), canonical_css);
    assert_eq!(std::fs::read(javascript_path).unwrap(), b"different js\n");
    assert_book_toml_unchanged(temp.path(), &before_command);
}

#[test]
fn install_missing_and_extra_command_arguments_fail_without_stdout() {
    for arguments in [&[][..], &["install", "theme", "extra"][..]] {
        let temp = book_directory();
        let before = book_toml_bytes(temp.path());

        command(arguments)
            .current_dir(temp.path())
            .assert()
            .failure()
            .stdout("");

        assert!(!temp.path().join(CSS_FILE).exists());
        assert!(!temp.path().join(JAVASCRIPT_FILE).exists());
        assert_book_toml_unchanged(temp.path(), &before);
    }
}

#[cfg(unix)]
#[test]
fn install_non_utf_inside_root_is_configuration_error_before_writes() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp = book_directory();
    let destination = temp.path().join(OsString::from_vec(b"theme-\xff".to_vec()));
    std::fs::create_dir(&destination).unwrap();
    let before = book_toml_bytes(temp.path());

    let assertion = Command::cargo_bin("mdbook-structured")
        .unwrap()
        .args(["install"])
        .arg(&destination)
        .current_dir(temp.path())
        .assert()
        .failure();

    assert!(assertion.get_output().stdout.is_empty());
    assert!(String::from_utf8_lossy(&assertion.get_output().stderr).contains("Configuration"));
    assert!(!destination.join(CSS_FILE).exists());
    assert!(!destination.join(JAVASCRIPT_FILE).exists());
    assert_book_toml_unchanged(temp.path(), &before);
}

fn book_directory() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("book.toml"), BOOK_TOML).unwrap();
    temp
}

fn book_toml_bytes(book_root: &Path) -> Vec<u8> {
    std::fs::read(book_root.join("book.toml")).unwrap()
}

fn assert_book_toml_unchanged(book_root: &Path, before: &[u8]) {
    assert_eq!(book_toml_bytes(book_root), before);
}

fn assert_registration_paths(stdout: &[u8], css: &str, javascript: &str) {
    let output = std::str::from_utf8(stdout).unwrap();
    assert!(output.contains("append"));
    let document = output.parse::<DocumentMut>().unwrap();
    assert_eq!(
        document["output"]["html"]["additional-css"]
            .as_array()
            .unwrap()
            .get(0)
            .and_then(|value| value.as_str()),
        Some(css)
    );
    assert_eq!(
        document["output"]["html"]["additional-js"]
            .as_array()
            .unwrap()
            .get(0)
            .and_then(|value| value.as_str()),
        Some(javascript)
    );
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
