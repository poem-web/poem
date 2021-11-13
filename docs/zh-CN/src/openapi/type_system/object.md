# 对象类型

用过程宏`Object`来定义一个对象，对象的成员必须是实现了`Type trait`的类型（除非你用`#[oai(skip)]`来标注它，那么序列化和反序列化时降忽略该字段用默认值代替）。

以下代码定义了一个对象类型，它包含四个字段，其中有一个字段是枚举类型。

_对象类型也是基础类型的一种，它同样实现了`Type trait`，所以它也可以作为另一个对象的成员。_

**Poem-openapi 会自动将每个成员的名称更改为 `camelCase` 约定。 你可以使用 `rename_all` 属性来重命名所有项。**

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
