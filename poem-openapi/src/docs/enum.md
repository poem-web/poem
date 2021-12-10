Define a OpenAPI enum

# Macro parameters

| Attribute  | description                                                                                                                                                                     | Type   | Optional |
|------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------|----------|
| rename     | Rename the enum                                                                                                                                                                 | string | Y        |
| rename_all | Rename all the items according to the given case convention. The possible values are "lowercase", "UPPERCASE", "PascalCase", "camelCase", "snake_case", "SCREAMING_SNAKE_CASE". | string | Y        |

# Item parameters

| Attribute   | description               | Type     | Optional |
|-------------|---------------------------|----------|----------|
| rename      | Rename the item           | string   | Y        |

# Examples

```rust
use poem_openapi::Enum;

#[derive(Enum)]
enum PetStatus {
    Available,
    Pending,
    Sold,
}
```