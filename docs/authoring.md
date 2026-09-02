# Authoring

Structured pages are registered from `SUMMARY.md` entries whose source paths
have the exact lowercase extension `.json`, `.yaml`, or `.yml`:

```markdown
- [Runtime YAML](config/runtime.yaml)
- [Runtime JSON](config/runtime.json)
```

Links in ordinary Markdown should use those source paths. The generated pages
preserve the extension, for example `config/runtime.yaml.html` and
`config/runtime.json.html`. A parser-confirmed Markdown link destination is
rewritten when relative resolution matches a registered structured source.

`README.yaml` and `README.json` are distinct registered source identities.
Direct references to them reach `index.yaml.html` and `index.json.html`,
respectively.

See the formal [authoring contract](design/authoring.md), [route rules](design/routes.md),
and [structured-target link rewriting](design/link-rewriting.md) chapters for complete
registration, route-projection, and target-matching behavior.
