# 枚举

使用过程宏 `Enum` 来定义枚举类型。

**Poem-openapi 会自动将每一项的名称改为`SCREAMING_SNAKE_CASE` 约定。 您可以使用 `rename_all` 属性来重命名所有项目。** 

```rust
use poem_api::Enum;

#[derive(Enum)]
enum PetStatus {
    Available,
    Pending,
    Sold,
}
```
