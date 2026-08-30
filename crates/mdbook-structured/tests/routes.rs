use std::fs;
use std::path::{Path, PathBuf};

use mdbook_core::book::{Book, BookItem, Chapter};
use mdbook_structured::{
    AppDiagnosticKind, LinkResolution, LinkRouteMap, LogicalChapterPath, MdBookPathHazardFact,
    StructuredExtension, StructuredSource, preflight_render_routes,
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

fn path(value: &str) -> LogicalChapterPath {
    LogicalChapterPath::try_from_path(Path::new(value)).unwrap()
}

fn assert_exact(
    map: &LinkRouteMap,
    lookup_path: &str,
    expected_source_path: Option<&str>,
    expected_output_route: &str,
) {
    let LinkResolution::Exact(target) = map.resolve(&path(lookup_path)) else {
        panic!("expected {lookup_path} to resolve exactly");
    };
    assert_eq!(
        target.source_path().map(LogicalChapterPath::as_path),
        expected_source_path.map(Path::new)
    );
    assert_eq!(
        target.output_route().as_path(),
        Path::new(expected_output_route)
    );
}

fn assert_unique_alias(map: &LinkRouteMap, lookup_path: &str, expected_output_route: &str) {
    let LinkResolution::UniqueAlias(target) = map.resolve(&path(lookup_path)) else {
        panic!("expected {lookup_path} to resolve as a unique README alias");
    };
    assert_eq!(
        target.output_route().as_path(),
        Path::new(expected_output_route)
    );
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
fn lookup_every_real_source_path_is_exact() {
    let book = Book::new_with_items(vec![
        chapter("Guide", "guide.md", "guide.md").into(),
        chapter("Runtime", "config/runtime.yaml", "config/runtime.yaml.md").into(),
        chapter("Readme", "docs/README.md", "docs/index.md").into(),
    ]);

    let map = LinkRouteMap::from_book(&book).unwrap();

    assert_exact(&map, "guide.md", Some("guide.md"), "guide.html");
    assert_exact(
        &map,
        "config/runtime.yaml",
        Some("config/runtime.yaml"),
        "config/runtime.yaml.html",
    );
    assert_exact(
        &map,
        "docs/README.md",
        Some("docs/README.md"),
        "docs/index.html",
    );
}

#[test]
fn lookup_independently_authored_logical_path_is_exact_without_source_provenance() {
    let book = Book::new_with_items(vec![
        synthetic_chapter("Generated", "generated/catalog.md").into(),
    ]);

    let map = LinkRouteMap::from_book(&book).unwrap();

    assert_exact(&map, "generated/catalog.md", None, "generated/catalog.html");
}

#[test]
fn lookup_structured_source_is_exact_but_its_render_shim_is_not() {
    let book = Book::new_with_items(vec![
        chapter("Runtime", "config/runtime.yaml", "config/runtime.yaml.md").into(),
    ]);

    let map = LinkRouteMap::from_book(&book).unwrap();

    assert_exact(
        &map,
        "config/runtime.yaml",
        Some("config/runtime.yaml"),
        "config/runtime.yaml.html",
    );
    assert!(matches!(
        map.resolve(&path("config/runtime.yaml.md")),
        LinkResolution::Missing
    ));
}

#[test]
fn lookup_authored_structured_shim_owns_its_exact_path() {
    let book = Book::new_with_items(vec![
        chapter("Runtime", "config/runtime.yaml", "config/runtime.yaml.md").into(),
        chapter(
            "Authored shim",
            "config/runtime.yaml.md",
            "config/runtime.yaml.md",
        )
        .into(),
    ]);

    let map = LinkRouteMap::from_book(&book).unwrap();

    assert_exact(
        &map,
        "config/runtime.yaml.md",
        Some("config/runtime.yaml.md"),
        "config/runtime.yaml.html",
    );
}

#[test]
fn lookup_direct_structured_readme_sources_resolve_to_distinct_routes() {
    let book = Book::new_with_items(vec![
        chapter("YAML readme", "README.yaml", "index.yaml.md").into(),
        chapter("JSON readme", "README.json", "index.json.md").into(),
    ]);

    let map = LinkRouteMap::from_book(&book).unwrap();

    assert_exact(&map, "README.yaml", Some("README.yaml"), "index.yaml.html");
    assert_exact(&map, "README.json", Some("README.json"), "index.json.html");
}

#[test]
fn lookup_ordinary_and_structured_readmes_supply_index_and_directory_aliases() {
    let ordinary_book = Book::new_with_items(vec![
        chapter("Guide readme", "guide/README.md", "guide/index.md").into(),
    ]);
    let ordinary = LinkRouteMap::from_book(&ordinary_book).unwrap();
    assert_unique_alias(&ordinary, "guide/index.md", "guide/index.html");
    assert_unique_alias(&ordinary, "guide", "guide/index.html");

    let structured_book = Book::new_with_items(vec![
        chapter(
            "Config readme",
            "config/README.yaml",
            "config/index.yaml.md",
        )
        .into(),
    ]);
    let structured = LinkRouteMap::from_book(&structured_book).unwrap();
    assert_unique_alias(&structured, "config/index.md", "config/index.yaml.html");
    assert_unique_alias(&structured, "config", "config/index.yaml.html");
}

#[test]
fn lookup_exact_index_chapter_wins_over_readme_aliases() {
    let book = Book::new_with_items(vec![
        chapter("Readme", "config/README.yaml", "config/index.yaml.md").into(),
        chapter("Index", "config/index.md", "config/index.md").into(),
    ]);

    let map = LinkRouteMap::from_book(&book).unwrap();

    assert_exact(
        &map,
        "config/index.md",
        Some("config/index.md"),
        "config/index.html",
    );
}

#[test]
fn lookup_unused_ordinary_and_structured_readme_aliases_do_not_fail_construction() {
    let book = Book::new_with_items(vec![
        chapter("Ordinary readme", "config/README.md", "config/index.md").into(),
        chapter(
            "Structured readme",
            "config/README.yaml",
            "config/index.yaml.md",
        )
        .into(),
    ]);

    LinkRouteMap::from_book(&book).unwrap();
}

#[test]
fn lookup_case_insensitive_readme_stems_require_the_received_index_relation() {
    let book = Book::new_with_items(vec![
        chapter("Ordinary", "guide/rEaDmE.md", "guide/index.md").into(),
        chapter("Structured", "config/ReAdMe.json", "config/index.json.md").into(),
        chapter("Unsynthesized", "other/README.yaml", "other/README.yaml.md").into(),
    ]);

    let map = LinkRouteMap::from_book(&book).unwrap();

    assert_unique_alias(&map, "guide/index.md", "guide/index.html");
    assert_unique_alias(&map, "config/index.md", "config/index.json.html");
    assert!(matches!(
        map.resolve(&path("other/index.md")),
        LinkResolution::Missing
    ));
}

#[test]
fn lookup_ambiguous_readme_aliases_preserve_exact_sources_and_candidate_order() {
    let book = Book::new_with_items(vec![
        chapter("YAML readme", "config/README.yaml", "config/index.yaml.md").into(),
        chapter("JSON readme", "config/README.json", "config/index.json.md").into(),
    ]);

    let map = LinkRouteMap::from_book(&book).unwrap();

    assert!(matches!(
        map.resolve(&path("config/README.yaml")),
        LinkResolution::Exact(_)
    ));
    assert!(matches!(
        map.resolve(&path("config/README.json")),
        LinkResolution::Exact(_)
    ));
    let LinkResolution::Ambiguous(candidates) = map.resolve(&path("config/index.md")) else {
        panic!("expected config/index.md to preserve both README candidates");
    };
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].source_path().unwrap().as_path(),
        Path::new("config/README.yaml")
    );
    assert_eq!(
        candidates[1].source_path().unwrap().as_path(),
        Path::new("config/README.json")
    );
}

