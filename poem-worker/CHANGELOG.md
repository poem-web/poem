# Changelog

# [0.2.0] - Unreleased

## Breaking changes

- Upgrade to Poem 4 and require Rust 1.94. [Migration guide](../docs/migration-4.0.md)
- Upgrade Worker from 0.6 to 0.8. Align direct Worker dependencies used for `Env`, bindings, requests, responses and errors. The standalone example uses worker-build 0.8 and the `build/index.js` entry point. [#1198](https://github.com/poem-web/poem/pull/1198)

## Release status

- `poem-worker` is outside the automated crate-publishing workflow. This version bump records its incompatible dependency boundary; it does not enable publication.
- Restore compilation of the standalone `wasm32-unknown-unknown` example by avoiding Poem's unconditional Tokio/Mio networking features. CI covers the WASM target and native Poem without server features. Target checks do not establish a successful Cloudflare deployment. See the [dependency migration notes](../docs/dependency-upgrades.md#worker-target-validation).

# [0.1.0] - Repository introduction (2025-07-28)

- Add the Cloudflare Worker adapter, environment/context extractors and an example. [#1075](https://github.com/poem-web/poem/pull/1075)
- Adapt to Worker 0.6 request conversion changes. [#1077](https://github.com/poem-web/poem/pull/1077)
