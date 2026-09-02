use std::path::{Path, PathBuf};
use std::str::FromStr;

use mdbook_core::book::{Book, BookItem, Chapter, SectionNumber};
use mdbook_preprocessor::{PreprocessorContext, config::Config};
use mdbook_structured::{
    AppDiagnosticKind, HtmlPreprocessorContext, RenderOptions, RewriteOptions, render_book,
    rewrite_book_links,
};

fn context(root: &Path, renderer: &str, configuration: &str) -> PreprocessorContext {
    PreprocessorContext::new(
        root.to_owned(),
        Config::from_str(configuration).unwrap(),
        renderer.to_owned(),
    )
}

fn html_context(root: &Path, configuration: &str) -> HtmlPreprocessorContext {
    HtmlPreprocessorContext::try_from_context(context(root, "html", configuration)).unwrap()
}

fn chapter(name: &str, content: &str, source_path: &str, logical_path: &str) -> Chapter {
    let mut chapter = Chapter::new(
        name,
        content.to_owned(),
        source_path,
        vec!["Part".to_owned()],
    );
    chapter.path = Some(PathBuf::from(logical_path));
    chapter.number = Some(SectionNumber::new(vec![2, 3]));
    chapter
}

fn generated_chapter(name: &str, content: &str, logical_path: &str) -> Chapter {
    let mut chapter = chapter(name, content, logical_path, logical_path);
    chapter.source_path = None;
    chapter
}

fn chapters(book: &Book) -> Vec<&Chapter> {
    book.chapters().collect()
}

fn assert_preserved_chapter_fields(before: &Book, after: &Book) {
    fn assert_items(before: &[BookItem], after: &[BookItem]) {
        assert_eq!(before.len(), after.len());
        for (before, after) in before.iter().zip(after) {
            match (before, after) {
                (BookItem::Chapter(before), BookItem::Chapter(after)) => {
                    assert_eq!(after.name, before.name);
                    assert_eq!(after.number, before.number);
                    assert_eq!(after.source_path, before.source_path);
                    assert_eq!(after.parent_names, before.parent_names);
                    assert_items(&before.sub_items, &after.sub_items);
                }
                (BookItem::PartTitle(before), BookItem::PartTitle(after)) => {
                    assert_eq!(after, before);
                }
                (BookItem::Separator, BookItem::Separator) => {}
                _ => panic!("book hierarchy changed"),
            }
        }
    }

    assert_items(&before.items, &after.items);
}

#[test]
fn render_refines_html_context_and_parses_plugin_options() {
    let temporary = tempfile::tempdir().unwrap();
    let refined = html_context(
        temporary.path(),
        r#"
[book]
src = "chapters"

[preprocessor.structured]
command = "mdbook-structured render"
after = ["index"]
before = ["links"]
renderers = ["html"]
optional = true
max-input-bytes = 7
max-nodes = 8
max-depth = 9
large-container-threshold = 10
"#,
    );

    let options = RenderOptions::from_context(&refined).unwrap();

    assert_eq!(options.limits().max_input_bytes().get(), 7);
    assert_eq!(options.limits().max_nodes().get(), 8);
    assert_eq!(options.limits().max_depth().get(), 9);
    assert_eq!(options.html().large_container_threshold(), 10);

    let defaults = RenderOptions::from_context(&html_context(
        temporary.path(),
        "[book]\nsrc = \"chapters\"\n",
    ))
    .unwrap();
    assert_eq!(defaults.limits().max_input_bytes().get(), 1_048_576);
    assert_eq!(defaults.limits().max_nodes().get(), 10_000);
    assert_eq!(defaults.limits().max_depth().get(), 64);
    assert_eq!(defaults.html().large_container_threshold(), 100);
}

#[test]
fn render_rejects_non_html_context_before_a_phase_can_receive_it() {
    let temporary = tempfile::tempdir().unwrap();

    let error = match HtmlPreprocessorContext::try_from_context(context(
        temporary.path(),
        "pdf",
        "[book]\nsrc = \"chapters\"\n",
    )) {
        Ok(_) => panic!("expected the pdf renderer to be rejected"),
        Err(error) => error,
    };

    let AppDiagnosticKind::UnsupportedRenderer { renderer } = error.kind() else {
        panic!("expected unsupported-renderer diagnostic");
    };
    assert_eq!(renderer, "pdf");
}

