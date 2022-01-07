# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [1.2.26] 2022-1-7

- Add `StaticFile` response.

# [1.2.25] 2022-1-4

- Add `deprecated` attribute to `Enum` macro.

# [1.2.24] 2022-1-4

- Add `OpenApiService::summary` method.

# [1.2.21] 2022-1-1

- The `OneOf` macro no longer automatically implements `serde::Serialize` and `serde::Deserialize` traits.

# [1.2.20] 2021-12-31

- The `Object` macro no longer automatically implements `serde::Serialize` and `serde::Deserialize` traits.

# [1.2.18] 2021-12-31

- Fix generates a field with #[oai(default)] marked as required even though it isn't. [#145](https://github.com/poem-web/poem/issues/145)

# [1.2.17] 2021-12-29

- Add `EventStream::keep_alive` method.

# [1.2.11] 2021-12-27

- Remove the `OpenApi::combine` method, `OpenApiSerice::new` can be passed a tuple to combine multiple API objects.

# [1.2.10] 2021-12-26

- The `content_type` attribute of the `ApiRequest` macro supports wildcards.
- Add `EventStream` payload.
- Implement `Type` for `serde_json::Value`.

```rust
#[derive(ApiRequest)]
enum UploadImageRequest {
    #[oai(content_type = "image/jpeg")]
    Jpeg(Binary<Vec<u8>>),
    #[oai(content_type = "image/png")]
    Png(Binary<Vec<u8>>),
    #[oai(content_type = "image/*")]
    Other(Binary<Vec<u8>>),
}
```

# [1.2.8] 2021-12-21

- Added the `content_type` attribute to the `ApiRequest` and `ApiResponse` macros to specify the content type of the request or response.
- Panic occurs when a duplicate operation id is detected.
- Add `OpenApiService::external_document` method to referencing an external resource for extended documentation.
- Add `Webhook` macro to define webhooks.
- Implement `OpenApi` for `()` to define an empty APIs.

# [1.2.7] 2021-12-19

- Make the `OpenAPI` macro can now report duplicate routing errors.

# [1.2.4] 2021-12-18

- Fix the parameter validator will cause compilation failure in some cases.

# [1.2.4] 2021-12-17

