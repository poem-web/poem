# 文件上传

`Multipart`宏通常用于文件上传，它可以定义一个表单来包含一个或者多个文件以及一些附加字段。下面的例子提供一个创建`Pet`对象的接口，它在创建`Pet`对象的同时上传一些图片文件。

```rust
use poem_openapi::{Multipart, OpenApi};
use poem::Result;

#[derive(Debug, Multipart)]
struct CreatePetPayload {
    name: String,
    status: PetStatus,
    photos: Vec<Upload>, // 多个照片文件
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/pet", method = "post")]
    async fn create_pet(&self, payload: CreatePetPayload) -> Result<Json<u64>> {
        todo!()
    }
}
```

完整的代码请参考[文件上传例子](https://github.com/poem-web/poem/tree/master/examples/openapi/upload`)。
