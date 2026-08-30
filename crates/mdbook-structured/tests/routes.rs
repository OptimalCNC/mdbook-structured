use std::fs;
use std::path::{Path, PathBuf};

use mdbook_core::book::{Book, BookItem, Chapter};
use mdbook_structured::{
    AppDiagnosticKind, MdBookPathHazardFact, StructuredExtension, StructuredSource,
    preflight_render_routes,
};
use mdbook_structured_core::StructuredFormat;

fn chapter(name: &str, source_path: &str, logical_path: &str) -> Chapter {
    let mut chapter = Chapter::new(name, String::new(), source_path, Vec::new());
    chapter.path = Some(PathBuf::from(logical_path));
    chapter
}

fn synthetic_chapter(name: &str, logical_path: &str) -> Chapter {
    let mut chapter = Chapter::new(name, String::new(), logical_path, Vec::new());
    chapter.source_path = None;
    chapter
}

fn projected_paths(plan: &mdbook_structured::RenderRoutePlan) -> Vec<(&Path, &Path, &Path)> {
    plan.chapters()
        .iter()
        .map(|chapter| {
            (
                chapter.original_logical_path().as_path(),
                chapter.transformed_logical_path().as_path(),
                chapter.output_route().as_path(),
            )
        })
        .collect()
}

#[test]
fn projects_structured_and_markdown_routes_from_typed_book_values() {
    let book = Book::new_with_items(vec![
        chapter("Runtime", "config/runtime.yaml", "config/runtime.yaml").into(),
        chapter("Manifest", "schemas/manifest.json", "schemas/manifest.json").into(),
        chapter("YAML index", "README.yaml", "index.md").into(),
        chapter("JSON index", "README.json", "index.md").into(),
        chapter("Guide", "guide.md", "guide.md").into(),
    ]);
    let before = book.clone();

    let plan = preflight_render_routes(&book, Path::new("missing-source-root")).unwrap();

    assert_eq!(
        projected_paths(&plan),
        vec![
            (
                Path::new("config/runtime.yaml"),
                Path::new("config/runtime.yaml.md"),
                Path::new("config/runtime.yaml.html"),
            ),
            (
                Path::new("schemas/manifest.json"),
                Path::new("schemas/manifest.json.md"),
                Path::new("schemas/manifest.json.html"),
            ),
            (
                Path::new("index.md"),
                Path::new("index.yaml.md"),
                Path::new("index.yaml.html"),
            ),
            (
                Path::new("index.md"),
                Path::new("index.json.md"),
                Path::new("index.json.html"),
            ),
            (
                Path::new("guide.md"),
                Path::new("guide.md"),
                Path::new("guide.html"),
            ),
        ]
    );
    assert_eq!(book, before);
}

#[test]
fn projects_eligibility_only_from_case_sensitive_source_path() {
    let book = Book::new_with_items(vec![
        chapter("Source wins", "data.json", "chapter.md").into(),
        chapter("Logical does not", "chapter.md", "logical.yaml").into(),
        chapter("Case-sensitive", "upper.YAML", "upper.md").into(),
        synthetic_chapter("Synthetic", "drafts/../synthetic.yaml").into(),
        Chapter::new_draft("Draft", Vec::new()).into(),
    ]);
    let before = book.clone();

    let plan = preflight_render_routes(&book, Path::new("missing-source-root")).unwrap();

    assert_eq!(plan.chapters().len(), 4);
    assert_eq!(
        projected_paths(&plan),
        vec![
            (
                Path::new("chapter.md"),
                Path::new("chapter.json.md"),
                Path::new("chapter.json.html"),
            ),
            (
                Path::new("logical.yaml"),
                Path::new("logical.yaml"),
                Path::new("logical.html"),
            ),
            (
                Path::new("upper.md"),
                Path::new("upper.md"),
                Path::new("upper.html"),
            ),
            (
                Path::new("synthetic.yaml"),
                Path::new("synthetic.yaml"),
                Path::new("synthetic.html"),
            ),
        ]
    );
    assert!(plan.chapters()[3].source_path().is_none());
    assert!(plan.chapters()[3].structured_source().is_none());
    assert_eq!(book, before);
}

#[test]
fn projects_structured_source_formats_and_extensions() {
    let sources = [
        (
            "data.json",
            StructuredExtension::Json,
            StructuredFormat::Json,
        ),
        (
            "data.yaml",
            StructuredExtension::Yaml,
            StructuredFormat::Yaml,
        ),
        ("data.yml", StructuredExtension::Yml, StructuredFormat::Yaml),
    ];

    for (source_path, expected_extension, expected_format) in sources {
        let book = Book::new_with_items(vec![chapter("Data", source_path, "data.md").into()]);
        let plan = preflight_render_routes(&book, Path::new("missing-source-root")).unwrap();
        let source: &StructuredSource = plan.chapters()[0].structured_source().unwrap();

        assert_eq!(source.source_path(), Path::new(source_path));
        assert_eq!(source.extension(), expected_extension);
        assert_eq!(source.format(), expected_format);
    }
}

