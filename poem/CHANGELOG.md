# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# Unreleased

- Add `RouteDomain` for `Host` header routing.

# [1.0.4] 2010-10-15

- Remove the `'static` constraint of `Endpoint`.
- Add `EndpointExt::around` method.

# [1.0.3] 2010-10-14

- Change the trait bounds of `FromRequest::Error` from `Into<Error>` to `IntoResponse`.
- Implements `IntoResponse` for `Body`.
- The `CookieJar::private` and `CookieJar::signed` methods now use the key specified by `CookieJarManager::with_key`.
