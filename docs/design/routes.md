# Routes and Collisions

The route shim preserves a structured source's extension while continuing to
use the stock HTML renderer. `render` takes the logical path established after
mdBook's `index` phase and replaces its extension with
`<source-extension>.md`. The stock renderer then replaces only that final
`.md`:

```text
source_path                 post-index path        structured path           HTML output
config/runtime.yaml         config/runtime.yaml    config/runtime.yaml.md    config/runtime.yaml.html
schemas/manifest.json       schemas/manifest.json  schemas/manifest.json.md  schemas/manifest.json.html
README.yaml                 index.md               index.yaml.md             index.yaml.html
```

The final `.md` is an integration shim for the stock renderer's
`with_extension("html")` rule. It is applied consistently to every structured
chapter, not only when a collision is detected, so published URLs do not
depend on the presence of another file.

The real source remains in `Chapter.source_path`, so edit links and diagnostics
continue to point to `config/runtime.yaml` or `schemas/manifest.json`. An
ordinary `settings.md` chapter and a structured `settings.yaml` chapter
therefore produce `settings.html` and `settings.yaml.html`.

## Preflight

Before mutating any chapter, `render` projects every final chapter
destination. Ordinary chapters use their existing logical path; structured
chapters use the source-extension shim. The preflight then applies mdBook's
`with_extension("html")` rule and rejects duplicate final chapter routes,
naming every conflicting source chapter.

Exact projected-route collisions remain possible, most notably between
`settings.yaml` and an authored `settings.yaml.md` chapter. The preflight fails
with every conflicting source listed. Routes are deterministic and never
change in response to a collision.

A projected structured HTML route that still contains a literal `.md`
substring is also rejected because the stock renderer would corrupt generated
navigation links to it. Finally, the preflight rejects the narrow case in
which a projected chapter destination exactly matches a non-`.md` source file
that stock mdBook would copy to that destination. Exact candidate-path probes
for this check do not discover or transform unlisted chapters.

The preflight does not claim to inventory every stock renderer artifact,
theme asset, redirect, or internal route. Those remain mdBook's
responsibility; the plugin's guarantee covers its projected chapter routes
and exact static-source conflicts with those routes.

The resulting route map is the authority consumed by
[chapter-link rewriting](link-rewriting.md).
