use std::path::{Path, PathBuf};
use std::str::FromStr;

use mdbook_core::book::{Book, BookItem, Chapter};
use mdbook_preprocessor::{PreprocessorContext, config::Config};
use mdbook_structured::{
    AppDiagnostic, AppDiagnosticKind, HtmlPreprocessorContext, RewriteOptions, rewrite_book_links,
};

fn chapter(name: &str, source_path: &str, logical_path: &str) -> Chapter {
    let mut chapter = Chapter::new(name, String::new(), source_path, Vec::new());
    chapter.path = Some(PathBuf::from(logical_path));
    chapter
}

fn target_items() -> Vec<BookItem> {
    vec![
        chapter("Runtime", "config/runtime.yaml", "config/runtime.yaml.md").into(),
        chapter(
            "Profile with spaces",
            "config/profile (dev).json",
            "config/profile (dev).json.md",
        )
        .into(),
        chapter(
            "Parenthesized runtime",
            "config/run(time).yaml",
            "config/run(time).yaml.md",
        )
        .into(),
        chapter(
            "Archived data",
            "config/(archive)/data.json",
            "config/(archive)/data.json.md",
        )
        .into(),
        chapter(
            "Percent spelling",
            "config/my file.yaml",
            "config/my file.yaml.md",
        )
        .into(),
        chapter("YAML readme", "README.yaml", "index.yaml.md").into(),
        chapter("JSON readme", "README.json", "index.json.md").into(),
    ]
}

fn rewrite(current_path: &str, markdown: &str) -> Result<String, AppDiagnostic> {
    let mut source = Chapter::new("Source", markdown.to_owned(), current_path, Vec::new());
    source.path = Some(PathBuf::from(current_path));
    let mut items = vec![source.into()];
    items.extend(target_items());
    let context = PreprocessorContext::new(
        PathBuf::from("book"),
        Config::from_str("[book]\nsrc = \"chapters\"\n").unwrap(),
        "html".to_owned(),
    );
    let context = HtmlPreprocessorContext::try_from_context(context).unwrap();
    let rewritten = rewrite_book_links(
        RewriteOptions::from_context(&context).unwrap(),
        Book::new_with_items(items),
    )?;
    Ok(rewritten.chapters().next().unwrap().content.clone())
}

fn assert_destination_edits(markdown: &str, rewritten: &str, edits: &[(&str, &str)]) {
    let mut source_cursor = 0;
    let mut rewritten_cursor = 0;

    for &(authored, replacement) in edits {
        let relative_start = markdown[source_cursor..]
            .find(authored)
            .unwrap_or_else(|| panic!("missing authored destination {authored:?}"));
        let source_start = source_cursor + relative_start;
        let unchanged_len = source_start - source_cursor;
        let rewritten_start = rewritten_cursor + unchanged_len;

        assert_eq!(
            &rewritten[rewritten_cursor..rewritten_start],
            &markdown[source_cursor..source_start],
            "non-destination bytes changed before {authored:?}",
        );
        assert_eq!(
            &rewritten[rewritten_start..rewritten_start + replacement.len()],
            replacement,
            "unexpected replacement for {authored:?}",
        );

        source_cursor = source_start + authored.len();
        rewritten_cursor = rewritten_start + replacement.len();
    }

    assert_eq!(
        &rewritten[rewritten_cursor..],
        &markdown[source_cursor..],
        "non-destination bytes changed after the final edit",
    );
}

#[test]
fn rewrite_rejects_duplicate_registered_source_identities() {
    let context = HtmlPreprocessorContext::try_from_context(PreprocessorContext::new(
        PathBuf::from("book"),
        Config::from_str("[book]\nsrc = \"chapters\"\n").unwrap(),
        "html".to_owned(),
    ))
    .unwrap();
    let error = rewrite_book_links(
        RewriteOptions::from_context(&context).unwrap(),
        Book::new_with_items(vec![
            chapter("First registration", "shared.yaml", "first.yaml.md").into(),
            chapter("Second registration", "shared.yaml", "second.yaml.md").into(),
        ]),
    )
    .unwrap_err();

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
fn rewrite_rejects_duplicate_registered_output_routes() {
    let context = HtmlPreprocessorContext::try_from_context(PreprocessorContext::new(
        PathBuf::from("book"),
        Config::from_str("[book]\nsrc = \"chapters\"\n").unwrap(),
        "html".to_owned(),
    ))
    .unwrap();
    let error = rewrite_book_links(
        RewriteOptions::from_context(&context).unwrap(),
        Book::new_with_items(vec![
            chapter("First registration", "first.yaml", "shared.yaml.md").into(),
            chapter("Second registration", "second.yaml", "shared.yaml.md").into(),
        ]),
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
    assert_eq!(second.output_route(), Path::new("shared.yaml.html"));
    assert!(additional.is_empty());
}

#[test]
fn inline_rewrites_only_destinations_with_nested_labels_angles_escapes_and_titles() {
    let markdown = concat!(
        "Before [ordinary](../config/runtime.yaml) and ",
        "[**nested**](<../config/profile (dev).json> \"quoted title\") and ",
        "[escaped](../config/run\\(time\\).yaml 'single title') and ",
        "[balanced](../config/(archive)/data.json (parenthesized title)).\n",
    );

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[
            ("../config/runtime.yaml", "../config/runtime.yaml.html"),
            (
                "../config/profile (dev).json",
                "../config/profile (dev).json.html",
            ),
            (
                "../config/run\\(time\\).yaml",
                "../config/run(time).yaml.html",
            ),
            (
                "../config/(archive)/data.json",
                "../config/(archive)/data.json.html",
            ),
        ],
    );
}

#[test]
fn inline_preserves_query_fragment_and_percent_spelling() {
    let markdown = concat!(
        "[space](../config/my%20file.yaml?download=.yaml#part%2Done) ",
        "[dot](../config/runtime%2Eyaml?raw=%2f#tail%2Fcase)\n",
    );

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[
            (
                "../config/my%20file.yaml?download=.yaml#part%2Done",
                "../config/my%20file.yaml.html?download=.yaml#part%2Done",
            ),
            (
                "../config/runtime%2Eyaml?raw=%2f#tail%2Fcase",
                "../config/runtime%2Eyaml.html?raw=%2f#tail%2Fcase",
            ),
        ],
    );
}