#[test]
fn lookup_exact_index_replaces_observable_readme_ambiguity() {
    let book = Book::new_with_items(vec![
        chapter("YAML readme", "config/README.yaml", "config/index.yaml.md").into(),
        chapter("JSON readme", "config/README.json", "config/index.json.md").into(),
        chapter("Index", "config/index.md", "config/index.md").into(),
    ]);

    let map = LinkRouteMap::from_book(&book).unwrap();

    assert_exact(
        &map,
        "config/index.md",
        Some("config/index.md"),
        "config/index.html",
    );
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

#[cfg(unix)]
#[test]
fn projects_exact_output_symlink_to_file_is_static_collision() {
    let source_dir = tempfile::tempdir().unwrap();
    fs::write(source_dir.path().join("static-target.txt"), "static").unwrap();
    std::os::unix::fs::symlink("static-target.txt", source_dir.path().join("guide.html")).unwrap();
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

#[cfg(unix)]
#[test]
fn projects_dangling_exact_output_symlink_is_io_failure() {
    let source_dir = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("missing-target.html", source_dir.path().join("guide.html"))
        .unwrap();
    let book = Book::new_with_items(vec![chapter("Guide", "guide.md", "guide.md").into()]);
    let before = book.clone();

    let diagnostic = preflight_render_routes(&book, source_dir.path()).unwrap_err();

    let AppDiagnosticKind::Io { path } = diagnostic.kind() else {
        panic!("expected I/O failure, got {:?}", diagnostic.kind());
    };
    assert_eq!(
        path.as_deref(),
        Some(source_dir.path().join("guide.html").as_path())
    );
    assert_eq!(book, before);
}