#[test]
fn render_transforms_nested_structured_chapters_without_touching_other_fields() {
    let temporary = tempfile::tempdir().unwrap();
    let mut json_parent = chapter(
        "JSON parent",
        r#"{"json-value":"parent"}"#,
        "data/parent.json",
        "data/parent.md",
    );
    json_parent.sub_items.push(
        chapter(
            "YAML child",
            "yaml-value: child\n",
            "data/child.yaml",
            "data/child.md",
        )
        .into(),
    );
    let book = Book::new_with_items(vec![
        chapter(
            "Markdown",
            "[parent](data/parent.json)",
            "guide.md",
            "guide.md",
        )
        .into(),
        json_parent.into(),
        generated_chapter(
            "Absent source",
            "this looks like an unlisted YAML path",
            "generated/unlisted.yaml",
        )
        .into(),
    ]);
    let before = book.clone();

    let rendered = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        book,
    )
    .unwrap();
    let rendered_chapters = chapters(&rendered);

    assert_eq!(rendered_chapters[0].content, "[parent](data/parent.json)");
    assert_eq!(
        rendered_chapters[0].path.as_deref(),
        Some(Path::new("guide.md"))
    );
    assert_eq!(
        rendered_chapters[1].path.as_deref(),
        Some(Path::new("data/parent.json.md"))
    );
    assert!(
        rendered_chapters[1]
            .content
            .contains("<h1>JSON parent</h1>")
    );
    assert!(rendered_chapters[1].content.contains("json-value"));
    assert!(rendered_chapters[1].content.contains("parent"));
    assert!(!rendered_chapters[1].content.contains("yaml-value"));
    assert_eq!(
        rendered_chapters[2].path.as_deref(),
        Some(Path::new("data/child.yaml.md"))
    );
    assert!(rendered_chapters[2].content.contains("<h1>YAML child</h1>"));
    assert!(rendered_chapters[2].content.contains("yaml-value"));
    assert!(rendered_chapters[2].content.contains("child"));
    assert!(!rendered_chapters[2].content.contains("json-value"));
    assert_eq!(
        rendered_chapters[3].content,
        "this looks like an unlisted YAML path"
    );
    assert_eq!(
        rendered_chapters[3].path.as_deref(),
        Some(Path::new("generated/unlisted.yaml"))
    );
    assert_preserved_chapter_fields(&before, &rendered);
}

#[test]
fn render_admits_structured_sources_before_interpreting_route_metadata() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "Ordinary sentinel",
            "ordinary Markdown",
            "guide.md",
            "../test/index.md",
        )
        .into(),
        chapter(
            "Registered YAML",
            "enabled: true\n",
            "config/runtime.yaml",
            "config/runtime.yaml",
        )
        .into(),
    ]);
    let before = book.clone();

    let rendered = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        book,
    )
    .unwrap();
    let before_chapters = chapters(&before);
    let rendered_chapters = chapters(&rendered);

    assert_eq!(rendered_chapters[0], before_chapters[0]);
    assert_eq!(
        rendered_chapters[1].path.as_deref(),
        Some(Path::new("config/runtime.yaml.md"))
    );
    assert!(
        rendered_chapters[1]
            .content
            .contains("<h1>Registered YAML</h1>")
    );
    assert!(rendered_chapters[1].content.contains("enabled"));
    assert!(rendered_chapters[1].content.contains("true"));
}

#[test]
fn render_admits_only_exact_lowercase_structured_source_extensions() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "Lowercase YML",
            "enabled: true\n",
            "config/runtime.yml",
            "config/runtime.yml",
        )
        .into(),
        chapter(
            "Uppercase YAML sentinel",
            "not: [valid",
            "config/runtime.YAML",
            "../ignored.md",
        )
        .into(),
    ]);
    let before = book.clone();

    let rendered = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        book,
    )
    .unwrap();
    let before_chapters = chapters(&before);
    let rendered_chapters = chapters(&rendered);

    assert_eq!(
        rendered_chapters[0].path.as_deref(),
        Some(Path::new("config/runtime.yml.md"))
    );
    assert!(
        rendered_chapters[0]
            .content
            .contains("<h1>Lowercase YML</h1>")
    );
    assert_eq!(rendered_chapters[1], before_chapters[1]);
}

#[test]
fn render_reports_missing_route_metadata_for_an_admitted_source() {
    let temporary = tempfile::tempdir().unwrap();
    let mut registered = chapter(
        "Registered YAML",
        "enabled: true\n",
        "config/runtime.yaml",
        "config/runtime.yaml",
    );
    registered.path = None;

    let result = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        Book::new_with_items(vec![registered.into()]),
    );

    let error = result.expect_err("an admitted source without a logical path must fail");
    let AppDiagnosticKind::RegisteredRoute { source_path } = error.kind() else {
        panic!("expected a registered-route diagnostic, got {error}");
    };
    assert_eq!(source_path, Path::new("config/runtime.yaml"));
}

