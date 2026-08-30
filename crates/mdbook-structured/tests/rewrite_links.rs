use std::path::{Path, PathBuf};

use mdbook_core::book::{Book, Chapter};
use mdbook_structured::{LinkRouteMap, LogicalChapterPath, rewrite_chapter_links};

fn chapter(name: &str, source_path: &str, logical_path: &str) -> Chapter {
    let mut chapter = Chapter::new(name, String::new(), source_path, Vec::new());
    chapter.path = Some(PathBuf::from(logical_path));
    chapter
}

fn path(value: &str) -> LogicalChapterPath {
    LogicalChapterPath::try_from_path(Path::new(value)).unwrap()
}

fn routes() -> LinkRouteMap {
    LinkRouteMap::from_book(&Book::new_with_items(vec![
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
    ]))
    .unwrap()
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
fn inline_rewrites_only_destinations_with_nested_labels_angles_escapes_and_titles() {
    let markdown = concat!(
        "Before [ordinary](../config/runtime.yaml) and ",
        "[**nested**](<../config/profile (dev).json> \"quoted title\") and ",
        "[escaped](../config/run\\(time\\).yaml 'single title') and ",
        "[balanced](../config/(archive)/data.json (parenthesized title)).\n",
    );

    let rewritten = rewrite_chapter_links(&path("guide/setup.md"), markdown, &routes()).unwrap();

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

    let rewritten = rewrite_chapter_links(&path("guide/setup.md"), markdown, &routes()).unwrap();

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
fn inline_resolves_nested_relative_paths_and_direct_readme_sources() {
    let markdown = concat!(
        "[runtime](../../config/runtime.yaml) ",
        "[yaml index](../../README.yaml) ",
        "[json index](../../README.json)\n",
    );

    let rewritten =
        rewrite_chapter_links(&path("guide/deep/setup.md"), markdown, &routes()).unwrap();

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

    let rewritten = rewrite_chapter_links(&path("guide/setup.md"), markdown, &routes()).unwrap();

    assert_eq!(rewritten.as_bytes(), markdown.as_bytes());
}

#[test]
fn inline_leaves_external_fragment_and_html_destinations_unchanged() {
    let markdown = concat!(
        "[scheme](https://example.com/config/runtime.yaml) ",
        "[protocol relative](//example.com/config/runtime.yaml) ",
        "[fragment](#runtime) [html](../config/runtime.html)\n",
    );

    let rewritten = rewrite_chapter_links(&path("guide/setup.md"), markdown, &routes()).unwrap();

    assert_eq!(rewritten.as_bytes(), markdown.as_bytes());
}
