# Object

Use the procedural macro `Object` to define an object. All object members must be types that implement the `Type trait`
(unless you mark it with `#[oai(skip)]`, the field will be ignored serialization and use the default value instead).

Use the following code to define an object type, which contains four fields, one of which is an enumerated type.

_Object type is also a kind of basic type, it also implements the `Type` trait, so it can also be a member of another object._

**Poem-openapi will automatically change the name of each member to `camelCase` convention. You can use `rename_all` attribute to rename all items.**

```rust
use poem_api::{Object, Enum};

#[derive(Enum)]
enum PetStatus {
    Available,
    Pending,
    Sold,
}

#[derive(Object)]
struct Pet {
    id: u64,
    name: String,
    photo_urls: Vec<String>,
    status: PetStatus,
}
```
