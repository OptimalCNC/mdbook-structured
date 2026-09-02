# Structured Routes

Route projection begins only after a chapter has been admitted as a registered
structured chapter. `render` takes the chapter's logical path after mdBook's
`index` phase and replaces its extension with `<source-extension>.md`. The
stock HTML renderer then replaces only that final `.md`:

```text
source_path                 post-index path        structured path           HTML output
config/runtime.yaml         config/runtime.yaml    config/runtime.yaml.md    config/runtime.yaml.html
schemas/manifest.json       schemas/manifest.json  schemas/manifest.json.md  schemas/manifest.json.html
README.yaml                 index.md               index.yaml.md             index.yaml.html
README.json                 index.md               index.json.md             index.json.html
```

The final `.md` is an integration shim for the stock renderer's
`with_extension("html")` rule. It is part of the admitted chapter's route
projection, not link rewriting. Applying the shim consistently also makes a
structured page's generated route independent of other book content.

`Chapter.source_path` remains the registered source identity. Consequently,
`README.yaml` and `README.json` project to distinct `index.yaml.html` and
`index.json.html` routes even though mdBook gives them the same interim
`index.md` path.

## Generated-route integrity

The route-integrity domain contains only pairs of registered structured source
identities and routes projected for them. A registered chapter that lacks the
metadata required to construct its pair produces a diagnostic scoped to that
chapter; it is not silently removed from the structured target set.

Any additional integrity rule must be defined entirely in terms of those
registered source-to-route pairs, and its diagnostic may identify only values
from those pairs. No other chapter, route, or file enters route projection or
its integrity checks; mdBook owns their routing and publication.

The resulting structured source-to-output projection supplies the target index
used by [structured-target link rewriting](link-rewriting.md).
