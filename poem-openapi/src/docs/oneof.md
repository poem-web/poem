Define a OpenAPI discriminator object.

# Macro parameters

| Attribute     | description                                                                     | Type   | Optional |
|---------------|---------------------------------------------------------------------------------|--------|----------|
| property_name | The name of the property in the payload that will hold the discriminator value. | string | Y        |

# Item parameters

| Attribute | description                                            | Type   | Optional |
|-----------|--------------------------------------------------------|--------|----------|
| mapping   | Rename the payload value. (Default is the object name) | string | Y        |

# Examples

```rust
use poem_openapi::{Object, OneOf};

#[derive(Object, Debug, PartialEq)]
struct A {
    v1: i32,
    v2: String,
}

#[derive(Object, Debug, PartialEq)]
struct B {
    v3: f32,
}

#[derive(OneOf, Debug, PartialEq)]
#[oai(property_name = "type")]
enum MyObj {
    A(A),
    B(B),
}
```