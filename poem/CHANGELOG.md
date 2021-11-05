# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [1.0.21]

- Add `template` feature.

# [1.0.20]

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
