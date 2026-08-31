# Documentation, CI, and Publication Design

**Status:** Approved in conversation; pending written-spec review
**Date:** 2026-08-31
**Repository:** `OptimalCNC/mdbook-structured`

## Goal

Give `mdbook-structured` a practical public entry point, a coherent published
mdBook, continuous verification for the Rust, mdBook, and browser surfaces, and
a guarded path to publish both workspace crates on crates.io and their API
documentation.

The work starts from the implemented workspace on `main` (`72b1332`). The
existing design book remains authoritative for v1 semantics; this change adds
reader and maintainer entry points around it and does not change the product
contract.

## Decisions

1. Add a repository-root `README.md` for installation, a first working example,
   links to published documentation, project status, and contributor commands.
2. Keep `docs/` as the mdBook source, but make its first pages task-oriented:
   Getting Started, Configuration, and Authoring. Put the existing normative
   chapters under a Design Reference branch.
3. Build the mdBook on pull requests and `main` pushes. Deploy it to GitHub
   Pages from `main` after changes to the book source or its deployment
   workflow.
4. Publish `mdbook-structured-core` before `mdbook-structured`, because the
   CLI crate depends on the library crate.
5. Use a protected, manually dispatched token workflow for the initial `v0.1.0`
   publication. After both crates exist, configure crates.io Trusted Publishing
   for the tag-driven release workflow and use short-lived OIDC tokens for all
   later versions.
6. Treat docs.rs as the automatic API-documentation publisher for both crates;
   the Pages book and docs.rs serve different audiences and are linked
   separately.

## Scope and non-goals

### In scope

- A root README with copyable installation and configuration examples.
- Three practical mdBook pages and a preserved Design Reference hierarchy.
- Package metadata required for discoverable crates.io and docs.rs entries.
- Rust, package, mdBook, and Chromium browser checks in GitHub Actions.
- GitHub Pages build/deploy workflow.
- One-time token bootstrap and repeatable OIDC release workflows.
- Maintainer documentation for versioning, first publication, trusted
  publisher setup, and release verification.

### Out of scope

- Changes to parser behavior, route resolution, HTML semantics, installer
  behavior, or the v1 feature set.
- Automatic version bumping, changelog generation, GitHub Release creation, or
  publishing from arbitrary branches.
- `mdbook test` as a structured-book acceptance check; v1 supports only the
  stock HTML renderer for transformed structured chapters.
- Automatic changes to repository settings, GitHub Pages enablement, crates.io
  ownership, or trusted-publisher registrations. Those require maintainer
  authority outside the repository.
- A coverage service or a new coverage denominator policy.

## Documentation architecture

### Ownership of public documents

Each document has one audience and one primary contract:

| Surface | Audience | Owns |
| --- | --- | --- |
| `README.md` | Repository visitor and prospective user | What the tool is, installation, shortest successful setup, links, status, and local contributor checks |
| `docs/README.md` | Book reader starting from the published site | Getting Started walkthrough and the first end-to-end book configuration |
| `docs/configuration.md` | Book author | Complete preprocessor/asset configuration, option defaults, and operational troubleshooting |
| `docs/authoring.md` | Book author | Practical file layout, chapter listing, source-path links, and route examples |
| `docs/design/*` | Implementer and reviewer | Normative v1 architecture, invariants, and deferred decisions already captured by the design book |
| `docs/maintainers.md` | Repository maintainer | CI expectations, release/tag procedure, crates.io bootstrap, Trusted Publishing, Pages, and docs.rs |
| `examples/self-contained/README.md` | Reader trying the fixture | The runnable example's own commands and observations; it stays example-local |

The root README is not a copy of the design book. `docs/README.md` is
rewritten as the practical landing page, while its current design overview is
preserved as `docs/design/overview.md` with only link and title adjustments
needed for the new navigation. No normative design paragraph is discarded.

### mdBook navigation

`docs/SUMMARY.md` remains hand-maintained. Its top-level order is:

```text
Getting Started          (docs/README.md)
Configuration            (docs/configuration.md)
Authoring                (docs/authoring.md)
Maintainer guide         (docs/maintainers.md)
Design Reference         (docs/design/overview.md)
  Authoring contract     (docs/design/authoring.md)
  Processing architecture (docs/design/architecture.md)
    Structured core
    Routes and collisions
    Chapter link rewriting
    HTML presentation
  Configuration and operations
  Verification
  Evolution
```

The exact existing design chapters remain linked once under Design Reference;
the practical pages link to them rather than replacing their normative text.
Cross-links are updated so no page points at the old root overview route.

### Practical content

The landing and authoring pages use the implemented command and configuration
surface:

```console
cargo install mdbook-structured
cd path/to/book
mdbook-structured install .
mdbook build
mdbook serve
```

The configuration example registers `render` after `index` and before `links`,
registers `rewrite-links` after `links`, restricts both to `html`, and includes
the four current resource options with their defaults. It explains that the
installer prints `additional-css` and `additional-js` entries but never edits
`book.toml`. It also explicitly calls out the supported `.json`, `.yaml`, and
`.yml` set, listed-chapter requirement, source-extension routes, and the
unsupported `mdbook test` renderer behavior.

