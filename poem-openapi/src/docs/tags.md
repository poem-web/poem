Define a OpenAPI Tags.

# Macro parameters

| Attribute  | description                                                                                                                                                                     | Type   | Optional |
|------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------|----------|
| rename_all | Rename all the items according to the given case convention. The possible values are "lowercase", "UPPERCASE", "PascalCase", "camelCase", "snake_case", "SCREAMING_SNAKE_CASE". | string | Y        |

# Item parameters

| Attribute   |     | description               | Type     | Optional |
|-------------|:----|---------------------------|----------|----------|
| rename      |     | Rename the tag name       | string   | Y        |

# Examples

```rust
use poem_openapi::Tags;

#[derive(Tags)]
enum ApiTags {
    /// Operations about user
    User,
    /// Operations about pet
    Pet,
}
```