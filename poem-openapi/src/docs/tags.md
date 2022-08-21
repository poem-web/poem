Define a OpenAPI Tags.

# Macro parameters

| Attribute  | Description                                                                                                                                                                                                           | Type   | Optional |
|------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------|----------|
| rename_all | Rename all the items according to the given case convention. The possible values are "lowercase", "UPPERCASE", "PascalCase", "camelCase", "snake_case", "SCREAMING_SNAKE_CASE", "kebab-case", "SCREAMING-KEBAB-CASE". | string | Y        |

# Item parameters

| Attribute |   | Description         | Type   | Optional |
|-----------|---|---------------------|--------|----------|
| rename    |   | Rename the tag name | string | Y        |
| external_docs | Specify a external resource for extended documentation | string              | Y      |

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