The authoring page demonstrates a `SUMMARY.md` entry for a JSON or YAML source,
a source-path Markdown link, distinct `README.yaml` and `README.json` routes,
and the ambiguity rule for convenience aliases. It links to the formal route
and link-rewriting chapters for edge cases instead of restating their proofs.

The root README contains the same minimum setup in a shorter form, links to:

- the GitHub Pages book at
  `https://optimalcnc.github.io/mdbook-structured/`;
- the `mdbook-structured` and `mdbook-structured-core` docs.rs pages;
- both crates.io pages;
- the self-contained example; and
- the maintainer and design-reference pages.

Badges are limited to the stable public workflows (`CI`, `Publish Docs`, and
`Release`) plus the two crates.io version badges. The one-time bootstrap
workflow is deliberately not presented as a permanent health signal.

### API documentation

Both package manifests receive complete public metadata:

- shared `edition = "2024"`, `rust-version = "1.88"`, `license`, `repository`, and
  `homepage` values;
- package-specific descriptions;
- `readme = "../../README.md"`;
- package-specific `documentation` URLs;
- focused crates.io keywords and valid categories; and
- `[package.metadata.docs.rs]` configuration with all available features.

The workspace dependency on `mdbook-structured-core` includes a registry
version as well as its path, so Cargo can package and publish the CLI crate
after the core crate is visible on crates.io. The API docs build in CI with
`cargo doc --workspace --all-features --locked --no-deps`; publishing a crate
automatically requests its docs.rs build. The README and maintainer guide
distinguish docs.rs API pages from the user-facing Pages book.

The workspace MSRV is `1.88`, the current floor imposed by the pinned mdBook
0.5.4 crates in the lockfile (and compatible with the pinned YAML adapter).
The manifests and the MSRV check use that exact value; if a future dependency
raises the floor, the dependency change and the documented MSRV change are one
reviewed decision rather than an implicit CI drift.

## Continuous integration

### `CI` workflow

Create `.github/workflows/ci.yml` with `pull_request`, pushes to `main`, and a
manual `workflow_dispatch` trigger. Set repository-wide `contents: read`
permissions, `CARGO_TERM_COLOR=always`, and a per-ref concurrency group with
`cancel-in-progress: true`.

