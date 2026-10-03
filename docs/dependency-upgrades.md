# Dependency upgrade migration guide

This development update targets stable direct dependencies available on 2026-10-02.
Many patch/minor requirement changes only refresh lower bounds which Cargo could
already resolve to current releases. Incompatible version-line changes need the
consumer review below; this is not a backwards-compatibility guarantee.

## Required application changes

- **Rust 1.94 minimum**: update local, CI and deployment toolchains. SQLx 0.9 sets
  this floor. All workspace crates inherit it; READMEs and a dedicated CI check
  reflect it. The [major-release migration guide](migration-4.0.md) records
  the prepared crate versions and additional changes since their last releases.
- **OpenAPI external types**: use BSON 3 ObjectId, ULID 3 Ulid and SQLx 0.9 Json<T>.
  Older crate generations are different Rust types and no longer get these trait
  implementations. BSON serde is now opt-in and enabled by Poem. When combining
  MongoDB BSON values with OpenAPI, select the driver's BSON 3 compatibility
  feature; the standalone MongoDB example intentionally uses its default BSON 2.
- **Poem integration types**: align OpenTelemetry 0.33, tungstenite 0.30
  WebSocketConfig and Fluent 0.17/fluent-syntax 0.12 dependencies. These identities
  can appear in public APIs, so changing only Poem may not suffice.
- **Worker 0.8**: align direct worker dependencies, including Env/binding/error
  types; use worker-build ^0.8. The generated entry point is build/index.js.
- **Lambda HTTP 1.x**: align request/context/body ecosystem dependencies. Unknown
  future body variants are preserved as bytes rather than discarded.

## Behavior to review before release

- Prometheus 0.33 changes default scope metadata: scope schema/attribute labels
  replace the otel_scope_info metric. Review dashboard queries. OTLP enables
  retries by default and validates endpoint configuration more strictly.
- GeoJSON 1 changes error diagnostics and serializes empty polygons as [] rather
  than [[]]. Public geo-types 0.7 types and OpenAPI schemas remain unchanged.
- Reqwest 0.13 removes old TLS-root features. Poem preserves native-only,
  Mozilla-only and combined root choices using explicit certificate APIs.
  RCGen 0.14 separates keys and certificates; P-256 keys, SANs, CSR behavior and
  the critical ACME extension are preserved and covered by regression tests.
- ULID construction becomes Ulid::generate(). MongoDB 3.9 requires server 4.4+.
  Tera 2 needs explicit filesystem loading; the example explicitly enables HTML
  template escaping. SQLx 0.9 uses separate runtime/TLS features and safe SQL;
  the todo example uses a static bound statement for partial/empty updates.
- Syn 3 receiver mutability moved into its receiver-kind AST. Macros still reject
  &mut self and mut self; regression tests cover both forms.
- The RMCP example retains supported legacy initialization for Poem interoperability.

## Intentional exceptions

- fluent-langneg remains 0.13.1: 0.14 uses ICU identifiers, while Fluent 0.17 and
  Poem's public APIs use unic_langid. Conversion-based API redesign is deferred.
- JWT examples retain hmac 0.12.1 and sha2 0.10.9: jwt 0.16 requires their digest
  0.10 traits; hmac 0.13/sha2 0.11 are incompatible substitutions.
- serde_yaml 0.9.34 is latest but deprecated upstream. Replacing the library is a
  separate dependency choice, not an available version upgrade.

## Official breaking-release review

Each semver-incompatible line was reviewed against official upstream release notes
or migration guides, including intermediate 0.x lines. Missing current upstream
changelogs are called out explicitly below.

