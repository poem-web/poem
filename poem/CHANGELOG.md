# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# Unreleased

- `Cors` middleware allows all HTTP methods and headers by default.
- Add `Cors::allow_origins_fn` method.

# [1.0.5] 2010-10-19

- Add `RouteDomain` for `Host` header routing.
- Add `CookieSession` and `RedisSession` middlewares.
- Add `RequestBuilder::typed_header` and `ResponseBuilder::typed_header` methods.
- Improve Cors middleware.

# [1.0.4] 2010-10-15

- Remove the `'static` constraint of `Endpoint`.
- Add `EndpointExt::around` method.

# [1.0.3] 2010-10-14

- Change the trait bounds of `FromRequest::Error` from `Into<Error>` to `IntoResponse`.
- Implements `IntoResponse` for `Body`.
- The `CookieJar::private` and `CookieJar::signed` methods now use the key specified by `CookieJarManager::with_key`.