- Add `remote` attribute to `Enum` macro.
- Remove the `BinaryStream` type, use `poem::Body` instead.
- Do not rename any types by default. [#128](https://github.com/poem-web/poem/issues/128)

# [1.1.1] 2021-12-12

- Add `BinaryStream::from_bytes_stream` and `BinaryStream::to_bytes_stream` methods.

# [1.0.51] 2021-12-12

- Add some methods to specify more API metadata.
- Add `Response` type, use it to modify the status code and HTTP headers.

# [1.0.50] 2021-12-11

- impl `ParseFromParameter` for [T; const LEN: usize].
- Add `example` attribute for `Object` macro.
- Add `deny_unknown_fields` attribute for `Object` and `Multipart` macros.

# [1.0.49] 2021-12-10

- Add `Email`/`Hostname` types.
- Integrate with the `regex`, `uuid`.
- Implement `Type` for `Uri`.
- Implement `Type` for `DateTime<Utc>` and `DateTime<Local>`.
- Add support for [Redoc](https://github.com/Redocly/redoc).

# [1.0.48] 2021-12-10

- Remove the `PoemExtractor` type because it is no longer needed.

# [1.0.47] 2021-12-10

- Add `Attachment` payload for download file.
- Added `BinaryStream` to support streaming payload.

# [1.0.46] 2021-12-10

- Change the default renaming rule of enum items from `ScreamingSnake` to `Pascal`.

# [1.0.45] 2021-12-09

- Remove the `desc` attribute of the response header in `ApiResponse` macro, and use rustdoc to add the header description.
- Implement `ParseFromParameter` for `Vec<T>`.

# [1.0.44] 2021-12-08

- Remove the `list` attribute of the validator, it is no longer needed.
- Add `maxProperties` and `minProperties` validators.
- Add support to API operation with optional payload.
- Add support to API responses with optional header.

# [1.0.43] 2021-12-07

- Change the schema type of enum to `string` [#121](https://github.com/poem-web/poem/issues/121)

# [1.0.42] 2021-12-07

- Implement `Type` for `&[T]`, `&T` and `[T; const N: usize]`.
- Add support for returning references from API operation functions.

# [1.0.41] 2021-12-07

- Fixed the bug that `Arc`, `Box`, `BTreeMap` and `HashMap` did not register subtypes.

# [1.0.40] 2021-12-07

- Add support for `additionalProperties`.

# [1.0.39] 2021-12-07

- Rework implement `Type` for `HashMap` and `BTreeMap`.

# [1.0.38] 2021-12-07

- Implement `Type` for `Box<T>`, `Arc<T>`, `HashMap<K, V>` and `BTreeMap<K, V>`. [#116](https://github.com/poem-web/poem/issues/116)

# [1.0.37] 2021-12-06

- Add support for [RapiDoc](https://github.com/mrin9/RapiDoc).
- Remove the `desc` attribute of the operation parameter in `OpenAPI` macro, and use rustdoc to add the parameter description.

# [1.0.35] 2021-12-05

- If a OpenAPI name conflict is detected when creating schema, it will cause panic.

# [1.0.33] 2021-11-30

- Remove `akasma` from dependencies.

- # [1.0.31] 2021-11-30

- `#[oai(validator(list))]` no longer applies to `max_items`, `min_items` and `unique_items`.

# [1.0.29] 2021-11-22

- Add `list` attribute to the validator.
- Rework `OpenAPI` macro.

# [1.0.28] 2021-11-17

- Omit empty security schemas from OpenAPI document. [#93](https://github.com/poem-web/poem/pull/93)

# [1.0.27] 2021-11-16

- Description is a required field for responses. [#86](https://github.com/poem-web/poem/issues/86)

# [1.0.26] 2021-11-15

- Add `version` and `title` parameters to `OpenAPIService::new`. [#87](https://github.com/poem-web/poem/issues/87)

# [1.0.19] 2021-11-03

- Add `checker` attribute for `SecurityScheme` macro.
- Use Rust 2021 edition.

# [1.0.18] 2021-11-02

- Some configurations no longer need `'static`.

# [1.0.12] 2021-10-27

- Correctly determine the type of payload.

# [1.0.11] 2021-10-27

- Bump `poem` to `1.0.11`.

# [1.0.10] 2021-10-26

- Make the return type of operation function more flexible.

# [1.0.9] 2021-10-26

- Add `Any` type.

# [1.0.8] 2021-10-25

- Add `read_only_all` and `write_only_all` to `ObjectArgs`. [#71](https://github.com/poem-web/poem/pull/71)

# [1.0.7] 2021-10-21

- Fix Json parsing not working for unsigned integers. [#68](https://github.com/poem-web/poem/pull/68)

# [1.0.4] 2021-10-15

- Bump `poem` from `1.0.3` to `1.0.4`.

# [1.0.3] 2021-10-14

- Add `prefix_path` and `tag` attributes for `#[OpenApi]`. [#57](https://github.com/poem-web/poem/pull/57)
- `OpenApiService::swagger_ui` method no longer needs the `absolute_uri` parameter.
- Add `inline` attribute for `Object` macro.
- Add generic support for `ApiRequest` and `ApiResponse` macros.

## [1.0.2] 2021-10-11

- Add `write_only` and `read_only` attributes for object fields.
- Add `OpenApiService::spec` method to get the generated OAS specification file.
- Implements `Default` trait for `poem_openapi::types::multipart::JsonField<T>`.
- Implements `ParseFromMultipartField` for some types.

## [1.0.1] 2021-10-10

- Add `Request::remote_addr` method.