The stable Rust job uses `actions/checkout`, `dtolnay/rust-toolchain` with
`rustfmt` and `clippy`, and `Swatinem/rust-cache`. It runs these commands with
the lockfile:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --all-features --locked --no-deps
cargo package --locked -p mdbook-structured-core
cargo package --locked -p mdbook-structured
```

A browser/integration job on `ubuntu-latest` installs the pinned mdBook
`0.5.3`, sets up Node from `package-lock.json`, runs `npm ci`, installs the
Chromium browser required by Playwright, and runs:

```bash
npm run test:browser
```

The existing Playwright global setup builds the binary and a temporary
integration book, so this job covers the real preprocessor protocol, stock
mdBook output, installed assets, and browser disclosure behavior. A docs job
or a final step in the same workflow runs `mdbook build .` against the root
`book.toml`. This is the PR guard that prevents a broken book from reaching
`main`; it must not be deferred solely to deployment.

The workflow does not claim that `mdbook test` validates structured chapters.
No coverage job is added in this change.

### Documentation deployment

Create `.github/workflows/docs.yml` with a push trigger limited to `main` and
changes under `docs/**`, `book.toml`, or the workflow itself, plus
`workflow_dispatch`. The build job has only `contents: read`; it:

1. checks out the commit;
2. runs `actions/configure-pages`;
3. installs mdBook `0.5.3` with `--locked`;
4. runs `mdbook build .`;
5. uploads the generated `book/` directory with
   `actions/upload-pages-artifact`.

A separate deploy job needs the build and has only `pages: write` and
`id-token: write`. It uses `actions/deploy-pages` in the `github-pages`
environment and exposes the deployment URL. Concurrency is a single `pages`
group with `cancel-in-progress: false`, so a completed deployment is not
interrupted by a later docs commit.

The repository setting **Pages -> Build and deployment -> GitHub Actions** is
an external one-time prerequisite. The workflow does not add a `CNAME` or
custom domain; the canonical URL is the owner/repository Pages URL above.

## Crates.io publication

### Version and tag contract

Both workspace packages share one version. A release commit updates the
workspace version and the registry version requirement on the CLI's core
dependency, then updates `Cargo.lock` if Cargo changes it. A maintainer creates
an annotated `vX.Y.Z` tag only after the corresponding commit is on `main` and
the `CI` workflow is green. The release workflow verifies all of the following
before it requests a publish token:

- the tag matches `^v[0-9]+\\.[0-9]+\\.[0-9]+$`;
- the tag commit is reachable from `main`;
- both package versions equal the tag without its leading `v`; and
- both packages pass formatting, Clippy, tests, docs, and `cargo package`.

Crates.io versions are immutable. A release is complete only when the exact
version of both crates is visible through the crates.io API and is not yanked;
a successful GitHub Actions job alone is not publication proof.

### One-time `v0.1.0` bootstrap

Create `.github/workflows/bootstrap-release.yml` as a temporary, manually
dispatched workflow. It accepts a tag input but rejects every value other than
`v0.1.0`, checks out that exact tag, and runs the same validation/package steps
as the normal release. Its publish job is in a protected `crates-io-bootstrap`
environment with `contents: read` and a single environment secret named
`CARGO_REGISTRY_TOKEN`. The token is passed only to `cargo publish` through
`CARGO_REGISTRY_TOKEN`; it is never printed or written to the repository.

The bootstrap publishes in this exact order:

1. `mdbook-structured-core`;
2. wait, with bounded polling, until the exact core version is visible in the
   crates.io API/index;
3. `mdbook-structured`.

The publish helper checks for an already-visible exact version before upload,
so a rerun after a downstream failure does not attempt to overwrite an
immutable version. It fails on a version conflict whose registry metadata does
not match the requested package and reports the package/version explicitly.

After both packages are confirmed, the maintainer must configure a GitHub
Trusted Publisher for each crate with:

```text
owner:    OptimalCNC
repository: mdbook-structured
workflow: .github/workflows/release.yml
environment: crates-io
```

Only after those registrations are tested should the maintainer remove the
bootstrap workflow and revoke/delete its long-lived token secret. Until that
cutover, the bootstrap environment remains reviewer-protected and is never
triggered by an arbitrary push.

### Tag-driven OIDC releases

Create `.github/workflows/release.yml` with only a `push` trigger for
`v*.*.*` tags. Its validation job has `contents: read`, a non-cancelled
per-tag concurrency group, and the same stable Rust/package checks as CI. The
publish job needs validation, uses the protected `crates-io` environment, and
grants exactly `contents: read` plus `id-token: write`. It authenticates with
`rust-lang/crates-io-auth-action@v1`, passes the resulting short-lived token to
Cargo, publishes core, waits for exact registry/index visibility, and publishes
the CLI.

The workflow contains no `pull_request_target`, branch-controlled publish
input, or long-lived token. It does not create a GitHub Release. A failed core
publish is retried only after inspecting the exact crates.io version; if core
is already present, the downstream CLI publish may be resumed at the same tag.

### Maintainer runbook and external proof

`docs/maintainers.md` records the local preparation commands, tag sequence,
environment names, trusted-publisher fields, and post-release checks. It
defines two separate proof obligations:

- **Registry proof:** query `https://crates.io/api/v1/crates/<name>/<version>`
  for both packages and check the `yanked` field is false.
- **Documentation proof:** inspect the Pages deployment URL and both docs.rs
  URLs after their asynchronous builds complete.

No workflow or local command is described as proof that an external deployment
has completed until the corresponding hosted endpoint is checked.

## Failure and recovery boundaries

- Formatting, Clippy, test, browser, docs-build, or package failures block a
  merge or release and are fixed in source; they are not bypassed by changing
  assertions or dropping the relevant job.
- A transient crates.io index delay is retryable through the bounded polling
  helper. A successful immutable upload followed by a workflow interruption is
  resumed by checking the exact version and continuing with the next crate.
- A malformed package or wrong version is not silently retried. Correct it in a
  new commit/tag; never attempt to overwrite a published version.
- If an accidentally published version is unusable, the maintainer follows
  crates.io's yank procedure and treats the version as permanently archived;
  yanking is not presented as rollback or deletion.
- Pages deployment is recoverable by redeploying a later `main` commit. The
  generated `book/` directory remains ignored and is never committed.
- Any failure caused by missing GitHub Pages enablement, crates.io ownership,
  environment approval, or trusted-publisher configuration is reported as an
  external setup gate, not disguised as a passing local test.

## Acceptance criteria

The implementation is ready for review when:

1. `README.md` and the practical mdBook pages provide a copyable first build
   and link to the design reference, example, Pages book, docs.rs, and crates.io.
2. `docs/SUMMARY.md` has the approved user-first hierarchy and every linked
   chapter builds without stale routes.
3. Both manifests package successfully with the root README and registry
   dependency metadata.
4. Pull-request CI runs the full Rust, package, browser, and mdBook checks with
   a locked dependency graph.
5. Pages deployment builds the same root book from `main` with least-privilege
   build/deploy permissions.
6. The bootstrap workflow is limited to the protected `v0.1.0` manual path,
   publishes core before CLI, and documents the OIDC cutover.
7. The normal release workflow is tag-only, validates the tag/version/main
   contract, uses OIDC, and verifies registry visibility before completion.
8. `cargo doc` succeeds for both crates and the README exposes their docs.rs
   destinations.
9. Existing implementation tests and the untracked
   `docs/superpowers/plans/2026-08-30-mdbook-structured-implementation.md`
   remain untouched.
