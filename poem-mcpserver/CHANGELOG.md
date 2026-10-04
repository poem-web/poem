# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [0.4.0] - Unreleased

This release also covers `poem-mcpserver-macros` 0.4.0. See the [major-release migration guide](../docs/migration-4.0.md).

## Breaking changes

- Require Rust 1.94 and use Poem 4 for the optional `streamable-http` transport. [#1198](https://github.com/poem-web/poem/pull/1198)
- Add `meta` to the public `Tool` struct and extend request types for prompts and resources. Update manual struct literals and exhaustive request matches. Explicit generic arguments to `stdio` and `streamable_http::endpoint` must account for prompt/resource handlers; inferred calls remain supported. [#1155](https://github.com/poem-web/poem/pull/1155), [#1160](https://github.com/poem-web/poem/pull/1160), [#1178](https://github.com/poem-web/poem/pull/1178)
- Require structured tool results to be JSON objects, as specified by MCP 2025-06-18. Wrap vectors and other non-object values in an object type. Invalid results return a tool error instead of panicking during output-schema generation. [#1172](https://github.com/poem-web/poem/pull/1172)

## Added

- Allow streamable-HTTP endpoint factories to return `Result<McpServer, E>` with `E: Into<poem::Error>`, propagating construction errors before creating a session. Existing infallible factories and four-parameter generic calls remain supported. [#1187](https://github.com/poem-web/poem/issues/1187)
- Add prompt handlers and the `#[Prompts]` macro, with stdio/streamable-HTTP examples and legacy SSE transport support. [#1155](https://github.com/poem-web/poem/pull/1155)
- Add MCP Apps UI resources and tool UI metadata. [#1160](https://github.com/poem-web/poem/pull/1160)
- Add resource listing, templates, reads and the `Resources` handler abstraction. [#1178](https://github.com/poem-web/poem/pull/1178)
- Add configurable streamable-HTTP session timeouts, retaining the existing default of five minutes of inactivity. [#1163](https://github.com/poem-web/poem/pull/1163)
- Accept Cline-style empty initialized parameters and nested cancellation parameters/`requestId`; add dedicated tracing targets for requests/responses. [#1167](https://github.com/poem-web/poem/pull/1167), [#1171](https://github.com/poem-web/poem/pull/1171)

## Fixed

- Avoid request-time panics when listing tools that return `StructuredContent<Vec<T>>` or its `Result` form. Only advertise output schemas with an explicit object root; reject non-object results and serialization failures with `isError: true` and no `structuredContent`. Object results, including nested arrays, remain supported.

- Preserve request-scoped server metadata and UI resource contents; share metadata across streamable-HTTP sessions only when all contents match.
- Keep Streamable HTTP POST responses on the POST connection when a GET event stream is attached or disconnects; retain legacy SSE response routing.
- Clean up closed and expired sessions, keep SSE reconnections working, add keep-alives to detect disconnected clients, stop background session tasks and reduce per-session metadata allocations. [#1170](https://github.com/poem-web/poem/pull/1170), [#1181](https://github.com/poem-web/poem/pull/1181), [#1183](https://github.com/poem-web/poem/pull/1183)
- Normalize nonstandard integer formats recursively in tool input/output schemas. [#1179](https://github.com/poem-web/poem/pull/1179)
- Refresh shared dependencies and migrate macro parsing to Syn 3. [#1198](https://github.com/poem-web/poem/pull/1198)

# [0.3.1] - 2025-10-13

- Add the `Json<T>` content wrapper to serialize values as text content. [bf367bdf](https://github.com/poem-web/poem/commit/bf367bdf)

# [0.3.0] - 2025-10-11

- Implement `IntoToolResponse` for `()` and `Result<(), E>`. [e52be9a3](https://github.com/poem-web/poem/commit/e52be9a3)

# [0.2.9] - 2025-10-10

- add support for output schema in tools.

# [0.2.8] - 2025-10-09

- Add `ContentsIter` for returning multiple contents. [a0d787b8](https://github.com/poem-web/poem/commit/a0d787b8)

# [0.2.7] - 2025-10-09

- Implement `IntoContent` for `Content`. [8cb54b70](https://github.com/poem-web/poem/commit/8cb54b70)

# [0.2.6] - 2025-10-09

- Add the `Content::ResourceLink` variant. [ff29eb6c](https://github.com/poem-web/poem/commit/ff29eb6c)

# [0.2.5] - 2025-07-28

- Upgrade Schemars from 0.9 to 1.0 and refresh the shared Poem dependency. [#1069](https://github.com/poem-web/poem/pull/1069), [f3cfdd8b](https://github.com/poem-web/poem/commit/f3cfdd8b)

# [0.2.4] 2025-06-06

- bump `schemars` to 0.9
- Fix typo [#1030](https://github.com/poem-web/poem/pull/1030)

# [0.2.3] 2025-05-07

- add `McpServer::disable_tools` method to disable specific tools.

# [0.2.2] 2025-05-03

- add with_server_info builder method [#1015](https://github.com/poem-web/poem/pull/1015)
