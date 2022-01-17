Define a OpenAPI discriminator object.

# Macro parameters

| Attribute          | description                                                                     | Type   | Optional |
|--------------------|---------------------------------------------------------------------------------|--------|----------|
| discriminator_name | The name of the property in the payload that will hold the discriminator value. | string | Y        |
| external_docs      | Specify a external resource for extended documentation                          | string | Y        |

# Item parameters

| Attribute | description                                            | Type   | Optional |
|-----------|--------------------------------------------------------|--------|----------|
| mapping   | Rename the payload value. (Default is the object name) | string | Y        |

# Example with discriminator

```rust
use poem_openapi::{Object, AnyOf};

#[derive(Object, Debug, PartialEq)]
struct A {
    v1: i32,
    v2: String,
}

#[derive(Object, Debug, PartialEq)]
struct B {
    v3: f32,
}

#[derive(AnyOf, Debug, PartialEq)]
#[oai(discriminator_name = "type")]
enum MyObj {
    A(A),
    B(B),
}
```

# Example without discriminator

```rust
use poem_openapi::{Object, AnyOf};

#[derive(Object, Debug, PartialEq)]
struct A {
    v1: i32,
    v2: String,
}

#[derive(AnyOf, Debug, PartialEq)]
enum MyObj {
    A(A),
    B(bool),
    C(String),
}
```