#[test]
fn render_rejects_duplicate_registered_source_identities() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "First registration",
            "first: true\n",
            "shared.yaml",
            "first.md",
        )
        .into(),
        chapter(
            "Second registration",
            "second: true\n",
            "shared.yaml",
            "second.md",
        )
        .into(),
    ]);

    let result = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        book,
    );

    let error = result.expect_err("duplicate registered source identities must fail");
    let AppDiagnosticKind::DuplicateRegisteredSource {
        first,
        second,
        additional,
    } = error.kind()
    else {
        panic!("expected a duplicate-source diagnostic, got {error}");
    };
    assert_eq!(first.source_path(), Path::new("shared.yaml"));
    assert_eq!(second.source_path(), Path::new("shared.yaml"));
    assert_eq!(first.output_route(), Path::new("first.yaml.html"));
    assert_eq!(second.output_route(), Path::new("second.yaml.html"));
    assert!(additional.is_empty());
}

#[test]
fn render_rejects_duplicate_registered_output_routes() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "First registration",
            "first: true\n",
            "first.yaml",
            "shared.md",
        )
        .into(),
        chapter(
            "Second registration",
            "second: true\n",
            "second.yaml",
            "shared.md",
        )
        .into(),
    ]);

    let result = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        book,
    );

    let error = result.expect_err("duplicate registered output routes must fail");
    let AppDiagnosticKind::RegisteredRouteCollision {
        first,
        second,
        additional,
    } = error.kind()
    else {
        panic!("expected a registered-route collision, got {error}");
    };
    assert_eq!(first.source_path(), Path::new("first.yaml"));
    assert_eq!(second.source_path(), Path::new("second.yaml"));
    assert_eq!(first.output_route(), Path::new("shared.yaml.html"));
    assert_eq!(second.output_route(), Path::new("shared.yaml.html"));
    assert!(additional.is_empty());
}

#[test]
fn render_projects_a_literal_md_segment_in_a_registered_structured_source() {
    let temporary = tempfile::tempdir().unwrap();
    let rendered = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        Book::new_with_items(vec![
            chapter(
                "Guide configuration",
                "enabled: true\n",
                "guide.md.yaml",
                "guide.md.yaml",
            )
            .into(),
        ]),
    )
    .unwrap();
    let rendered_chapter = chapters(&rendered)[0];

    assert_eq!(
        rendered_chapter.path.as_deref(),
        Some(Path::new("guide.md.yaml.md"))
    );
    assert!(
        rendered_chapter
            .content
            .contains("<h1>Guide configuration</h1>")
    );
    assert!(rendered_chapter.content.contains("enabled"));
    assert!(rendered_chapter.content.contains("true"));
}

#[test]
fn render_projects_registered_readme_sources_to_distinct_routes() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "YAML README",
            "kind: yaml-index\n",
            "README.yaml",
            "index.md",
        )
        .into(),
        chapter(
            "JSON README",
            r#"{"kind":"json-index"}"#,
            "README.json",
            "index.md",
        )
        .into(),
    ]);

    let rendered = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        book,
    )
    .unwrap();
    let rendered_chapters = chapters(&rendered);

    assert_eq!(
        rendered_chapters[0].path.as_deref(),
        Some(Path::new("index.yaml.md"))
    );
    assert_eq!(
        rendered_chapters[1].path.as_deref(),
        Some(Path::new("index.json.md"))
    );
}

#[test]
fn render_route_diagnostics_do_not_observe_ordinary_or_synthetic_sentinels() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "Ordinary sentinel",
            "ordinary Markdown",
            "guide.md",
            "../ordinary/index.md",
        )
        .into(),
        generated_chapter(
            "Synthetic sentinel",
            "generated content",
            "../../synthetic.md",
        )
        .into(),
        chapter("First", "first: true\n", "first.yaml", "shared.md").into(),
        chapter("Second", "second: true\n", "second.yaml", "shared.md").into(),
    ]);

    let error = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        book,
    )
    .unwrap_err();

    let AppDiagnosticKind::RegisteredRouteCollision {
        first,
        second,
        additional,
    } = error.kind()
    else {
        panic!("expected a registered-route collision, got {error}");
    };
    assert_eq!(first.source_path(), Path::new("first.yaml"));
    assert_eq!(second.source_path(), Path::new("second.yaml"));
    assert_eq!(first.output_route(), Path::new("shared.yaml.html"));
    assert_eq!(second.output_route(), first.output_route());
    assert!(additional.is_empty());
}

