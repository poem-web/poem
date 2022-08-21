Define a OAuth scopes.

# Macro parameters

| Attribute  | Description                                                                                                                                                                                                           | Type   | Optional |
|------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------|----------|
| rename_all | Rename all the items according to the given case convention. The possible values are "lowercase", "UPPERCASE", "PascalCase", "camelCase", "snake_case", "SCREAMING_SNAKE_CASE", "kebab-case", "SCREAMING-KEBAB-CASE". | string | Y        |

# Item parameters

| Attribute | Description           | Type   | Optional |
|-----------|-----------------------|--------|----------|
| rename    | Rename the scope name | string | Y        |

# Examples

```rust
use poem_openapi::OAuthScopes;

#[derive(OAuthScopes)]
enum GithubScopes {
    /// Read data
    Read,
    /// Write data
    Write,
}
```