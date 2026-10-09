# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [0.6.0] - Unreleased

This changelog also covers `poem-grpc-build` 0.6.0.

## Breaking changes

- Upgrade to Poem 4 and require Rust 1.94. Update `poem-grpc` and `poem-grpc-build` together, then regenerate checked-in generated code. [Migration guide](../docs/migration-4.0.md)
- Use the 0.6 runtime/build release line for generated APIs referring to Poem 4 types. Prost remains on 0.14; this release updates it to 0.14.4 and migrates the generator to Syn 3/prettyplease 0.3. [#1198](https://github.com/poem-web/poem/pull/1198)

## Fixed

- Prefer `application/grpc+json` over `application/json` in `JsonCodec::CONTENT_TYPES`, so clients send the gRPC JSON content type. Both content types remain accepted. [#1191](https://github.com/poem-web/poem/pull/1191)
- Serve original reflection descriptor bytes (preserving unknown fields), return transitive file dependencies, and answer extension queries for both `grpc.reflection.v1` and `v1alpha`. [#1196](https://github.com/poem-web/poem/pull/1196)

## Changed

- Reduce the size of `Status` by boxing its private metadata; public accessors remain unchanged. [#1191](https://github.com/poem-web/poem/pull/1191)

# [0.5.9] 2025-12-23

- Add `poem_grpc_build::Config::enum_attribute` and `message_attribute` for generated enum/oneof and message attributes. [54c42e32](https://github.com/poem-web/poem/commit/54c42e32)

# [0.5.8] 2025-12-23

- Upgrade Prost/prost-build/prost-types from 0.13 to 0.14; update the Tonic interop example dependency. Keep the generated-code and application Prost versions aligned. [#1078](https://github.com/poem-web/poem/pull/1078)

# [0.5.7] 2025-07-28

- Refresh the shared Poem dependency to 3.1.12. [f3cfdd8b](https://github.com/poem-web/poem/commit/f3cfdd8b)

# [0.5.6]

- Bump `webpki-roots` to 1.0

# [0.5.5] 2025-05-03

- poem-grpc-build: add more methods to config [#1025](https://github.com/poem-web/poem/pull/1025)

# [0.5.4] 2025-03-24

- Update MSRV to `1.85.0`

# [0.5.3] 2025-01-04

- feat: Implement enable_type_name config method [#924](https://github.com/poem-web/poem/pull/924)

# [0.5.2] 2024-11-20

- Add `ClientConfigBuilder::http2_max_header_list_size` method to set the max size of received header frames.
- Update MSRV to `1.81.0`

# [0.5.1] 2024-09-12

- set the correct `content-type` for `GrpcClient`

# [0.5.0] 2024-09-08

- add support for GRPC compression

# [0.4.2] 2024-07-19

- Fix #840: Grpc build emit package when package is empty [#841](https://github.com/poem-web/poem/pull/841)
- chore: bump prost to 0.13 [#849](https://github.com/poem-web/poem/pull/849)

# [0.4.1] 2024-05-18

- message can span multiple frame [#817](https://github.com/poem-web/poem/pull/817)