#[test]
fn render_returns_a_core_diagnostic_when_a_later_structured_chapter_is_malformed() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter("First", r#"{"valid":true}"#, "first.json", "first.md").into(),
        chapter("Second", "unclosed: [\n", "second.yaml", "second.md").into(),
    ]);

    let error = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        book,
    )
    .unwrap_err();

    assert!(error.core_diagnostic().is_some());
}

#[test]
fn rewrite_accepts_only_empty_plugin_options_after_mdbook_registration_keys() {
    let temporary = tempfile::tempdir().unwrap();
    for configuration in [
        "[book]\nsrc = \"chapters\"\n",
        "[book]\nsrc = \"chapters\"\n[preprocessor.structured-links]\n",
        r#"
[book]
src = "chapters"

[preprocessor.structured-links]
command = "mdbook-structured rewrite-links"
after = ["links"]
before = ["html"]
renderers = ["html"]
optional = true
"#,
    ] {
        RewriteOptions::from_context(&html_context(temporary.path(), configuration)).unwrap();
    }

    let error = RewriteOptions::from_context(&html_context(
        temporary.path(),
        "[book]\nsrc = \"chapters\"\n[preprocessor.structured-links]\nunknown = true\n",
    ))
    .unwrap_err();
    assert!(matches!(
        error.kind(),
        AppDiagnosticKind::Configuration { .. }
    ));
}

#[test]
fn rewrite_rewrites_nested_markdown_links_without_scanning_generated_html() {
    let temporary = tempfile::tempdir().unwrap();
    let mut parent = chapter(
        "Guide",
        "[runtime](data/runtime.yaml)",
        "guide.md",
        "guide.md",
    );
    parent.sub_items.push(
        chapter(
            "Nested guide",
            "[runtime](../data/runtime.yaml)",
            "guide/nested.md",
            "guide/nested.md",
        )
        .into(),
    );
    let rendered = render_book(
        &html_context(temporary.path(), "[book]\nsrc = \"chapters\"\n"),
        Book::new_with_items(vec![
            parent.into(),
            chapter(
                "Runtime",
                "link-like: '[guide](guide.md)'\n",
                "data/runtime.yaml",
                "data/runtime.md",
            )
            .into(),
        ]),
    )
    .unwrap();
    let generated_html = chapters(&rendered)[2].content.clone();

    let rewritten = rewrite_book_links(
        RewriteOptions::from_context(&html_context(
            temporary.path(),
            "[book]\nsrc = \"chapters\"\n",
        ))
        .unwrap(),
        rendered.clone(),
    )
    .unwrap();
    let rewritten_chapters = chapters(&rewritten);

    assert_eq!(
        rewritten_chapters[0].content,
        "[runtime](data/runtime.yaml.html)"
    );
    assert_eq!(
        rewritten_chapters[1].content,
        "[runtime](../data/runtime.yaml.html)"
    );
    assert_eq!(rewritten_chapters[2].content, generated_html);
    assert_preserved_chapter_fields(&rendered, &rewritten);
}

#[test]
fn rewrite_ignores_unusable_ordinary_route_metadata() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "Ordinary sentinel",
            "[runtime](config/runtime.yaml)",
            "guide.md",
            "../test/index.md",
        )
        .into(),
        chapter(
            "Runtime",
            "<section class=\"structured-document\"></section>",
            "config/runtime.yaml",
            "config/runtime.yaml.md",
        )
        .into(),
    ]);
    let before = book.clone();

    let rewritten = rewrite_book_links(
        RewriteOptions::from_context(&html_context(
            temporary.path(),
            "[book]\nsrc = \"chapters\"\n",
        ))
        .unwrap(),
        book,
    )
    .unwrap();

    assert_eq!(rewritten, before);
}

#[test]
fn rewrite_matches_a_direct_registered_structured_source() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "Guide",
            "[runtime](config/runtime.yaml)",
            "guide.md",
            "guide.md",
        )
        .into(),
        chapter(
            "Runtime",
            "<section class=\"structured-document\"></section>",
            "config/runtime.yaml",
            "config/runtime.yaml.md",
        )
        .into(),
    ]);

    let rewritten = rewrite_book_links(
        RewriteOptions::from_context(&html_context(
            temporary.path(),
            "[book]\nsrc = \"chapters\"\n",
        ))
        .unwrap(),
        book,
    )
    .unwrap();

    assert_eq!(
        chapters(&rewritten)[0].content,
        "[runtime](config/runtime.yaml.html)"
    );
}

