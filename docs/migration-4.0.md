# Migrating to the next Poem release family

These versions are **prepared, not yet released**. Release dates must be filled in
when publication is approved. The release is based on merged `master` through
[dfc0a435](https://github.com/poem-web/poem/commit/dfc0a435), including dependency
upgrade [#1198](https://github.com/poem-web/poem/pull/1198) and TLS configuration
[#1199](https://github.com/poem-web/poem/pull/1199).

## Versions and compatibility boundaries

| Crate(s) | Previous version | Prepared version | Reason |
| --- | --- | --- | --- |
| `poem`, `poem-derive` | 3.1.12 | 4.0.0 | Public integration types/features change; paired runtime and macros |
| `poem-openapi`, `poem-openapi-derive` | 5.1.16 | 6.0.0 | Poem 4, external type implementations and generated schema changes |
| `poem-lambda` | 5.1.4 | 6.0.0 | Poem 4 and Lambda HTTP 1.x types |
| `poem-grpc`, `poem-grpc-build` | 0.5.9 | 0.6.0 | Poem 4 types in runtime/generated APIs; paired runtime and generator |
| `poem-mcpserver`, `poem-mcpserver-macros` | 0.3.1 | 0.4.0 | Public protocol/model and transport behavior changes; Poem 4 transport |
| `poem-worker` | 0.1.0 (repository version) | 0.2.0 | Poem 4 and Worker 0.8 types; still outside automatic publishing |

Pre-1.0 crates advance their minor version for an incompatible release; this does
not declare them 1.0-stable. All ten workspace crates require **Rust 1.94**,
including crates built with default features disabled. This replaces the former
1.85 minimum. Update CI, local and deployment toolchains first.

Update Poem-related dependencies together. A Poem 3 `Request`, `Response`,
`Endpoint` or middleware implementation is not interchangeable with the Poem 4
version even if its source looks identical. Check third-party Poem integrations
for compatible releases. Explicit derive or build dependencies need matching
versions too; normal users can keep using the macros re-exported by the runtime.

For example, an OpenAPI application should use:

```toml
[dependencies]
poem = "4"
poem-openapi = { version = "6", features = ["swagger-ui"] }
```

Cargo will accept these registry requirements only after the prepared versions
are published. Before publication, validate with local path dependencies or a
consistent Git revision for the entire Poem dependency family.

## Poem 4

Align direct dependencies whose types cross Poem's public boundary:

- **OpenTelemetry 0.33**: tracers passed to `OpenTelemetryTracing` and associated
  tracing/metrics integrations.
- **Redis 1.7**: connections supplied to `RedisStorage` and Redis error types.
- **tungstenite 0.30**: the re-exported `WebSocketConfig` and WebSocket conversion
  types. Prefer Poem's re-export when configuring its WebSocket extractor.
- **Fluent 0.17 / fluent-syntax 0.12**: values and errors used by the i18n API.
  Language negotiation remains `fluent-langneg` 0.13 with `unic-langid` 0.9;
  do not switch it to ICU identifiers as part of this upgrade.
- **quick-xml 0.42**: XML error types, including
  `poem::error::ParseXmlError::Parse`, now refer to the updated crate generation.

If your feature list explicitly included `rustls-pemfile`, remove it and enable
`rustls`. The former was an implicit optional-dependency feature and no longer
exists. PEM certificates and keys remain supported. Applications directly using
PEM parsing can use rustls's `pki_types::pem::PemObject` APIs.

`RustlsConfig::versions(&[&rustls::version::TLS13])` can now limit a listener to
TLS 1.3. The default remains rustls's default TLS 1.2 and TLS 1.3 selection. Empty
or provider-incompatible selections fail configuration loading; an invalid
streamed reload retains the last valid configuration.

Review behavior-sensitive tests and monitoring:

- Compression now honors `identity`, correctly recognizes `zstd`, and chooses an
  enabled algorithm for wildcard requests.
- Accepted TCP sockets enable `TCP_NODELAY`.
- HTTP/2 customizers run after individual server setters and can override them.
- Tracing records richer errors and server-error span status and no longer adds
  `telemetry.sdk.*` metadata to each request span. Prometheus exporter 0.33 uses
  scope schema/attribute labels in place of the old `otel_scope_info` metadata.
  Review dashboards against the new dependency family.

Tower moves to 0.5, but its shared `tower-layer`/`tower-service` traits remain on
0.3; this is not a blanket requirement to rewrite all Tower middleware. Reqwest,
RCGen and Rand migrations are internal to Poem. ACME root-source choices and
32-byte session entropy are preserved.

## OpenAPI 6

### Exact numeric bounds

`MetaSchema.minimum` and `MetaSchema.maximum` now use
`Option<registry::MetaSchemaNumber>` instead of `Option<f64>`. Use `.into()` in
struct literals: `minimum: Some(9_007_199_254_740_993_u64.into())`. The public
`Integer(i64)`, `Unsigned(u64)` and `Float(f64)` variants are also available.
Integer bounds serialize as JSON integers and preserve the full `i64`/`u64`
range. `multiple_of` remains `Option<f64>`.

`Minimum::new` and `Maximum::new` accept `impl Into<MetaSchemaNumber>`, so existing
calls such as `Maximum::new(10.0, false)` still work. Pass an integer argument to
retain integer precision, e.g. `Maximum::new(u64::MAX, false)`. A value already
rounded to `f64` cannot recover its original integer precision.

Minimum/maximum validators now require `Copy + Into<MetaSchemaNumber>` instead
of `num_traits::AsPrimitive<f64>`. Supported built-ins are `i8` through `i64`,
`u8` through `u64`, `isize`/`usize`, their `NonZero` counterparts, and `f32`/`f64`.
Custom numeric types must supply an exact conversion or use a custom validator;
`i128`/`u128` are not implicitly converted to lossy floating-point values.

Derive attributes accept quoted numeric strings and numeric literals, including
negative literals. Integer syntax is preserved exactly; values outside
`i64::MIN..=u64::MAX` are rejected rather than rounded to floating point. Decimal
and exponent syntax still uses `f64` precision, not arbitrary-precision decimal
arithmetic. For example, use `value = "9007199254740993"` or
`value = 9007199254740993` for an exact integer, not `value = "9007199254740993.0"`.

Comparisons no longer round integer inputs to floats, including when a bound is
fractional or exclusive. Requests previously accepted only because of rounding
can now fail validation. Regenerate OpenAPI snapshots and clients. Non-finite
float bounds are rejected by the derive, reject all runtime values when
constructed directly, and return a serialization error instead of emitting
invalid `null` numeric bounds. Non-finite input values also fail these validators.

### Generated schemas and registry literals

Discriminator/externally tagged union wrapper component names now use the Rust
variant identifier rather than its payload type. For example, a tagged union
`enum Result { Success(User) }` produces
`Result_Success` instead of `Result_User` for the variant component. Regenerate
OpenAPI snapshots and clients and update explicit references to these names.
Serialized discriminator tag values remain unchanged; discriminator mapping
`$ref` targets follow the new component names.

`registry::MetaInfo` has an `extensions` field. Add
`extensions: Default::default()` to complete struct literals, or retain
`..Default::default()` when appropriate. Prefer `OpenApiService::info_extension`
for adding information-object extensions.

Fixed-size array (`[T; LEN]`) constraints now use `minItems`/`maxItems` rather than
`minLength`/`maxLength`. Regenerate schema consumers and review validation
expectations. GeoJSON 1 also serializes an empty polygon as `[]` rather than
`[[]]`; the public `geo-types` dependency remains 0.7.

### Optional external types

- Use BSON **3** `ObjectId`, SQLx **0.9** `Json<T>` and prost-wkt-types **0.7**
  `Duration`/`Timestamp`/`Struct`/`Value`. Types from older dependency generations no longer
  receive these OpenAPI trait implementations.
- The newly added `ulid` feature targets ULID **3**; use `Ulid::generate()` to
  construct a new random value. This support was not part of OpenAPI 5.1.16.
- The newly added `camino` feature supports `Utf8Path` and `Utf8PathBuf`;
  standard `Path`/`PathBuf` and unit `()` support do not require those features.
- BSON's serde integration is enabled explicitly by Poem. If sharing values
  with a MongoDB driver, select that driver's BSON 3 compatibility feature.
- SQLx 0.9 separates runtime and TLS features and requires safe SQL query inputs.
  See the updated todo example when migrating application queries.

Childless union variants are supported, but not with `externally_tagged`.
Externally tagged unions now accept primitive payload types. Unit `()` JSON
values must be `null`; missing unit fields remain accepted.

## Lambda 6

Align direct Lambda HTTP/runtime dependencies with Lambda HTTP **1.x**. Its body
is non-exhaustive; the adapter preserves future variants as bytes, while keeping
empty/text/binary handling and HTTP request parts intact. Validate real Lambda
request/context integrations before deploying; local tests do not exercise AWS.

## gRPC 0.6

Update `poem-grpc` and `poem-grpc-build` to **0.6** together and regenerate any
checked-in generated code so its Poem types match version 4. Prost stays on
**0.14** (now 0.14.4); the 0.13-to-0.14 transition already belongs to the restored
0.5.8 history. Keep application Prost versions aligned with generated messages.

`JsonCodec` now prefers `application/grpc+json` instead of `application/json`
when choosing the request content type. Review custom codec/content-type matching and mixed
old/new JSON-codec clients and servers. `application/json` remains accepted.
The Protobuf codec retains its existing content types.

## MCP server 0.4

For hand-written protocol integrations:

- Add `meta: None` (or an intentional `ToolMeta`) to `protocol::tool::Tool`
  literals.
- Update exhaustive `Requests` matches for `PromptsGet`,
  `ResourcesTemplatesList` and `ResourcesRead`.
- Construct `Requests::ResourcesList` with `ResourcesListRequest`, no longer
  `PromptsListRequest`.
- `stdio` now has three type parameters and `streamable_http::endpoint` has
  four. Prefer inferred calls; update explicit turbofish arguments if used.
  `McpServer`'s additional handler parameters have defaults.
- `StructuredContent<Vec<T>>` and the corresponding `Result` form now panic
  during output-schema generation. Wrap arrays in an object, for example a
  `#[derive(Serialize, JsonSchema)] struct Items { items: Vec<Item> }` and return
  `StructuredContent<Items>`. Tool schemas also normalize nonstandard unsigned
  integer formats recursively.

Streamable HTTP retains its **five-minute idle timeout** by default, now
configurable. Use
`endpoint_with_config` and `Config { session_timeout: None }` only when indefinite
HTTP-only sessions are intentional; clients must explicitly `DELETE` those
sessions. Legacy SSE-created sessions are reclaimed when their stream closes.
POST-initialized Streamable HTTP sessions survive SSE detachment for reconnect
until timeout or `DELETE`. Keep-alives help detect disconnected clients.

Server information, disabled tools and static UI resources remain scoped to the
server factory result. Configurations identical to the first factory result
share storage across sessions; differing metadata and resource contents remain
isolated. Per-session mutable `Tools`, `Prompts` and `Resources` state is created
fresh in the factory. Do not use `disable_tools` as a per-user authorization
boundary: it filters the advertised tool list, not tool execution.

## Worker 0.2: not publication-ready

The repository adapter targets Worker **0.8**. Align any directly used Worker
`Env`, binding, request/response and error types, and use worker-build **0.8**.
The standalone example's generated entry point is `build/index.js`.

`poem-worker` is not included in `.github/workflows/release.yml`; this preparation
does not add it. No prior published `poem-worker` version was found in the crate
index during the release audit. Its 0.1.0 changelog entry records repository
introduction, not a registry publication.

The final release review restores the `wasm32-unknown-unknown` example build by
restricting Tokio's `net` feature to server-enabled builds and Unix targets.
Native Unix address APIs remain available with default features disabled.
CI now checks the standalone Worker example on its WASM target. This is a target
compilation check, not a Cloudflare deployment check.

## Changelog scope and release process

The family changelogs also cover their paired macro/generator crates:
[Poem](../poem/CHANGELOG.md), [OpenAPI](../poem-openapi/CHANGELOG.md),
[Lambda](../poem-lambda/CHANGELOG.md), [gRPC](../poem-grpc/CHANGELOG.md),
[MCP](../poem-mcpserver/CHANGELOG.md), [Worker](../poem-worker/CHANGELOG.md).

The audit starts from each crate family's previous release, rather than the old
repository tags: Poem 3.1.12 / OpenAPI 5.1.16 / Lambda 5.1.4 at `f3cfdd8b`, gRPC
0.5.9 at `54c42e32`, and MCP 0.3.1 at `82355cdc`. Previously missing historical
sections are restored separately so published work is not presented as new.
MCP's 0.3.0 date and 0.2.4 year typo are corrected, and its Schemars 1.0 upgrade
is attributed to 0.2.5 rather than 0.2.9.

The root workspace dependency requirements now match the prepared crate
versions, including the previously stale `poem-openapi-derive` lower bound.
Example workspaces retain local paths; installation snippets target the new
major versions. Lockfiles remain untracked, consistent with repository policy.

Preparing or merging these changes does not itself publish a release. The
existing release workflow is triggered by manifest changes pushed to the exact
`release` branch. Only move an approved release there when publication is
intended; verify registry versions and packaging first, publish paired
macro/build dependencies before their consumers, and record actual release
dates. Do not use a release-preparation branch named exactly `release`.

For the complete dependency-generation review, upstream references and preserved
behavior, see [dependency-upgrades.md](dependency-upgrades.md).
