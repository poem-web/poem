# Basic types

The basic type can be used as a request parameter, request content or response content. `Poem` defines a `Type` trait to
represent a basic type, which can provide some information about the type at runtime to generate OpenAPI definitions.

`Poem` implements `Type` traits for most common types, you can use them directly, and you can also customize new types,
but you need to have a certain understanding of [Json Schema](https://json-schema.org/).

The following table lists the Rust data types corresponding to some OpenAPI data types:

| Open API                                | Rust                              |
|-----------------------------------------|-----------------------------------|
| `{type: "integer", format: "int32" }`   | i32                               |
| `{type: "integer", format: "float32" }` | f32                               |
| `{type: "bool" }`                       | bool                              |
| `{type: "string" }`                     | String, &str                      |
| `{type: "string", format: "binary" }`   | Binary                            |
| `{type: "string", format: "bytes" }`    | Base64                            |
| `{type: "array" }`                      | Vec<T>                            |
