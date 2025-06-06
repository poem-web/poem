# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [5.1.15] 2025-06-06

- Bump `derive_more` to `2.0`
- Fix webhook nesting [#1031](https://github.com/poem-web/poem/pull/1031)
- add support for num::NonZero [#1041](https://github.com/poem-web/poem/pull/1041)
- Support deep linking in Swagger UI [#1049](https://github.com/poem-web/poem/pull/1049)
- Add support to externally tagged unions [#1043](https://github.com/poem-web/poem/pull/1043)

# [5.1.14] 2025-05-03

- add scalar ui support [#1019](https://github.com/poem-web/poem/pull/1019)
- support `Duration` and `Timestamp` from `prost_wkt_types` [#1016](https://github.com/poem-web/poem/pull/1016)
- Object fields deprecation [#1026](https://github.com/poem-web/poem/pull/1026)
- add support for server variables [#962](https://github.com/poem-web/poem/pull/962)

# [5.1.12] 2025-03-30

- the `Binary` type no longer requires `content-type` to be `application/octet-stream`.

# [5.1.11] 2025-03-29

- feat: `ignore_case` parameter in `OpenApi` macro can be used to operation.

# [5.1.10] 2025-03-29

- feat: add `ignore_case` parameter for `OpenApi` macro.

# [5.1.9] 2025-03-24

- fix(openapi): do not use `cookie` feature by default [#986](https://github.com/poem-web/poem/pull/986)
- fix: guard cookie features behind feature toggle [#997](https://github.com/poem-web/poem/pull/997)
- Support rename in NewType [#964](https://github.com/poem-web/poem/pull/964)
- fix(openapi): exclude style parameter when serializing if none [#989](https://github.com/poem-web/poem/pull/989)
- fix(poem-openapi): handle additional_properties correctly in flatten [#961](https://github.com/poem-web/poem/pull/961)
- fix(poem-openapi-derive): Allow different path param names on same route [#952](https://github.com/poem-web/poem/pull/952)
- Update MSRV to `1.85.0`

# [5.1.6] 2025-02-21

- Allows passing the style of a parameter in the openapi spec. [#940](https://github.com/poem-web/poem/pull/940)
- Add support for Stoplight Elements [#954](https://github.com/poem-web/poem/pull/954)
- Correct server object reference URL anchor [#957](https://github.com/poem-web/poem/pull/957)
- feat(openapi): reflect fallback security scheme in spec [#958](https://github.com/poem-web/poem/pull/958)
- Fix missing condition for Stoplight Elements UI [#972](https://github.com/poem-web/poem/pull/972)
- Update MSRV to `1.83.0`

# [5.1.5] 2025-01-04

- Add description to Union descriminator object schema [#921](https://github.com/poem-web/poem/pull/921)
- make Json from poem-openapi derive Default because Json from poem does [#938](https://github.com/poem-web/poem/pull/938)
- Pass `ParsePayload<T>::IS_REQUIRED` to `T` instead of defaulting to `true` [#932](https://github.com/poem-web/poem/pull/932)
- allow path in status for ApiResponse [#937](https://github.com/poem-web/poem/pull/937)

# [5.1.4] 2024-11-25

- Assign the description to the request object in OpenAPI [#886](https://github.com/poem-web/poem/pull/886)
- Implemented nullable fields for openapi spec generation [#865](https://github.com/poem-web/poem/pull/865)
- refactor: change type name delimiters from `<>` `()` `[]` to `_` [#904](https://github.com/poem-web/poem/pull/904)

# [5.1.3] 2024-11-20

- Update MSRV to `1.81.0`

# [5.1.2] 2024-10-02

- implements `Serialize` and `Deserialize` for `poem_openapi::types::Any<T>`.
- add `ParseError::message method` to get the error message.

# [5.1.1] 2024-09-13

- fix [#883](https://github.com/poem-web/poem/issues/883)

# [5.1.0] 2024-09-08

- fix read_only_with_default test when only default features are enabled [#854](https://github.com/poem-web/poem/pulls)
- feat: add AsyncSeek trait to Upload::into_async_read return type [#853](https://github.com/poem-web/poem/pull/853)
- Added derivations for Type, ParseFromJSON and ToJSON for sqlx::types::Json<T>. [#833](https://github.com/poem-web/poem/pull/833)
- chore(openapi): bump derive_more [#867](https://github.com/poem-web/poem/pull/867)

# [5.0.3] 2024-07-27

- Added derivations for Type, ParseFromJSON and ToJSON for sqlx types [#833](https://github.com/poem-web/poem/pull/833)

# [5.0.1] 2024-05-18

- Add enum_items to discriminated union [#741](https://github.com/poem-web/poem/pull/741)
- fix Union doesn't implement IsObjectType [#800](https://github.com/poem-web/poem/issues/800)
- fix Union doesn't support generics in the last version [#799](https://github.com/poem-web/poem/issues/799)
- Expose Poem-OpenApi Upload File struct [#816](https://github.com/poem-web/poem/pull/816)

# [5.0.0] 2024-03-30

- use AFIT instead of `async_trait`
- add `Upload::size` method
- when `Union` uses discriminator, if the members is not an `Object`, an error will be reported at compile time

# [4.0.1] 2024-03-04

- added example value support for param/schema [#717](https://github.com/poem-web/poem/pull/717)
- Adding serialize_with and deserialize_with attributes to struct fields [#749](https://github.com/poem-web/poem/pull/749)

# [4.0.0] 2024-01-06

- upgrade to `hyper1`
- added documentation on how to merge API specs [#716](https://github.com/poem-web/poem/pull/716)
- impl Type for std::time::Duration instead of only humantime::Duration [#713](https://github.com/poem-web/poem/pull/713)

# [3.0.6] 2023-11-19

- add [`prost-wkt-types` crate](https://crates.io/crates/prost-wkt-types) support [#689](https://github.com/poem-web/poem/pull/689)
- add [`geo-types` crate](https://crates.io/crates/geo-types) support [#693](https://github.com/poem-web/poem/pull/693)
- count string length correctly in OpenAPI validators [#666](https://github.com/poem-web/poem/pull/666)
- Support for custom hash functions for HashMap/HashSet [#654](https://github.com/poem-web/poem/pull/654)
- Misplaced `</html>`` in swagger_ui HTML template [#660](https://github.com/poem-web/poem/issues/660)
- for `read-only` properties, can use `default` to specify a function for creating a default value. [#647](https://github.com/poem-web/poem/issues/647)

```rust
fn default_offset_datetime() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

#[derive(Debug, Object, PartialEq)]
struct Obj {
    #[oai(read_only, default = "default_offset_datetime")]
    time: OffsetDateTime,
}
```

# [3.0.5] 2023-09-06

- fixes [#648](https://github.com/poem-web/poem/issues/648)

# [3.0.4] 2023-09-02

- allow using expressions as `prefix_path` parameter [#635](https://github.com/poem-web/poem/issues/635)
- bump `quick-xml` from `0.29.0` to `0.30.0`

# [3.0.3] 2023-08-18

- Add fallback support for `SecurityScheme` macro used on enums

# [3.0.2] 2023-08-12

- Add support for multiple authentication methods. [#627](https://github.com/poem-web/poem/discussions/627)

## Breaking changes

- change `fn ApiExtractor::security_schemes() -> Option<&str>` to `fn ApiExtractor::security_schemes() -> Vec<&str>`

# [3.0.1] 2023-08-02

- openapi: allows multiple security schemes on one operation [#621](https://github.com/poem-web/poem/issues/621)

# [3.0.0] 2023-06-21

- bump `syn` from `1.0` to `2.0`
- bump `darling` from `0.14` to `0.20`
- feat: introduce idle timeout [#603](https://github.com/poem-web/poem/pull/603)

## Breaking Changes

- Since `syn 2.0` no longer supports keywords as meta path, renamed some parameters in macros.

    | Macro              | Old Name | New Name |
    |--------------------|----------|----------|
    | SecuritySchema     | type     | ty       |
    | SecuritySchema     | in       | key_in   |
    | ApiResponse.header | type     | ty       |

    https://github.com/dtolnay/syn/issues/1458
    https://github.com/TedDriggs/darling/issues/238

- Change `ApiExtractor::TYPE` to `ApiExtractor::TYPES` to allow implementing multiple extractor in single type.

# [2.0.27] 2023-06-06

- feat: Implement Type on the char primitive [#518](https://github.com/poem-web/poem/pull/518)
- Pattern matching in OpenAPI function args  [#517](https://github.com/poem-web/poem/pull/517)
- chore: add Clone for `OpenApiService` [#527](https://github.com/poem-web/poem/pull/527)
- Fix `#[derive(Multipart)]` for struct, so it will work with `#[derive(ApiRequest)]` [#551](https://github.com/poem-web/poem/pull/551)
- feat: Allow more types to be prased into strings [#545](https://github.com/poem-web/poem/pull/545)
- Support for ipnet crate + IpAddr [#544](https://github.com/poem-web/poem/pull/544)

# [2.0.24] 2023-01-31

- Allow optional prefix in generated spec [#473](https://github.com/poem-web/poem/pull/473)
- Fixes [#489](https://github.com/poem-web/poem/issues/489)

# [2.0.23] 2023-01-13

- Add the missing feature `openapi-explorer` in `ui` mod [#480](https://github.com/poem-web/poem/pull/480)
- Add yaml support [#476](https://github.com/poem-web/poem/pull/476)
- Remove `poem_openapi::response::StaticFileResponse` and implement `ApiResponse trait` for `poem::web::StaticFileResponse`

# [2.0.22] 2023-01-11

- Add support for OpenAPI Explorer [#440](https://github.com/poem-web/poem/pull/440)
- Add `Ipv4Addr` and `Ipv6Addr` openapi support [#442](https://github.com/poem-web/poem/pull/442)
- Parse Value(Number) into Decimal [#452](https://github.com/poem-web/poem/pull/452)
- Parse other number types as well and fix float [#454](https://github.com/poem-web/poem/pull/454)
- Responses generated by the `ApiResponse` macro have correct error messages when converted to `poem::Error`

# [2.0.21] 2022-12-01

- Add generic support to the `NewType` macro
- Fixes [#436](https://github.com/poem-web/poem/issues/436)

# [2.0.20] 2022-11-21

- Bump quick-xml to `0.26.0`
- Fixes [#429](https://github.com/poem-web/poem/issues/429)

# [2.0.19] 2022-10-25

- Add `example` attribute to the `NewType` macro [#404](https://github.com/poem-web/poem/issues/404)
- Implement `ApiResponse` for `WebSocketUpgraded<T>` [#415](https://github.com/poem-web/poem/issues/415)

# [2.0.18] 2022-10-19

- Fixes [#346](https://github.com/poem-web/poem/issues/346) [#395](https://github.com/poem-web/poem/issues/395)

# [2.0.17] 2022-10-17

- Throws an error when `flatten` combination with structs that use `deny_unknown_fields `

# [2.0.16] 2022-10-07

- Fixes [#405](https://github.com/poem-web/poem/issues/405)

# [2.0.11] 2022-08-30

- Add `EventStream::to_event` method to set a function used to convert the message to SSE event. [#378](https://github.com/poem-web/poem/issues/378)
- OpenApi XML support [#354](https://github.com/poem-web/poem/pull/354)
- Add `hidden` attribute for the operation [#376](https://github.com/poem-web/poem/issues/376)

# [2.0.10] 2022-08-16

- Add the `default` attribute to Object macro. [#369](https://github.com/poem-web/poem/issues/369)

# [2.0.9] 2022-08-16

- Add `actual_type` to schema registry. [#366](https://github.com/poem-web/poem/pull/366)
- Add `explode` attribute for the operation parameter. [#367](https://github.com/poem-web/poem/issues/367)

# [2.0.8] 2022-08-12

- Fixes [#362](https://github.com/poem-web/poem/issues/362)
- Add `OperationId` extension for response of OpenAPI [#351](https://github.com/poem-web/poem/issues/351)

# [2.0.7] 2022-08-02

- Expose `AttachmentType` enum [#344](https://github.com/poem-web/poem/issues/344)
- Add `rename_all` attribute for `Union` macro [#347](https://github.com/poem-web/poem/issues/347)
- Change the default attachment type to `attachment` [#325](https://github.com/poem-web/poem/issues/325)
- Update `serde_yaml` to 0.9.0 [#352](https://github.com/poem-web/poem/pull/352)

# [2.0.6] 2022-07-26

- Use first line of comment as title of newtype param [#319](https://github.com/poem-web/poem/issues/319)
- Add `Content-Disposition` header to schema for `Attachment` [#325](https://github.com/poem-web/poem/issues/325)
- Add support for `x-code-samples` [#335](https://github.com/poem-web/poem/issues/335)
- Return `400` when parsing path fails, not `404` [#326](https://github.com/poem-web/poem/pull/326)
- Use Parent_Child instead of Parent[Child] for generated intermediate type [#340](https://github.com/poem-web/poem/pull/340)
- Fixed docs for `NewType` macro

# [2.0.5] 2022-07-16

- Add support for specifying contact field [#306](https://github.com/poem-web/poem/issues/306)
- Add `actual_type` attribute to `OpenApi`, `Response`, `ResponseContent` macros [#314](https://github.com/poem-web/poem/issues/314)

# [2.0.4] 2022-07-12

- Bump `uuid` crate from `0.8.2` to `1.1.0` [#304](https://github.com/poem-web/poem/issues/304)
- Fixed `Union` macro generating incorrect schema. [#263](https://github.com/poem-web/poem/issues/263)

# [2.0.3] 2022-07-10

- Add integrate with the [`time` crate](https://crates.io/crates/time).
- Add support generating openapi UI html through the dedicated function. [#298](https://github.com/poem-web/poem/issues/298)

# [2.0.1] 2022-06-17

- Add support for getting the spec as YAML [#287](https://github.com/poem-web/poem/issues/287)
- Add optional support for humantime Duration in poem Object [#293](https://github.com/poem-web/poem/issues/293)

# [2.0.0] 2022-05-30

- Publish `Poem-openapi v2.0.0` 🙂

# [2.0.0-alpha.2] 2022-05-20

- Re-added the `example` attribute for `Object` macro.
- Response `404 NOT FOUND` when parsing path parameters fails. [#279](https://github.com/poem-web/poem/discussions/279)

# [2.0.0-alpha.1] 2022-05-15

- Remove `inline` and `concrete` attributes of `Object` and `Union` macros, now automatically generate reference names for generic objects.

# [1.3.28] 2022-04-16

- If the `inline` or `concretes` attribute of the generic object is not specified, the exact error will be reported at compile time.

# [1.3.28] 2022-04-15

- Add support for generic union. [#259](https://github.com/poem-web/poem/issues/259) 

# [1.3.26] 2022-04-14

- Fixed `poem::web::StaticFileResponse` conversion to `poem_openapi::respoinse::StaticFileResponse` missing `Content-Type` header.

# [1.3.25] 2022-04-13

- Downgrades the `indexmap` dependency to `1.6.2` to resolve https://github.com/tkaitchuck/aHash/issues/95

# [1.3.23] 2022-4-11

- Implement `Type` for `chrono::NaiveDateTime`, `chrono::NaiveDate`, `chrono::NaiveTime` [#252](https://github.com/poem-web/poem/issues/252)

# [1.3.20] 2022-4-1

- Fixed `#[oai(default)]` not working with operation parameters.
- Add `MaybeUndefined::update_to` method.

# [1.3.18] 2022-3-24

- Generate responses in schema when `ApiResponse` as an error. [#244](https://github.com/poem-web/poem/issues/244)

# [1.3.17] 2022-3-23

- Implement `From<T: ApiResponse>` for `poem::Error` so that it can be used as error type.

# [1.3.14] 2022-3-10

- Add support for use multiple methods on a single endpoint. [#229](https://github.com/poem-web/poem/discussions/229)

# [1.3.13] 2022-3-10

- Normalize decimals in responses. [#228](https://github.com/poem-web/poem/pull/228)

# [1.3.12] 2022-3-9

- Add support for extra request/response headers.

# [1.3.11] 2022-3-9

- Add support for extra headers.

# [1.3.10] 2022-3-7

- Add support generic for `OpenAPI` macro. [#216](https://github.com/poem-web/poem/issues/216)

# [1.3.9] 2022-3-7

- Add `skip_serializing_if_is_none`, `skip_serializing_if_is_empty` and `skip_serializing_if` attributes to `Object` macro. [#220](https://github.com/poem-web/poem/issues/220)

# [1.3.7] 2022-3-1

- Implement `Type` for `rust_decimal::Decimal`. [#214](https://github.com/poem-web/poem/issues/214)

# [1.2.58] 2022-2-15

- Add support for deriving remote objects to the `Object` macro.
- Add `poem_openapi::payload::Base64`.

# [1.2.57] 2022-2-10

- Implement `From<T>`, `IntoIterator` for `MaybeUndefined<T>`.
- Add `MaybeUndefined::from_opt_undefined`, `MaybeUndefined::from_opt_null`, `MaybeUndefined::as_ref`, `MaybeUndefined::as_deref` methods.

# [1.2.56] 2022-2-10

- Implement `ToJson` for `MaybeUndefined`.

# [1.2.55] 2022-2-10

- Add `MaybeUndefined` type.

# [1.2.53] 2022-2-3

- Fix OpenAPI doesn't work with tracing::instrument. [#194](https://github.com/poem-web/poem/issues/194)

# [1.2.51] 2022-1-31

- Fix unsupported media-type (415) instead of method not allowed (405). [#188](https://github.com/poem-web/poem/pull/188)
- Integrate with `bson::oid::ObjectId`. [#185](https://github.com/poem-web/poem/pull/185/) 

# [1.2.50] 2022-1-29

- Make the `ApiRequest` macro exactly match the mime type.

# [1.2.48] 2022-1-29

- Add `Html` payload type.
- Add `StaticFileResponse` type.

# [1.2.46] 2022-1-26 

- Fixed stack overflow on recursive structures. [#184](https://github.com/poem-web/poem/issues/184)

# [1.2.43] 2022-1-22

- Fixed `EventStream` not registering internal types.
- OpenApi schemas generated by the `Union` macro are no longer inlined by default.

# [1.2.41] 2022-1-21

- Set Rapidoc's `schema-description-expanded` option to `true`.

# [1.2.40] 2022-1-21

- Add `flatten` `attribute` for `Object` macro.

# [1.2.38] 2022-1-19

- Fix stack overflow when generating schema for structure references Self. [#171](https://github.com/poem-web/poem/issues/171)

# [1.2.36] 2022-1-17

- Add `HashSet` and `BTreeSet` support to OpenAPI. [#167](https://github.com/poem-web/poem/pull/167)
- Add `Url` support to OpenAPI. [#168](https://github.com/poem-web/poem/pull/168)
- **Breaking change:** Remove `OneOf` macro and add `Union` macro, and replace `property_name` with `discriminator_name`.
  - The implementation of `OneOf` was incorrect. To migrate, change all instances of `OneOf` to `Union`, and all instances of `property_name` to `discriminator_name`.

# [1.2.34] 2022-1-14

- Add `deprecated` attribute to `ApiResponse`'s header field.

# [1.2.33] 2022-1-12

- Fixed the `externalDocs` field name is incorrect in the specification.

# [1.2.32] 2022-1-12

- Add `ToJSON::to_json_string` method.

# [1.2.31] 2022-1-12

- Add support for custom validator.
- Add `external_docs` attribute for some macros.
- Add `ParseFromJSON::parse_from_json_string` method.

# [1.2.29] 2022-1-11

- Add `NewType` macro. [#159](https://github.com/poem-web/poem/issues/159)

# [1.2.27] 2022-1-8

- Add `ResponseContent` macro.

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

- Remove the `OpenApi::combine` method, `OpenApiService::new` can be passed a tuple to combine multiple API objects.

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

- If an OpenAPI name conflict is detected when creating schema, it will cause panic.

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
