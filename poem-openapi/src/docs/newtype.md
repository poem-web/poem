Define a new type.

# Macro parameters

| Attribute      | Description                                                  | Type      | Optional |
|----------------|--------------------------------------------------------------|-----------|----------|
| from_json      | Implement `ParseFromJSON` trait. Default is `true`           | bool      | Y        |
| from_parameter | Implement `ParseFromParameter` trait. Default is `true`      | bool      | Y        |
| from_multipart | Implement `ParseFromMultipartField` trait. Default is `true` | bool      | Y        |
| to_json        | Implement `ToJSON` trait. Default is `true`                  | bool      | Y        |
| to_header      | Implement `ToHeader` trait. Default is `true`                | bool      | Y        |
| external_docs  | Specify a external resource for extended documentation       | string    | Y        |
| example        | Indicates that the type has implemented `Example` trait      | bool      | Y        |
| rename         | Rename the type                                              | string    | Y        |
| validator      | Add validators to the type (see examples below)              | Validator | Y        |

# Examples

```rust
use poem_openapi::NewType;

#[derive(NewType)]
struct MyString(String);
```

## With validators

Validators are applied at runtime during parsing and are also reflected in the generated OpenAPI schema.

```rust
use poem_openapi::NewType;

/// A username with length constraints
#[derive(NewType)]
#[oai(validator(min_length = 3, max_length = 50))]
struct Username(String);

/// A percentage value between 0 and 100
#[derive(NewType)]
#[oai(validator(minimum(value = 0.0), maximum(value = 100.0)))]
struct Percentage(f64);

/// An email address matching a regex pattern
#[derive(NewType)]
#[oai(validator(pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"))]
struct Email(String);
```
