use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use mdbook_structured::{AppDiagnosticKind, InstallState, install_assets};

const BOOK_TOML: &[u8] = b"sentinel = \"unchanged\"\n";
const CSS_FILE: &str = "mdbook-structured.css";
const JAVASCRIPT_FILE: &str = "mdbook-structured.js";

#[test]
fn install_absent_assets_are_created_and_matching_assets_remain_unchanged() {
    let temp = existing_book_directory();
    let destination = temp.path();

    let created = install_without_configuration_mutation(destination, destination).unwrap();
    assert_eq!(created.css(), InstallState::Created);
    assert_eq!(created.javascript(), InstallState::Created);

    let css = fs::read(destination.join(CSS_FILE)).unwrap();
    let javascript = fs::read(destination.join(JAVASCRIPT_FILE)).unwrap();
    assert!(contains(&css, b".structured-document"));
    assert!(contains(&javascript, b"data-structured-action"));
    let css_hash = hash(&css);
    let javascript_hash = hash(&javascript);

    let unchanged = install_without_configuration_mutation(destination, destination).unwrap();
    assert_eq!(unchanged.css(), InstallState::Unchanged);
    assert_eq!(unchanged.javascript(), InstallState::Unchanged);
    assert_eq!(
        hash(&fs::read(destination.join(CSS_FILE)).unwrap()),
        css_hash
    );
    assert_eq!(
        hash(&fs::read(destination.join(JAVASCRIPT_FILE)).unwrap()),
        javascript_hash
    );
}

#[test]
fn install_differing_css_stops_before_javascript_and_preserves_existing_bytes() {
    let temp = existing_book_directory();
    let destination = temp.path();
    let css_path = destination.join(CSS_FILE);
    fs::write(&css_path, b"different css\n").unwrap();

    let error = install_without_configuration_mutation(destination, destination).unwrap_err();
    let AppDiagnosticKind::InstallConflict { path } = error.kind() else {
        panic!("expected InstallConflict, got {error:?}");
    };
    assert_eq!(path.as_path(), css_path.as_path());
    assert_eq!(fs::read(css_path).unwrap(), b"different css\n");
    assert!(!destination.join(JAVASCRIPT_FILE).exists());
}

#[test]
fn install_differing_javascript_preserves_canonical_css_and_existing_javascript() {
    let temp = existing_book_directory();
    let destination = temp.path();
    install_without_configuration_mutation(destination, destination).unwrap();
    let css_path = destination.join(CSS_FILE);
    let canonical_css = fs::read(&css_path).unwrap();
    let javascript_path = destination.join(JAVASCRIPT_FILE);
    fs::write(&javascript_path, b"different js\n").unwrap();

    let error = install_without_configuration_mutation(destination, destination).unwrap_err();
    let AppDiagnosticKind::InstallConflict { path } = error.kind() else {
        panic!("expected InstallConflict, got {error:?}");
    };
    assert_eq!(path.as_path(), javascript_path.as_path());
    assert_eq!(fs::read(css_path).unwrap(), canonical_css);
    assert_eq!(fs::read(javascript_path).unwrap(), b"different js\n");
}

#[test]
fn install_invalid_destination_reports_the_css_path_without_changing_the_parent_file() {
    let temp = existing_book_directory();
    let destination = temp.path().join("not-a-directory");
    fs::write(&destination, b"parent file\n").unwrap();

    let error = install_without_configuration_mutation(temp.path(), &destination).unwrap_err();
    let AppDiagnosticKind::Io { path } = error.kind() else {
        panic!("expected Io, got {error:?}");
    };
    assert_eq!(path.as_deref(), Some(destination.join(CSS_FILE).as_path()));
    assert_eq!(fs::read(destination).unwrap(), b"parent file\n");
}

fn existing_book_directory() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("book.toml"), BOOK_TOML).unwrap();
    temp
}

fn install_without_configuration_mutation(
    book_root: &Path,
    destination: &Path,
) -> Result<mdbook_structured::InstallReport, mdbook_structured::AppDiagnostic> {
    let book_toml = book_root.join("book.toml");
    let before = fs::read(&book_toml).unwrap();
    let result = install_assets(destination);
    assert_eq!(fs::read(book_toml).unwrap(), before);
    result
}

fn hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
