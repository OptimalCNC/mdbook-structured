mod support;

use support::{ObservedContainer, ObservedNodeKind};

fn assert_link(links: &[support::ObservedLink], text: &str, href: &str) {
    assert!(
        links
            .iter()
            .any(|link| link.text == text && link.href == href)
    );
}

#[test]
fn stock_mdbook_build_preserves_routes_links_and_semantics() {
    let built = support::build_integration_fixture().unwrap();
    let output_root = built.output_root();

    for path in [
        "config/runtime.yaml.html",
        "config/index.yaml.html",
        "config/index.json.html",
        "config/index.html",
        "single/index.yaml.html",
        "config/runtime.yaml",
    ] {
        assert!(output_root.join(path).is_file(), "missing {path}");
    }

    let links = support::observe_page_links(&output_root.join("index.html")).unwrap();
    for (text, href) in [
        ("Runtime direct", "config/runtime.yaml.html"),
        ("YAML README direct", "config/index.yaml.html"),
        ("JSON README direct", "config/index.json.html"),
        ("Exact config index", "config/index.html"),
        ("Single README direct", "single/index.yaml.html"),
        ("Single index alias", "single/index.yaml.html"),
        ("Single directory alias", "single/index.yaml.html"),
        ("Runtime from include", "config/runtime.yaml.html"),
    ] {
        assert_link(&links, text, href);
    }

    let exact_links = support::observe_page_links(&output_root.join("config/index.html")).unwrap();
    assert_link(
        &exact_links,
        "Runtime from exact index",
        "runtime.yaml.html",
    );

    let runtime =
        support::observe_built_page(&output_root.join("config/runtime.yaml.html")).unwrap();
    assert_eq!(runtime.heading, "Runtime");
    assert_eq!(runtime.root_kind, ObservedNodeKind::Mapping);
    assert_eq!(runtime.node_kinds.len(), 16);
    for text in [
        "",
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-repeat-without-truncation",
        "line one\nline two",
        "{{#include missing.md}}",
    ] {
        assert!(runtime.scalar_texts.iter().any(|actual| actual == text));
    }
    assert_eq!(
        runtime.containers,
        vec![
            ObservedContainer {
                kind: ObservedNodeKind::Mapping,
                open: true,
                immediate_child_count: 3
            },
            ObservedContainer {
                kind: ObservedNodeKind::Mapping,
                open: true,
                immediate_child_count: 3
            },
            ObservedContainer {
                kind: ObservedNodeKind::Sequence,
                open: false,
                immediate_child_count: 2
            },
            ObservedContainer {
                kind: ObservedNodeKind::Mapping,
                open: false,
                immediate_child_count: 3
            },
            ObservedContainer {
                kind: ObservedNodeKind::Mapping,
                open: false,
                immediate_child_count: 4
            },
        ]
    );
    assert_eq!(runtime.original_summary, "Original source (YAML)");
    assert_eq!(runtime.original_format, "yaml");
    assert_eq!(
        runtime.original_source,
        include_str!("fixtures/integration-book/src/config/runtime.yaml")
    );
    assert_eq!(
        std::fs::read(output_root.join("config/runtime.yaml")).unwrap(),
        include_bytes!("fixtures/integration-book/src/config/runtime.yaml"),
    );
}
