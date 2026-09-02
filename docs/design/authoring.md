# Authoring Contract

An author may list a structured source directly in `SUMMARY.md`, alongside
ordinary Markdown chapters:

```md
# Summary

- [Introduction](README.md)
- [Runtime configuration](config/runtime.yaml)
- [Protocol manifest](schemas/manifest.json)
- [Deployment guide](deployment.md)
```

mdBook loads each listed target as UTF-8 chapter text. The structured
preprocessor registers a chapter when its `Chapter.source_path` has the exact
lowercase extension `.json`, `.yaml`, or `.yml`. That field remains the source
identity, and the preprocessor leaves it unchanged. The title supplied in
`SUMMARY.md` continues to be the chapter title.

Structured chapter URLs preserve the source extension. For example,
`config/runtime.yaml` is published as `config/runtime.yaml.html`. The
[structured route contract](routes.md) defines the logical-path shim and the
integrity of routes generated for registered structured chapters.

Merely linking to a JSON or YAML source does not register it. Registration
comes from the chapter metadata in the received `Book`; the preprocessor does
not discover sources from the filesystem. Only a registered source can enter
structured transformation or the structured target index.

Links to registered structured chapters use the source path in authored
Markdown:

```md
[Runtime configuration](config/runtime.yaml)
```

The separate [structured-target link rewriter](link-rewriting.md) maps that
destination to the generated HTML page. It resolves the authored path
lexically from the current chapter and preserves the query, fragment, title,
and surrounding Markdown bytes.

Extension-qualified README links remain distinct. If both `README.yaml` and
`README.json` are listed from one directory, authors can link to either source
name and reach `index.yaml.html` or `index.json.html`, respectively. These
extension-qualified source identities are the keys used for target matching.

Authors register the two preprocessor phases and the generated assets in
`book.toml`. The installer prints the required entries but never inserts them.
Asset registration remains an author decision. The complete configuration and
installer behavior are defined in [Configuration and operations](operations.md).
