# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [1.0.36] 2021-12-01

- Add helper methods `Request::deserialize_path` and `Request::deserialize_query`.
- Rename `error::ErrorInvalidPathParams` to `error::ParsePathError`.

# [1.0.34] 2021-12-01

- Implement `FromRequest` for `LocalAddr`.

# [1.0.33] 2021-11-30

- Remove `akasma` from dependencies.

# [1.0.32] 2021-11-29

- Add CSRF middleware. [#98](https://github.com/poem-web/poem/issues/98)

# [1.0.31] 2021-11-26

- Add `Request::header` and `Response::header` methods.

# [1.0.30] 2021-11-23

- `Server::new` is no longer an asynchronous method and has no return value.
- Remove `Server::local_addr` method.
- Add the `Server::name` method to specify the name of the server, it is only used for logs.

# [1.0.28] 2021-11-17

- Add `EndpointExt::with_if` method.

# [1.0.27] 2021-11-16

- Use percent-encoding before adding cookies to the header.
- Fix `CookieJar` does not support parsing from multiple `Cookie` headers.
- Fix websocket not working in `Firefox`. [#91](https://github.com/poem-web/poem/issues/91)

# [1.0.26] 2021-11-15

- Fix Cors middleware response incorrect `Access-Control-Allow-Headers` header.

# [1.0.25] 2021-11-15

- Fix the bug that `Cookie::http_only` sets incorrect attributes.

# [1.0.24] 2021-11-13

- Add `PropagateHeader` middleware.

# [1.0.23] 2021-11-10

- Add `MemoryStore` for session.
- Add `from_json::from_json` and `Body::into_json` methods.
- Add support for [`native-tls`](https://crates.io/crates/native-tls).

# [1.0.22] 2021-11-08

- Support TLS rotation.

# [1.0.21] 2021-11-06

- Add `template` and `staticfiles` features.

# [1.0.20] 2021-11-05

- Improve `EndpointExt::around`.

# [1.0.19] 2021-11-03

- Add `Request::data`, `Request::set_data`, `Response::data` and `Response::set_data` methods.
- Use Rust 2021 edition.

# [1.0.18] 2021-11-02

- Remove some useless code.

# [1.0.17] 2021-11-01

- Add `Cors::allow_headers`, `Cors::allow_methods`, `Cors::allow_origins` and `Cors::expose_headers` methods. 

# [1.0.16] 2021-10-30

- Use `Request::take_upgrade` instead of `Request::upgrade` method.

# [1.0.14] 2021-10-29

- Add `AcceptorExt::tls` method.

# [1.0.13] 2021-10-27

- Implements `From<T: Display>` for `Error`.

# [1.0.11] 2021-10-27

- Move the HTTP error helper functions to the `error` module.

# [1.0.9] 2021-10-26

- Add `LocalAddr` extractor.
- Add `Request::local_addr` method.

# [1.0.8] 2021-10-25

- Add `SizeLimit` middleware.
- Move the trace log in `serve_connection` to the new `Tracing` middleware.

# [1.0.7] 2021-10-21

- Update some docs.

# [1.0.6] 2021-10-20

- `Cors` middleware allows all HTTP methods and headers by default.
- Add `Cors::allow_origins_fn` method.

# [1.0.5] 2021-10-19

- Add `RouteDomain` for `Host` header routing.
- Add `CookieSession` and `RedisSession` middlewares.
- Add `RequestBuilder::typed_header` and `ResponseBuilder::typed_header` methods.
- Improve Cors middleware.

# [1.0.4] 2021-10-15

- Remove the `'static` constraint of `Endpoint`.
- Add `EndpointExt::around` method.

# [1.0.3] 2021-10-14

- Change the trait bounds of `FromRequest::Error` from `Into<Error>` to `IntoResponse`.
- Implements `IntoResponse` for `Body`.
- The `CookieJar::private` and `CookieJar::signed` methods now use the key specified by `CookieJarManager::with_key`.