#[test]
fn projects_nested_chapters_in_parent_first_non_draft_order() {
    let child = chapter("Child", "nested/child.json", "nested/child.json");
    let mut parent = chapter("Parent", "parent.yaml", "parent.yaml");
    parent.sub_items.push(BookItem::Chapter(child));
    let book = Book::new_with_items(vec![BookItem::Chapter(parent)]);
    let before = book.clone();

    let plan = preflight_render_routes(&book, Path::new("missing-source-root")).unwrap();

    assert_eq!(plan.chapters().len(), 2);
    assert_eq!(plan.chapters()[0].ordinal().get(), 0);
    assert_eq!(plan.chapters()[1].ordinal().get(), 1);
    let parent_ordinal = plan.chapters()[0].ordinal();
    let child_ordinal = plan.chapters()[1].ordinal();
    assert_eq!(
        plan.chapter(parent_ordinal).unwrap().source_path(),
        Some(Path::new("parent.yaml")),
    );
    assert_eq!(
        plan.chapter(child_ordinal).unwrap().source_path(),
        Some(Path::new("nested/child.json")),
    );
    assert_eq!(book, before);
}

#[test]
fn projects_all_exact_route_collisions_into_one_deterministic_diagnostic() {
    let book = Book::new_with_items(vec![
        chapter("Structured settings", "settings.yaml", "settings.yaml").into(),
        chapter("Authored shim", "settings.yaml.md", "settings.yaml.md").into(),
    ]);
    let before = book.clone();

    let diagnostic = preflight_render_routes(&book, Path::new("missing-source-root")).unwrap_err();

    let AppDiagnosticKind::RouteCollision { first, additional } = diagnostic.kind() else {
        panic!("expected route collision, got {:?}", diagnostic.kind());
    };
    assert_eq!(
        first.output_route().as_path(),
        Path::new("settings.yaml.html")
    );
    assert_eq!(first.first_chapter().chapter_name(), "Structured settings");
    assert_eq!(
        first.first_chapter().source_path().unwrap().as_path(),
        Path::new("settings.yaml"),
    );
    assert_eq!(
        first.first_chapter().logical_path().as_path(),
        Path::new("settings.yaml"),
    );
    assert_eq!(first.second_chapter().chapter_name(), "Authored shim");
    assert!(first.additional_chapters().is_empty());
    assert!(additional.is_empty());
    assert_eq!(book, before);
}

#[test]
fn projects_literal_md_hazard_as_refined_projected_route() {
    let book = Book::new_with_items(vec![
        chapter("Hazard", "guide.md.yaml", "guide.md.yaml").into(),
    ]);
    let before = book.clone();

    let diagnostic = preflight_render_routes(&book, Path::new("missing-source-root")).unwrap_err();

    let AppDiagnosticKind::MdBookPathHazard {
        fact: MdBookPathHazardFact::ProjectedRoute(route),
    } = diagnostic.kind()
    else {
        panic!(
            "expected projected route hazard, got {:?}",
            diagnostic.kind()
        );
    };
    assert_eq!(route.as_path(), Path::new("guide.md.yaml.html"));
    assert_eq!(book, before);
}

#[test]
fn projects_exact_structured_static_source_collision() {
    let source_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(source_dir.path().join("config")).unwrap();
    fs::write(source_dir.path().join("config/runtime.yaml.html"), "static").unwrap();
    let book = Book::new_with_items(vec![
        chapter("Runtime", "config/runtime.yaml", "config/runtime.yaml").into(),
    ]);
    let before = book.clone();

    let diagnostic = preflight_render_routes(&book, source_dir.path()).unwrap_err();

    let AppDiagnosticKind::StaticSourceCollision {
        output_route,
        static_source,
    } = diagnostic.kind()
    else {
        panic!(
            "expected static source collision, got {:?}",
            diagnostic.kind()
        );
    };
    assert_eq!(
        output_route.as_path(),
        Path::new("config/runtime.yaml.html")
    );
    assert_eq!(
        static_source,
        &source_dir.path().join("config/runtime.yaml.html")
    );
    assert_eq!(book, before);
}

#[test]
fn projects_exact_ordinary_static_source_collision() {
    let source_dir = tempfile::tempdir().unwrap();
    fs::write(source_dir.path().join("guide.html"), "static").unwrap();
    let book = Book::new_with_items(vec![chapter("Guide", "guide.md", "guide.md").into()]);
    let before = book.clone();

    let diagnostic = preflight_render_routes(&book, source_dir.path()).unwrap_err();

    let AppDiagnosticKind::StaticSourceCollision {
        output_route,
        static_source,
    } = diagnostic.kind()
    else {
        panic!(
            "expected static source collision, got {:?}",
            diagnostic.kind()
        );
    };
    assert_eq!(output_route.as_path(), Path::new("guide.html"));
    assert_eq!(static_source, &source_dir.path().join("guide.html"));
    assert_eq!(book, before);
}
