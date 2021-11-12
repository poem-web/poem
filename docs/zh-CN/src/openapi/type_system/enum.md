# Enum

Use the procedural macro `Enum` to define an enumerated type.

**Poem-openapi will automatically change the name of each item to `SCREAMING_SNAKE_CASE` convention. You can use `rename_all` attribute to rename all items.**

```rust
use poem_api::Enum;

#[derive(Enum)]
enum PetStatus {
    Available,
    Pending,
    Sold,
}
```
