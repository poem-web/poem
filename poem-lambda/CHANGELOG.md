# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [6.0.0] - Unreleased

## Breaking changes

- Upgrade to Poem 4 and require Rust 1.94. [Migration guide](../docs/migration-4.0.md)
- Upgrade Lambda HTTP from 0.15 through 0.16 to 1.x. Align direct Lambda HTTP/runtime dependencies and the request/context/body types used by your application. [#1085](https://github.com/poem-web/poem/pull/1085), [#1198](https://github.com/poem-web/poem/pull/1198)

## Fixed

- Preserve bytes for future non-exhaustive Lambda body variants. Add regression coverage for empty, text and binary bodies and HTTP request parts. [#1198](https://github.com/poem-web/poem/pull/1198)

# [5.1.4] 2025-07-28

- Refresh the shared Poem dependency to 3.1.12; no Lambda-specific API changes. [f3cfdd8b](https://github.com/poem-web/poem/commit/f3cfdd8b)

# [5.1.3] 2025-06-06

- chore(deps): update lambda_http requirement from 0.13.0 to 0.15.0 [#1045](https://github.com/poem-web/poem/pull/1045)

# [5.1.2] 2025-03-24

- Update MSRV to `1.85.0`

# [5.1.1] 2024-11-20

- Update MSRV to `1.81.0`

# [5.0.0] 2024-03-30

- use AFIT instead of `async_trait`
- Bump `lambda_http` from `0.9` to `0.10`

# [1.3.47] 2022-10-19

- Bump lambda_http from `0.6.0` to `0.7.0`

# [1.3.41]

- Upgrade lambda_http version to `v0.6.0`

# [1.3.16] 2022-3-18

- Bump `lambda_http` from `0.4.1` to `0.5.1`. 

# [1.0.19] 2021-11-03

# [1.0.19] 2021-11-03

- Use Rust 2021 edition.

# [1.0.12] 2021-10-27

- Return the correct payload type to the gateway.