#[test]
fn rewrite_leaves_unregistered_static_like_destinations_unchanged() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "Guide",
            "[json](static/data.json) [yaml](static/data.yaml)",
            "guide.md",
            "guide.md",
        )
        .into(),
    ]);
    let before = book.clone();

    let rewritten = rewrite_book_links(
        RewriteOptions::from_context(&html_context(
            temporary.path(),
            "[book]\nsrc = \"chapters\"\n",
        ))
        .unwrap(),
        book,
    )
    .unwrap();

    assert_eq!(rewritten, before);
}

#[test]
fn rewrite_skips_a_source_chapter_without_a_usable_relative_base() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        generated_chapter(
            "Synthetic sentinel",
            "[runtime](config/runtime.yaml)",
            "/generated/index.md",
        )
        .into(),
        chapter(
            "Runtime",
            "<section class=\"structured-document\"></section>",
            "config/runtime.yaml",
            "config/runtime.yaml.md",
        )
        .into(),
    ]);
    let before = book.clone();

    let rewritten = rewrite_book_links(
        RewriteOptions::from_context(&html_context(
            temporary.path(),
            "[book]\nsrc = \"chapters\"\n",
        ))
        .unwrap(),
        book,
    )
    .unwrap();

    assert_eq!(rewritten, before);
}

#[test]
fn rewrite_matches_direct_readme_sources_to_distinct_routes() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "Guide",
            "[yaml](config/README.yaml) [json](config/README.json)",
            "guide.md",
            "guide.md",
        )
        .into(),
        chapter(
            "YAML README",
            "<section class=\"structured-document\"></section>",
            "config/README.yaml",
            "config/index.yaml.md",
        )
        .into(),
        chapter(
            "JSON README",
            "<section class=\"structured-document\"></section>",
            "config/README.json",
            "config/index.json.md",
        )
        .into(),
    ]);

    let rewritten = rewrite_book_links(
        RewriteOptions::from_context(&html_context(
            temporary.path(),
            "[book]\nsrc = \"chapters\"\n",
        ))
        .unwrap(),
        book,
    )
    .unwrap();

    assert_eq!(
        chapters(&rewritten)[0].content,
        "[yaml](config/index.yaml.html) [json](config/index.json.html)"
    );
}

#[test]
fn rewrite_does_not_create_index_or_directory_aliases() {
    let temporary = tempfile::tempdir().unwrap();
    let book = Book::new_with_items(vec![
        chapter(
            "Guide",
            "[index](config/index.md) [directory](config/)",
            "guide.md",
            "guide.md",
        )
        .into(),
        chapter(
            "YAML README",
            "<section class=\"structured-document\"></section>",
            "config/README.yaml",
            "config/index.yaml.md",
        )
        .into(),
    ]);
    let before = book.clone();

    let rewritten = rewrite_book_links(
        RewriteOptions::from_context(&html_context(
            temporary.path(),
            "[book]\nsrc = \"chapters\"\n",
        ))
        .unwrap(),
        book,
    )
    .unwrap();

    assert_eq!(rewritten, before);
}

#[test]
fn rewrite_reports_unusable_route_metadata_for_a_registered_target() {
    let temporary = tempfile::tempdir().unwrap();

    for target_path in [None, Some(PathBuf::from("config/runtime.yaml"))] {
        let mut target = chapter(
            "Runtime",
            "<section class=\"structured-document\"></section>",
            "config/runtime.yaml",
            "config/runtime.yaml.md",
        );
        target.path = target_path;
        let book = Book::new_with_items(vec![
            chapter(
                "Guide",
                "[runtime](config/runtime.yaml)",
                "guide.md",
                "guide.md",
            )
            .into(),
            target.into(),
        ]);

        let error = rewrite_book_links(
            RewriteOptions::from_context(&html_context(
                temporary.path(),
                "[book]\nsrc = \"chapters\"\n",
            ))
            .unwrap(),
            book,
        )
        .unwrap_err();

        let AppDiagnosticKind::RegisteredRoute { source_path } = error.kind() else {
            panic!("expected a registered-route diagnostic, got {error}");
        };
        assert_eq!(source_path, Path::new("config/runtime.yaml"));
    }
}