| Upgrade | Upstream source and migration decision |
| --- | --- |
| base64 0.23 | [Release notes](https://github.com/marshallpierce/rust-base64/blob/master/RELEASE-NOTES.md): changed error payload, padding, SIMD and MSRV; existing explicit GeneralPurpose engines retained. |
| BCS 0.2 | [Source changes](https://github.com/zefchain/bcs/compare/1d03bda49fd006289352c0e022aee4d71945668b...a66ea2cb50ff99a66434351b6a00ac5913d0ae02): shipped changelog stops before this release; Serde-core migration preserves from_bytes/to_bytes; wire regressions added. |
| BSON 3 | [Migration](https://github.com/mongodb/bson-rust/blob/main/migration-3.0.md): explicit serde feature; ObjectId JSON/parameter/header behavior tested. |
| Darling 0.21–0.24 | [Changelog](https://github.com/TedDriggs/darling/blob/master/CHANGELOG.md): attribute-overlap and Syn 3 changes reviewed; Poem attribute/forwarding sets do not overlap. |
| Syn 3 / prettyplease 0.3 | [Syn release](https://github.com/dtolnay/syn/releases/tag/3.0.0), [prettyplease release](https://github.com/dtolnay/prettyplease/releases/tag/0.3.0): AST path/receiver changes adapted; generator uses matching Syn 3 types. |
| Fluent 0.17 / syntax 0.12 | [Fluent](https://github.com/projectfluent/fluent-rs/releases/tag/fluent%400.17.0), [bundle](https://github.com/projectfluent/fluent-rs/releases/tag/fluent-bundle%400.16.0), [syntax](https://github.com/projectfluent/fluent-rs/releases/tag/fluent-syntax%400.12.0): parser/error versions aligned; language negotiation exception above. |
| GeoJSON 1 | [Changelog](https://github.com/georust/geojson/blob/main/CHANGES.md): removed Value conversion traits replaced by Serde; all geometry shapes and invalid input tested. |
| JWT 0.16 / HMAC 0.12 / SHA2 0.10 | [JWT API](https://docs.rs/jwt/0.16.0/jwt/), [HMAC](https://github.com/RustCrypto/MACs/blob/hmac-v0.12.1/hmac/CHANGELOG.md), [SHA2](https://github.com/RustCrypto/hashes/blob/sha2-v0.10.9/sha2/CHANGELOG.md): JWT has no current release changelog; published source requirements inspected; NewMac→Mac, signing/wrong-key tests. |
| itertools 0.15 | [Changelog](https://github.com/rust-itertools/itertools/blob/master/CHANGELOG.md#0150): Position/all_equal_value changes do not affect Either usage. |
| Lambda HTTP 1 | [1.0 release](https://github.com/aws/aws-lambda-rust-runtime/releases/tag/v1.0), [changelog](https://github.com/aws/aws-lambda-rust-runtime/blob/lambda_http-v1.3.1/lambda-http/CHANGELOG.md): non-exhaustive Body handled; body and HTTP parts preservation tested. |
| MongoDB 3 | [Upgrade guide](https://www.mongodb.com/docs/drivers/rust/current/reference/upgrade/), [release notes](https://www.mongodb.com/docs/drivers/rust/current/reference/release-notes/): fluent operations and server minimum updated. |
| nix 0.31 | [Changelog](https://docs.rs/crate/nix/0.31.3/source/CHANGELOG.md): removed platform/signal APIs unused; chown/Uid/Gid unchanged. |
| OpenTelemetry 0.32–0.33 | [API](https://docs.rs/crate/opentelemetry/0.33.0/source/CHANGELOG.md), [HTTP](https://docs.rs/crate/opentelemetry-http/0.33.0/source/CHANGELOG.md), [SDK](https://docs.rs/crate/opentelemetry_sdk/0.33.0/source/CHANGELOG.md), [OTLP](https://docs.rs/crate/opentelemetry-otlp/0.33.0/source/CHANGELOG.md), [semantic conventions](https://docs.rs/crate/opentelemetry-semantic-conventions/0.33.0/source/CHANGELOG.md), [Prometheus](https://docs.rs/crate/opentelemetry-prometheus/0.33.0/source/CHANGELOG.md): coordinated family update, including Prometheus from 0.29; metadata/retries reviewed. |
| rand 0.10 | [Changelog](https://github.com/rust-random/rand/blob/0.10.3/CHANGELOG.md): Rng→RngExt; session entropy width stays 32 bytes. |
| RCGen 0.13–0.14 | [0.13 migration](https://github.com/rustls/rcgen/blob/main/rcgen/docs/0.12-to-0.13.md), [0.14 release](https://github.com/rustls/rcgen/releases/tag/v0.14.0): explicit key/issuance/DER APIs; certificate regression tests. |
| Reqwest 0.13 | [Changelog](https://github.com/seanmonstar/reqwest/blob/v0.13.5/CHANGELOG.md): TLS-root features migrated; optional form/query helpers unused. |
| RMCP 2–3 | [2.x migration](https://github.com/modelcontextprotocol/rust-sdk/discussions/926), [3.x migration](https://github.com/modelcontextprotocol/rust-sdk/discussions/969): model/init changes reviewed; supported legacy client flow retained. |
| SQLx 0.9 | [Changelog](https://github.com/transact-rs/sqlx/blob/v0.9.0/CHANGELOG.md): Rust 1.94, safe SQL, feature split; Json and SQLite update regressions. |
| Tera 2 | [Migration](https://github.com/Keats/tera/blob/master/MIGRATION.md): constructor/filesystem loading and escaping migrated. |
| tokio-metrics 0.5 | [Changelog](https://docs.rs/crate/tokio-metrics/0.5.2/source/CHANGELOG.md): runtime histogram changes do not affect TaskMonitor. |
| tokio-tungstenite 0.28–0.30 | [Adapter](https://docs.rs/crate/tokio-tungstenite/0.30.0/source/CHANGELOG.md), [tungstenite](https://docs.rs/crate/tungstenite/0.30.0/source/CHANGELOG.md): error boxing/handshake changes reviewed; Poem's own HTTP upgrade extractor is unchanged. |
| Tower 0.5 | [Changelog](https://github.com/tower-rs/tower/blob/tower-0.5.3/tower/CHANGELOG.md): RateLimitLayer unchanged; Buffer generic changes inferred. |
| ULID 2–3 | [Changelog](https://github.com/dylanhart/ulid-rs#changelog): rand/overflow/constructor changes; parsing/display tests retained. |
| Worker 0.7–0.8 | [0.7](https://github.com/cloudflare/workers-rs/releases/tag/v0.7.0), [0.8](https://github.com/cloudflare/workers-rs/releases/tag/v0.8.0), [0.8.7](https://github.com/cloudflare/workers-rs/releases/tag/v0.8.7): KV/durable storage/event/build changes reviewed; output entry documented. |
| Tonic 0.14 completion | [Release](https://github.com/grpc/grpc-rust/releases/tag/v0.14.0): disabled example's existing incomplete migration repaired with tonic-prost and tonic-prost-build. |

Local compilation and regression tests do not establish production interoperability
with AWS, Cloudflare, MongoDB, GitHub OAuth or an OTLP collector. Validate deployment
services before release.

## Worker target validation

The final release review corrected a regression introduced after Poem 3.1.12:
Tokio's `net` feature is needed for native server listeners and Unix address
APIs, but must not be enabled unconditionally on `wasm32-unknown-unknown`.
Poem now enables it through the server feature and its Unix target dependency.
The standalone Worker example passes `cargo check --target wasm32-unknown-unknown`,
and CI checks that target alongside native Poem without default features. This
does not establish a successful Cloudflare deployment or change publication policy.
