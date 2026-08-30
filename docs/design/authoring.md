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
preprocessor dispatches from `Chapter.source_path`, which is the real source
path, and leaves that field unchanged. The title supplied in `SUMMARY.md`
continues to be the chapter title.

Structured chapter URLs preserve the source extension. For example,
`config/runtime.yaml` is published as `config/runtime.yaml.html`. The
[route contract](routes.md) defines the logical-path shim, README/index
handling, and collision policy.

Only listed chapters are transformed. A Markdown link to an unlisted JSON or
YAML file does not create a structured page. Stock mdBook may nevertheless
copy any non-`.md` file under the source directory, including listed and
unlisted JSON/YAML files, into the output as a static file. V1 accepts this
stock publication behavior.

Links to listed structured chapters use the source path in authored Markdown:

```md
[Runtime configuration](config/runtime.yaml)
```

The separate [chapter-link rewriter](link-rewriting.md) maps that destination
to the generated HTML page. Relative paths, README/index handling, query
strings, and fragments otherwise follow mdBook's path conventions.

Extension-qualified README links remain distinct. If both `README.yaml` and
`README.json` are listed from one directory, authors can link to either source
name and reach its corresponding page. Their shared `index.md` and
directory-style convenience aliases are ambiguous when no actual chapter owns
the destination, however, so an authored link using either alias stops the
build instead of selecting a target by `SUMMARY.md` order.

Authors register the two preprocessor phases and the generated assets in
`book.toml`. The installer prints the required entries but never inserts them.
Asset registration remains an author decision. The complete configuration and
installer behavior are defined in [Configuration and operations](operations.md).
