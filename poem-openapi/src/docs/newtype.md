Define a new type.

# Macro parameters

| Attribute      | description                                                  | Type   | Optional |
|----------------|--------------------------------------------------------------|--------|----------|
| from_json      | Implement `ParseFromJSON` trait. Default is `true`           | bool   | Y        |
| from_parameter | Implement `ParseFromParameter` trait. Default is `true`      | bool Y |
| from_multipart | Implement `ParseFromMultipartField` trait. Default is `true` | bool Y |
| to_json        | Implement `ToJSON` trait. Default is `true`                  | bool Y |
| to_header      | Implement `ToHeader` trait. Default is `true`                | bool Y |

# Examples

```rust
use poem_openapi::NewType;

#[derive(NewType)]
struct MyString(String);
```
