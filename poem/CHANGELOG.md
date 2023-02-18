# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [1.3.55] 2023-02-18

- fix: export real error `RedisSessionError` for `redis-session` [#501](fix: export real error `RedisSessionError` for `redis-session`)
- Add From<HashMap> implementation for I18NArgs [#507](https://github.com/poem-web/poem/pull/507)
- fix errors when parse yaml & xml request [#498](https://github.com/poem-web/poem/pull/498)
- fix `SSE::keep_alive` caused the event stream to not terminate properly.

# [1.3.53] 2023-01-31

- fix: static_files percent encode filename [#495](https://github.com/poem-web/poem/pull/495)
- bump `tokio-tungstenite` from `0.17.1`to `0.18.0` [#463](https://github.com/poem-web/poem/pull/463)
- bump base64 from `0.13.0` to `0.21.0`

# [1.3.52] 2023-01-13

- Add yaml support [#476](https://github.com/poem-web/poem/pull/476)

# [1.3.51] 2023-01-11

- More compact packing of random bytes in session_id [#437](https://github.com/poem-web/poem/pull/437)
- Fixes opentelemetry_metrics: Correct duration conversion [#449](https://github.com/poem-web/poem/pull/449)
- Support fall back to the index file when serving static files [#450](https://github.com/poem-web/poem/pull/450)
- Record and use PathPattern in response [#462](https://github.com/poem-web/poem/pull/462)
- listener::rustls: add support for elliptic curve private keys [#460](https://github.com/poem-web/poem/pull/460)
- Add `Error::set_error_message` to change the error message

# [1.3.50] 2022-12-01

- Fixes not enough randomness in session keys [#430](https://github.com/poem-web/poem/issues/430)

# [1.3.49] 2022-11-21

- Bump `quick-xml` to `0.26.0`

# [1.3.48] 2022-10-25

- Fixes [#416](https://github.com/poem-web/poem/issues/416)
- Re-enable the `brotli` compression algorithm.
- Add `Compress::with_quality` and `Compression::with_quality` methods.
- Add `Compression::algorithms` to specify the enabled algorithms.
- Make `WebSocketUpgraded<T>` and `BoxWebSocketUpgraded` public [#415](https://github.com/poem-web/poem/issues/415)

# [1.3.47] 2022-10-19

- Fixes [#346](https://github.com/poem-web/poem/issues/346) [#395](https://github.com/poem-web/poem/issues/395)
- Bump redis version to 0.22.0 [#412](https://github.com/poem-web/poem/pull/412)
- Bump opentelemetry from `0.17.0` to `0.18.0`

# [1.3.46] 2022-10-17

- Add `path_pattern` to log [#337](https://github.com/poem-web/poem/issues/337)
- Add support to generics handler [#408](https://github.com/poem-web/poem/issues/408)

# [1.3.45] 2022-09-28

- Add `Error::is_from_response` method.

# [1.3.44] 2022-09-25

- Add `Error::status` method to get the status code of error.

# [1.3.43] 2022-09-23

- Removed dependency on `typed-headers`. [#394](https://github.com/poem-web/poem/issues/394)

# [1.3.42] 2022-09-11

- Fixed `StaticFileEndpoint` returning an incorrect `Content-Length` header when a `Range` header is in the request.
- Fixed `Compression` middleware returning incorrect `Content-Length` header.
- Disabled `brotli(CompressionAlgo::BR)` algorithm, very slow, still looking for the reason.

# [1.3.41] 2022-08-16

- Use the real IP as the `remote_addr` in the logs of the Tracing middleware. [#370](https://github.com/poem-web/poem/issues/370)
- Automatically decode percent-encoded path parameters. [#375](https://github.com/poem-web/poem/issues/375)
- Fix: trace nested route with original uri. [#371](https://github.com/poem-web/poem/pull/371)
- Automatically decode percent-encoded path parameters. [#375](https://github.com/poem-web/poem/issues/375)
- Add `ForceHttps::filter` method to determine if a request should be redirect. [#360](https://github.com/poem-web/poem/issues/360)
- Add `content-length` for `StaticFileResponse` [#373](https://github.com/poem-web/poem/pull/373)

# [1.3.39] 2022-08-16

- CORS middleware use 403 instead of 401. [#368](https://github.com/poem-web/poem/issues/368)

# [1.3.38] 2022-08-12

- Expose libcookie `iter()` [#361](https://github.com/poem-web/poem/pull/361)

# [1.3.37] 2022-08-02

- Add `StaticFilesEndpoint::redirect_to_slash_directory`  to enable Redirects to a slash-ended path when browsing a directory.
- Add `has_source` method to `poem::Error` [#349](https://github.com/poem-web/poem/pull/349)
- Use `LENGTH_REQUIRED` instead of `BAD_REQUEST` in `SizeLimit` middleware [#348](https://github.com/poem-web/poem/pull/348)

# [1.3.35] 2022-07-16

- Expose macro `impl_apirequest_for_payload` for custom payload type and add an example to demonstrating the custom payload. [#309](https://github.com/poem-web/poem/pull/309)
- Add `Accept` extractor.
- Add `TcpAcceptor::from_tokio` method. [#317](https://github.com/poem-web/poem/issues/317)

# [1.3.33] 2022-07-10

- Chose Compress algorithm with priority [#302](https://github.com/poem-web/poem/pull/302)
- Increase the MSRV to `1.61`

# [1.3.32] 2022-06-24

- Add XML support [#297](https://github.com/poem-web/poem/pull/297)
- Add `RealIp` extractor

# [1.3.31] 2022-06-17

- Add openssl TLS listener. [#289](https://github.com/poem-web/poem/pull/289)

# [1.3.30] 2022-5-15

- Fix route variable matching incorrectly. [#275](https://github.com/poem-web/poem/issues/275)

# [1.3.28] 2022-4-16

- Add support for multiple domains to `RustlsConfig`.
- Bump `rustls-pemfile` from `0.3.0` to `1.0.0`.

# [1.3.24] 2022-4-13

- ~~Do not include a body for HTTP 304~~ [#257](https://github.com/poem-web/poem/pull/257)
- An error created by a status code should be converted to a response without a body.

# [1.3.23] 2022-4-11

- Integrate with `rust-embed`. [#251](https://github.com/poem-web/poem/pull/251)

# [1.3.22] 2022-4-8

- Make the `AutoCert` middleware to register an account only when it needs to create a new certificate.

# [1.3.21] 2022-4-7

- Add `CatchPanic` middleware.

# [1.3.19] 2022-3-30

- Add the ability for parameters to come from urlencoded forms. [#245](https://github.com/poem-web/poem/issues/245)

# [1.3.17] 2022-3-23

- Add `server` feature. (enable by default)
- Change `Error::as_response` to `Error::into_response`.

# [1.3.16] 2022-3-18

- Add `Cache-Control: no-cache` header to SSE response.
- Add `TokioMetrics` middleware. [#206](https://github.com/poem-web/poem/issues/206)

# [1.3.15] 2022-3-16

- Check the `Content-type` when use `Json` extractor. [#236](https://github.com/poem-web/poem/pull/236)
- Add `EndpointExt::data_opt` method.

# [1.3.13] 2022-3-10

- Make all test cases use `poem::test`.

# [1.3.8] 2022-3-4

- Fix Poem with Tonic doesn't return GRPC status header when rpc handler returns an error. [#212](https://github.com/poem-web/poem/issues/212)

# [1.3.7] 2022-3-1

- Fix Poem with Tonic doesn't return GRPC status header. [#212](https://github.com/poem-web/poem/issues/212)

# [1.3.6] 2022-2-22

- Change charset from `utf8` to `utf-8`. [#213](https://github.com/poem-web/poem/pull/213)

# [1.3.5] 2022-2-22

- Add `X-Accel-Buffering: no` header to SSE response.
- Add `AutoCertBuilder::contact` method to add a contact email for ACME account.
- Add `TestJsonArray::assert_is_empty` and `TestJsonObject::assert_is_empty` methods.
- Add `TestJsonArray::assert_contains` and `TestJsonObject::assert_contains_exactly_one` methods.

# [1.3.4] 2022-2-21

- Fixed `AutoCert` loading cached certificates with incorrect paths.

# [1.3.3] 2022-2-21

- Implement `Listener` for `BoxListener`.

# [1.3.2] 2022-2-21

- Add `ListenerExt::boxed` method.

# [1.3.1] 2022-2-21

- Add `Body::is_empty` method.
- Add `RouteScheme` for scheme routing.
- Add support `HTTP-01` challenge for ACME.

# [1.3.0] 2022-2-20

- Add support for ACME(Automatic Certificate Management Environment).

# [1.2.59] 2022-2-17

- Add `Response::set_content_type` method.
- Add `IntoResponse::with_content_type` method.

# [1.2.54] 2022-2-8

- Fix session renew gets overwritten by session change. [#196](https://github.com/poem-web/poem/issues/196)

# [1.2.52] 2022-2-2

- Integrate with `eyre`. [#190](https://github.com/poem-web/poem/pull/190)
- Bump `tokio-rustls` from `0.22.0` to `0.23.2`.
- Add default response type to `BoxEndpoint`.

# [1.2.51] 2022-1-31

- Replace `SystemTime` with `Instant` in tracing middleware. [#187](https://github.com/poem-web/poem/pull/187)

# [1.2.49] 2022-1-29

- Make the `StaticFileRequest::create_response` method correctly return `Err(StaticFileError::NotFound)` when the specified file does not exist.

# [1.2.48] 2022-1-27

- The `Content-Type` header of the `Html` response was changed from `text/html` to `text/html; charset=utf8`.

# [1.2.47] 2022-1-26

- Add `TestJsonValue::assert_not_null` method.

# [1.2.46] 2022-1-26

- Add `TestRequestBuilder::typed_header` method.

# [1.2.45] 2022-1-25

- Changed the return error type of multipart related methods from `IoError` to `ParseMultipartError`.
- Add `TestRequestBuilder::form` method.
- Add more examples to `poem::test`.

# [1.2.44] 2022-1-22

- `Redirect` parameter type changed to `impl Display`. [#176](https://github.com/poem-web/poem/issues/176)

# [1.2.42] 2022-1-22

- Fix crash caused by invalid request URI. [#174](https://github.com/poem-web/poem/issues/174)

# [1.2.41] 2022-1-21

- Add `Body::into_bytes_limit` method.

# [1.2.39] 2022-1-20

- Add `TestJsonArray::get_opt` and `TestJsonObject::get_opt` methods.

# [1.2.37] 2022-1-18

- Add `EndpointExt::to_response` method.

# [1.2.34] 2022-1-14

- Add `TcpAcceptor::from_std` and `UnixAcceptor::from_std` methods.
- Add support for multipart tests.

# [1.2.31] 2022-1-12

- Add `I18NArgs::set` method.

# [1.2.30] 2022-1-11

- Change the behavior of the `TestClient::query` method to add a KV pair.

# [1.2.29] 2022-1-11

- Add `SensitiveHeader` middleware.

# [1.2.28] 2022-1-8

- Add support handling `Range` header for the `StaticFile` extractor.
- Add `ResponseError::as_response` method.

# [1.2.27] 2022-1-7

- Rename `poem::endpoint::StaticFiles` to `poem::endpoint::StaticFilesEndpoint`.
- Rename `poem::endpoint::StaticFile` to `poem::endpoint::StaticFileEndpoint`.
- Add `poem::web::StaticFileRequest` extractor.

# [1.2.26] 2022-1-6

- Add I18N support with [`fluent`](https://crates.io/crates/fluent).

# [1.2.22] 2022-1-4

- Add test utilities.

# [1.2.20] 2022-1-1

- `RouteMethod` returns `MethodNotAllowedError` error instead of `NotFoundError` when the corresponding method is not found.

# [1.2.19] 2021-12-31

- Fixed the `Cors` middleware to return incorrect headers when an error occurs.

# [1.2.18] 2021-12-31

- Bump `cookie` crate from `0.15.1` to `0.16`. 

# [1.2.17] 2021-12-29

- Add `FromRequest:: from_request_without_body` method.

# [1.2.16] 2021-12-29

- Fix panic when accessing HTTPS endpoint with HTTP. [#141](https://github.com/poem-web/poem/issues/141)
- Add `ForceHttps::https_port` method.

# [1.2.15] 2021-12-28

- Improve TLS listeners.

# [1.2.14] 2021-12-28

- Rename `poem::endpoint::Files` to `poem::endpoint::StaticFiles`.
- Add `poem::endpoint::StaticFile` to handing single static file.

# [1.2.13] 2021-12-28

- Add `Request::scheme` method.
- Add `ForceHttps` middleware.

# [1.2.12] 2021-12-27

- Add `Files` endpoint support for  `If-None-Match`, `If-Modified-Since`, `If-Match`, `If-Unmodified-Since` headers.

# [1.2.11] 2021-12-27

- Add `Response::is_ok` method to check the status code of response is `200 OK`.

# [1.2.10] 2021-12-26

- Add `Request::uri_str` method.

# [1.2.9] 2021-12-22

- Add `Route::try_at`, `Route::try_nest`, `Route::try_nest_no_strip` methods.
- Add `RouteDomain::try_at` method.
- Rename `RouteDomain::add` to `RouteDomain::at`.

# [1.2.8] 2021-12-21

- Fix session data is serialized twice. [#109](https://github.com/poem-web/poem/issues/109)

# [1.2.6] 2021-12-19

- Panic when there are duplicates in the routing table. [#126](https://github.com/poem-web/poem/issues/126)
- Add error messages to the tracing middleware.

# [1.2.4] 2021-12-17

- Rename `EndpointExt::inspect_error` to `EndpointExt::inspect_all_error`.
- Rename `EndpointExt::inspect_typed_error` to `EndpointExt::inspect_error`.
- Add `EndpointExt::catch_all_error` method.

# [1.2.3] 2021-12-17

- Add `Endpoint::get_response` method.

# [1.2.2] 2021-12-16

- Add `EndpointExt::inspect_typed_err` method.
- Rename `Error::new_with_string` to `Error::from_string`.
- Rename `Error::new_with_status` to `Error::from_status`.
- Integrate with the [`anyhow`](https://crates.io/crates/anyhow) crate.

# [1.2.0] 2021-12-16

## Breaking changes

- Refactor error handling.
- The return value type of the `Endpoint::call` function is changed from `Self::Output` to `Result<Self::Output>`.
- Remove the associated type `Error` from `FromRequest`. 
- The return value of the `FromRequest::from_request` function is changed from `Result<Self, Self::Error>` to `Result<Self>`.
- Add some helper methods to `EndpointExt`.

# [1.1.1] 2021-12-13

- Add `Body::from_bytes_stream` and `Body::to_bytes_stream` methods.
- Remove the `BinaryStream` type, use `poem::Body` instead.

# [1.1.0] 2021-12-13

- Remove `nom` from dependencies.

# [1.0.38] 2021-12-07

- Rename `Request::deserialize_path` to `Request::path_params`, `Request::deserialize_query` to `Request::params`.
- Rename `Request::path_param` to `Request::raw_path_param`.

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
