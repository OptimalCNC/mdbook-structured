# Maintainer Guide

## Local checks

Run the same checks required by CI before merging or tagging:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --all-features --locked --no-deps
cargo package --locked -p mdbook-structured-core
cargo package --locked -p mdbook-structured
npm ci
npx playwright install --with-deps chromium
npm run test:browser
mdbook build .
```

## Versions and releases

Both crates share the workspace version and the CLI's registry dependency on
`mdbook-structured-core` must match it. From green `main`, create an annotated
`vX.Y.Z` tag. Versions on crates.io are immutable.

For the one-time `v0.1.0` bootstrap, protect the `crates-io-bootstrap`
environment and provide its `CARGO_REGISTRY_TOKEN` secret. Publish in order:
`mdbook-structured-core`, wait for its exact registry visibility, then
`mdbook-structured`. Check each package with:

```console
curl -sS https://crates.io/api/v1/crates/mdbook-structured-core/0.1.0
curl -sS https://crates.io/api/v1/crates/mdbook-structured/0.1.0
```

Configure Trusted Publishers for both crates with these exact fields:

```text
owner:       OptimalCNC
repository:  mdbook-structured
workflow:    .github/workflows/release.yml
environment: crates-io
```

After testing OIDC publication, remove the bootstrap workflow and revoke or
delete its long-lived token. The normal release workflow publishes core before
the CLI and retries index visibility delays with bounded polling.

## Hosted publication proof

Enable GitHub Pages with **Build and deployment: GitHub Actions**. The book is
published at [optimalcnc.github.io/mdbook-structured](https://optimalcnc.github.io/mdbook-structured/).
Inspect that URL after deployment, and inspect both [mdbook-structured docs.rs](https://docs.rs/mdbook-structured)
and [core docs.rs](https://docs.rs/mdbook-structured-core) after their
asynchronous builds complete. A local command or green workflow is not proof
that a hosted endpoint is live.

If a publish is interrupted, inspect the exact crates.io API version before
retrying. A transient index delay is retryable; a published version cannot be
overwritten. Yanking follows crates.io procedure and is not rollback.