#[test]
fn inline_preserves_percent_triplet_that_decodes_to_a_path_separator() {
    let markdown = "[archive](../config%2F(archive)/data.json)\n";

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[(
            "../config%2F(archive)/data.json",
            "../config%2F(archive)/data.json.html",
        )],
    );
}

#[test]
fn inline_regression_normalizes_percent_encoded_dot_segments() {
    let cases = [
        (
            "[parent](%2E%2E/config/runtime.yaml)\n",
            "%2E%2E/config/runtime.yaml",
            "../config/runtime.yaml.html",
        ),
        (
            "[current](../%2E/config/runtime.yaml)\n",
            "../%2E/config/runtime.yaml",
            "../config/runtime.yaml.html",
        ),
    ];

    for (markdown, authored, replacement) in cases {
        let rewritten = rewrite("guide/setup.md", markdown).unwrap();

        assert_destination_edits(markdown, &rewritten, &[(authored, replacement)]);
    }
}

#[test]
fn inline_regression_preserves_numeric_entity_query_delimiter() {
    let markdown = "[runtime](../config/runtime.yaml&#63;mode=raw)\n";

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[(
            "../config/runtime.yaml&#63;mode=raw",
            "../config/runtime.yaml.html&#63;mode=raw",
        )],
    );
}

#[test]
fn inline_regression_preserves_named_entity_query_delimiter() {
    let markdown = "[runtime](../config/runtime.yaml&quest;mode=raw)\n";

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[(
            "../config/runtime.yaml&quest;mode=raw",
            "../config/runtime.yaml.html&quest;mode=raw",
        )],
    );
}

#[test]
fn inline_regression_preserves_numeric_entity_fragment_delimiter() {
    let markdown = "[runtime](../config/runtime.yaml&#35;runtime)\n";

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[(
            "../config/runtime.yaml&#35;runtime",
            "../config/runtime.yaml.html&#35;runtime",
        )],
    );
}

#[test]
fn inline_regression_preserves_named_entity_fragment_delimiter() {
    let markdown = "[runtime](../config/runtime.yaml&num;runtime)\n";

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[(
            "../config/runtime.yaml&num;runtime",
            "../config/runtime.yaml.html&num;runtime",
        )],
    );
}

#[test]
fn inline_preserves_authored_entities_in_query_and_fragment_bytes() {
    let markdown = "[entity](../config/runtime.yaml?x=&amp;#frag&amp;)\n";

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[(
            "../config/runtime.yaml?x=&amp;#frag&amp;",
            "../config/runtime.yaml.html?x=&amp;#frag&amp;",
        )],
    );
}

#[test]
fn inline_handles_container_prefixed_multiline_title() {
    let markdown = "> [runtime](../config/runtime.yaml\n> \"title\")\n";

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[("../config/runtime.yaml", "../config/runtime.yaml.html")],
    );
}

#[test]
fn inline_resolves_nested_relative_paths_and_direct_readme_sources() {
    let markdown = concat!(
        "[runtime](../../config/runtime.yaml) ",
        "[yaml index](../../README.yaml) ",
        "[json index](../../README.json)\n",
    );

    let rewritten = rewrite("guide/deep/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[
            (
                "../../config/runtime.yaml",
                "../../config/runtime.yaml.html",
            ),
            ("../../README.yaml", "../../index.yaml.html"),
            ("../../README.json", "../../index.json.html"),
        ],
    );
}

#[test]
fn inline_leaves_empty_query_root_invalid_percent_and_above_root_destinations_unchanged() {
    let markdown = concat!(
        "[empty]() [query](?mode=raw) [root](/config/runtime.yaml) ",
        "[bad escape](../config/bad%ZZ.yaml) [non utf](../config/bad%FF.yaml) ",
        "[above root](../../../config/runtime.yaml)\n",
    );

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_eq!(rewritten.as_bytes(), markdown.as_bytes());
}

