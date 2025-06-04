# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [Unreleased]

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