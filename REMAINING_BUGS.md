# Poem Framework - Open Bug Analysis

This document categorizes all open bug issues in the poem repository as of December 2024.

## Summary

| Category | Count |
|----------|-------|
| Fixed (PRs submitted) | 7 |
| Fixed (already in codebase) | 5 |
| Still exists - needs fix | 24 |
| Environment/user-specific | 5 |
| **Total Open Issues** | **41** |

---

## Fixed Issues (PRs Submitted)

These issues have fixes submitted via pull requests:

| Issue | Title | PR |
|-------|-------|-----|
| [#1122](https://github.com/poem-web/poem/issues/1122) | MaybeUndefined nullable attribute not working | #1125 |
| [#1119](https://github.com/poem-web/poem/issues/1119) | Box/Arc support in Union derive | #1130 |
| [#1117](https://github.com/poem-web/poem/issues/1117) | TLS 1.3 only mode | #1128 |
| [#1105](https://github.com/poem-web/poem/issues/1105) | IntoResponse for Either type | #1129 |
| [#1098](https://github.com/poem-web/poem/issues/1098) | Root path misrouting with nested routes | #1124 |
| [#1036](https://github.com/poem-web/poem/issues/1036) | Path parameter parsing with type aliases | #1126 |
| [#1000](https://github.com/poem-web/poem/issues/1000) | XML response ignores serde attributes | #1127 |

---

## Fixed Issues (Already in Codebase)

These issues appear to already be fixed and can likely be closed:

| Issue | Title | Evidence |
|-------|-------|----------|
| [#1072](https://github.com/poem-web/poem/issues/1072) | time feature missing macros | `macros` feature now included in time dependency in Cargo.toml |
| [#1028](https://github.com/poem-web/poem/issues/1028) | Empty project build failure | Tested - builds successfully with current codebase |
| [#992](https://github.com/poem-web/poem/issues/992) | style: null field attribute | Test exists at `poem-openapi/tests/api.rs:1085` confirming fix |
| [#800](https://github.com/poem-web/poem/issues/800) | Union IsObjectType trait | Implementation exists at `poem-openapi-derive/src/union.rs:363` |
| [#799](https://github.com/poem-web/poem/issues/799) | Union generics support | Test passes at `poem-openapi/tests/union.rs:507` |

---

## Still Exists - Needs Fix

### OpenAPI Schema/Spec Issues

| Issue | Title | Description | Difficulty |
|-------|-------|-------------|------------|
| [#1032](https://github.com/poem-web/poem/issues/1032) | CQRS duplicate mapping key | Merging APIs with same path produces duplicate YAML keys in OpenAPI spec | Medium |
| [#923](https://github.com/poem-web/poem/issues/923) | OpenAPI path not merging | Same root cause as #1032 - paths not properly merged | Medium |
| [#698](https://github.com/poem-web/poem/issues/698) | Union of 2 APIs same endpoint | Same root cause as #1032 | Medium |
| [#760](https://github.com/poem-web/poem/issues/760) | Unsigned integer format | Uses non-standard `uint16/32/64` format instead of `minimum`/`maximum` constraints | Easy |
| [#519](https://github.com/poem-web/poem/issues/519) | $ref not RFC3986 compliant | Generic types produce `<>` characters in $ref URIs | Medium |
| [#318](https://github.com/poem-web/poem/issues/318) | Generics invalid OpenAPI identifiers | Same as #519 - generic type names not URL-safe | Medium |
| [#960](https://github.com/poem-web/poem/issues/960) | Union example not supported | `#[oai(example)]` attribute fails on Union types | Medium |
| [#945](https://github.com/poem-web/poem/issues/945) | Union flatten documentation | Schema missing fields when using flatten in docs | Medium |
| [#913](https://github.com/poem-web/poem/issues/913) | Option<T> ToJSON mismatch | Returns `null` but schema says field is optional (should be absent) | Medium |

### Validation Issues

| Issue | Title | Description | Difficulty |
|-------|-------|-------------|------------|
| [#669](https://github.com/poem-web/poem/issues/669) | NewType validator not working | Validator attribute compiles but doesn't actually validate | Medium |
| [#650](https://github.com/poem-web/poem/issues/650) | Optional path + validation | Validation fails on missing optional path parameters | Medium |
| [#649](https://github.com/poem-web/poem/issues/649) | Form data returns 400 | POST with form data always fails validation | Medium |

### Derive Macro Issues

| Issue | Title | Description | Difficulty |
|-------|-------|-------------|------------|
| [#699](https://github.com/poem-web/poem/issues/699) | Missing PARAM_IS_REQUIRED in Multipart | Multipart derive missing required const | Easy |
| [#826](https://github.com/poem-web/poem/issues/826) | Path params in prefix_path | `:name` syntax not converted to `{name}` in OpenAPI prefix | Easy |

### Security Scheme Issues

| Issue | Title | Description | Difficulty |
|-------|-------|-------------|------------|
| [#967](https://github.com/poem-web/poem/issues/967) | bad_request_handler not overridable | Custom handler ignored in security schemes | Medium |
| [#870](https://github.com/poem-web/poem/issues/870) | Security scheme error handling | First scheme error swallowed when multiple schemes fail | Medium |
| [#726](https://github.com/poem-web/poem/issues/726) | Enum SecurityScheme errors | Same as #870 - error handling issues | Medium |
| [#796](https://github.com/poem-web/poem/issues/796) | Cookie SecurityScheme key_name | `key_name` attribute ignored for cookie-based auth | Easy |

### Core Poem Issues

| Issue | Title | Description | Difficulty |
|-------|-------|-------------|------------|
| [#955](https://github.com/poem-web/poem/issues/955) | Trait objects in handler | `Data<&Box<dyn Trait>>` not found in request extensions | Medium |
| [#661](https://github.com/poem-web/poem/issues/661) | Graceful shutdown hang | `notify_waiters` not called, shutdown hangs indefinitely | Hard |
| [#653](https://github.com/poem-web/poem/issues/653) | Rustls livelock | 100% CPU when PEM files in wrong order | Hard |
| [#490](https://github.com/poem-web/poem/issues/490) | Regex routing panic | Byte index out of bounds on certain regex patterns | Medium |
| [#417](https://github.com/poem-web/poem/issues/417) | Higher-ranked lifetime error | Rust compiler limitation with certain async patterns | Hard |

### gRPC Issues

| Issue | Title | Description | Difficulty |
|-------|-------|-------------|------------|
| [#538](https://github.com/poem-web/poem/issues/538) | gRPC missing Content-Type | Response lacks required Content-Type header | Easy |

---

## Environment/User-Specific Issues

These issues are likely not code bugs but environment, documentation, or user-specific:

| Issue | Title | Notes |
|-------|-------|-------|
| [#1018](https://github.com/poem-web/poem/issues/1018) | Auth example WSL2 | Platform-specific networking issue |
| [#922](https://github.com/poem-web/poem/issues/922) | URL extraction only path | By design - `Uri` extractor only has path component |
| [#912](https://github.com/poem-web/poem/issues/912) | Docs not in sync | Documentation maintenance issue |
| [#881](https://github.com/poem-web/poem/issues/881) | SSE Nginx 25s delay | Infrastructure/proxy configuration issue |
| [#995](https://github.com/poem-web/poem/issues/995) | Send not general enough | Rust compiler limitation with diesel integration |

---

## Recommended Priority Fixes

### Quick Wins (Easy difficulty)
1. **#760** - Fix unsigned integer format (use min/max instead of uint*)
2. **#699** - Add missing PARAM_IS_REQUIRED const to Multipart
3. **#826** - Convert `:name` to `{name}` in prefix_path
4. **#796** - Honor key_name for Cookie SecurityScheme
5. **#538** - Add Content-Type header to gRPC responses

### High Impact (Medium difficulty)
1. **#1032/#923/#698** - Fix API path merging (affects many users)
2. **#669** - Fix NewType validation (critical for data integrity)
3. **#519/#318** - Fix generic type $ref encoding (spec compliance)

### Complex but Important (Hard difficulty)
1. **#661** - Fix graceful shutdown hang
2. **#653** - Fix Rustls livelock on bad PEM order