#[test]
fn inline_leaves_external_fragment_and_html_destinations_unchanged() {
    let markdown = concat!(
        "[scheme](https://example.com/config/runtime.yaml) ",
        "[protocol relative](//example.com/config/runtime.yaml) ",
        "[fragment](#runtime) [html](../config/runtime.html)\n",
    );

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_eq!(rewritten.as_bytes(), markdown.as_bytes());
}

#[test]
fn reference_rewrites_full_collapsed_and_shortcut_definition_destinations() {
    let markdown = concat!(
        "[full][runtime] [collapsed][] [Shortcut]\n\n",
        "[runtime]: ../config/runtime.yaml \"full title\"\n",
        "[collapsed]: <../config/runtime.yaml> 'collapsed title'\n",
        "[shortcut]: ../config/runtime.yaml (shortcut title)\n",
    );

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[
            ("../config/runtime.yaml", "../config/runtime.yaml.html"),
            ("../config/runtime.yaml", "../config/runtime.yaml.html"),
            ("../config/runtime.yaml", "../config/runtime.yaml.html"),
        ],
    );
}

#[test]
fn reference_handles_container_prefixed_title_on_the_following_line() {
    let markdown = concat!(
        "> [runtime link][runtime]\n>\n",
        "> [runtime]: ../config/runtime.yaml\n",
        ">   \"title\"\n",
    );

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[("../config/runtime.yaml", "../config/runtime.yaml.html")],
    );
}

#[test]
fn reference_shared_normal_definition_is_edited_once() {
    let markdown = concat!(
        "[first][runtime] and [second][runtime].\n\n",
        "[runtime]: ../config/runtime.yaml \"shared title\"\n",
    );

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[("../config/runtime.yaml", "../config/runtime.yaml.html")],
    );
}

#[test]
fn reference_image_only_definition_remains_byte_identical() {
    let markdown = concat!(
        "![runtime diagram][runtime]\n\n",
        "[runtime]: ../config/runtime.yaml \"image title\"\n",
    );

    let rewritten = rewrite("guide/setup.md", markdown).unwrap();

    assert_eq!(rewritten.as_bytes(), markdown.as_bytes());
}

#[test]
fn failure_literal_md_in_preserved_query_or_fragment_reports_complete_destination() {
    let cases = [
        (
            "[query](../config/runtime.yaml?source=.md#safe)\n",
            "../config/runtime.yaml.html?source=.md#safe",
        ),
        (
            "[fragment](../config/runtime.yaml?safe=1#notes.md)\n",
            "../config/runtime.yaml.html?safe=1#notes.md",
        ),
    ];

    for (markdown, expected_destination) in cases {
        let error = rewrite("guide/chapter.md", markdown).unwrap_err();

        let AppDiagnosticKind::MatchedReferencePathHazard {
            rewritten_destination,
        } = error.kind()
        else {
            panic!("expected a rewritten-destination hazard");
        };
        assert_eq!(rewritten_destination, expected_destination);
    }
}

#[test]
fn failure_literal_md_hazard_is_case_sensitive() {
    let markdown = "[runtime](../config/runtime.yaml?source=.MD#notes.Md)\n";

    let rewritten = rewrite("guide/chapter.md", markdown).unwrap();

    assert_destination_edits(
        markdown,
        &rewritten,
        &[(
            "../config/runtime.yaml?source=.MD#notes.Md",
            "../config/runtime.yaml.html?source=.MD#notes.Md",
        )],
    );
}

#[test]
fn failure_raw_html_images_code_and_nonlocal_destinations_remain_byte_identical() {
    let markdown = concat!(
        "<a href=\"../config/runtime.yaml\">raw</a>\n\n",
        "![inline image](../config/runtime.yaml)\n\n",
        "`[inline code](../config/runtime.yaml)`\n\n",
        "```markdown\n[fenced code](../config/runtime.yaml)\n```\n\n",
        "[external](https://example.com/runtime.yaml) ",
        "[fragment](#runtime) [missing](../config/missing.yaml) ",
        "[html](../config/runtime.html)\n",
    );

    let rewritten = rewrite("guide/chapter.md", markdown).unwrap();

    assert_eq!(rewritten.as_bytes(), markdown.as_bytes());
}

#[test]
fn failure_empty_query_root_invalid_percent_and_above_root_destinations_remain_byte_identical() {
    let markdown = concat!(
        "[empty]() [query](?mode=raw) [root](/config/runtime.yaml) ",
        "[invalid](../config/bad%Q0.yaml) [non utf](../config/bad%FE.yaml) ",
        "[above](../../../config/runtime.yaml)\n",
    );

    let rewritten = rewrite("guide/chapter.md", markdown).unwrap();

    assert_eq!(rewritten.as_bytes(), markdown.as_bytes());
}
