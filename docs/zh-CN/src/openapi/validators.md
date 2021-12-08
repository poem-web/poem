# 参数校验

`OpenAPI`引用了`Json Schema`的校验规范，`Poem-openapi`同样支持它们。你可以在请求的参数，对象的成员和`Multipart`的字段三个地方应用校验器。校验器是类型安全的，如果待校验的数据类型和校验器所需要的不匹配，那么将无法编译通过。例如`maximum`只能用于数值类型，`max_items`只能用于数组类型。

更多的校验器请参考[文档](https://docs.rs/poem-openapi/*/poem_openapi/attr.OpenApi.html#operation-argument-parameters)。

```rust
use poem_openapi::{Object, OpenApi, Multipart};

#[derive(Object)]
struct Pet {
    id: u64,

    /// 名字长度不能超过32
    #[oai(validator(max_length = "32"))]
    name: String,

    /// 数组长度不能超过3，并且url长度不能超过256
    #[oai(validator(max_items = "3", max_length = "256"))]
    photo_urls: Vec<String>,

    status: PetStatus,
}
```
