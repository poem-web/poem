Define a OpenAPI enum

# Macro parameters

| Attribute     | Description                                                                                                                                                                     | Type   | Optional |
|---------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------|----------|
| rename        | Rename the enum                                                                                                                                                                 | string | Y        |
| rename_all    | Rename all the items according to the given case convention. The possible values are "lowercase", "UPPERCASE", "PascalCase", "camelCase", "snake_case", "SCREAMING_SNAKE_CASE". | string | Y        |
| deprecated    | Schema deprecated                                                                                                                                                               | bool   | Y        |
| external_docs | Specify a external resource for extended documentation                                                                                                                          | string | Y        |
| remote        | Derive a remote enum                                                                                                                                                            | string | Y        |

# Item parameters

| Attribute | Description     | Type   | Optional |
|-----------|-----------------|--------|----------|
| rename    | Rename the item | string | Y        |

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