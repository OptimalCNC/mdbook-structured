# Rendering gallery

This book combines ordinary Markdown navigation with JSON and YAML chapters.
Links to the structured chapters use their source paths; `mdbook-structured`
resolves those matches to the corresponding extension-preserving HTML pages.

- [Runtime configuration](config/runtime.yaml)
- [Mixed top-level fields](mixed-top-level.yaml)
- [YAML README route](config/README.yaml)
- [JSON README route](config/README.json)
- [Markdown index](exact/index.md)
- [Single README route](single/README.yaml)

The [runtime page](config/runtime.yaml) includes text that resembles an mdBook
helper and an HTML element. Both are rendered as literal data rather than
expanded or interpreted as markup.
