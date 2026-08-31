# Authoring

Structured pages are selected by `SUMMARY.md`. Entries can point directly to
JSON or YAML sources:

```markdown
- [Runtime YAML](config/runtime.yaml)
- [Runtime JSON](config/runtime.json)
```

Links in ordinary Markdown should use those source paths. The generated pages
preserve the extension, for example `config/runtime.yaml.html` and
`config/runtime.json.html`.

`README.yaml` and `README.json` are distinct routes, as are
`index.yaml.html` and `index.json.html`. Exact source-path links always win.
Convenience README/index/directory aliases are created only when unique; an
ambiguous alias is an authored error and fails the build rather than choosing
silently.

See the formal [authoring contract](design/authoring.md), [route rules](design/routes.md),
and [chapter link rewriting](design/link-rewriting.md) chapters for complete
precedence and collision behavior.
