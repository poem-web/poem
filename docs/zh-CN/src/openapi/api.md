# 定义API

下面定义一组API对宠物表进行增删改查的操作。

`add_pet`和`update_pet`用于添加和更新`Pet`对象，**这是我们在之前定义的基本类型，基本类型不能直接作为请求内容，需要使用一个`Payload`类型来包装它**，这样就可以确定内容的`Content-Type`。在下面的例子中，我们使用`payload::Json`来包装它，表示这两个API请求内容的`Content-Type`为`application/json`。

`find_pet_by_id`和`find_pets_by_status`用于查找`Pet`对象，它们的响应也是一个`Pet`对象，同样需要使用`Payload`类型来包装。

我们可以用`#[oai(name = "...", in = "...")]`来修饰一个函数参数用于指定此参数值的来源，`in`的值可以是`query`, `path`, `header`, `cookie`四种类型。`delete_pet`的`id`参数从路径中提取，`find_pet_by_id`和`find_pets_by_status`的参数从Query中获取。如果参数类型不是`Option<T>`，那么表示这个参数不是一个可选参数，提取失败时会返回`400 Bad Request`错误。

你可以定义多个函数参数，但只能有一个`Payload`类型作为请求内容，或者多个基本类型作为请求的参数。

```rust
use poem_api::{
  OpenApi,
  poem_api::payload::Json,
};
use poem::Result;

struct Api;

#[OpenApi]
impl Api {
    /// 添加新Pet
    #[oai(path = "/pet", method = "post")]
    async fn add_pet(&self, pet: Json<Pet>) -> Result<()> {
        todo!()
    }
  
    /// 更新已有的Pet
    #[oai(path = "/pet", method = "put")]
    async fn update_pet(&self, pet: Json<Pet>) -> Result<()> {
        todo!()
    }

    /// 删除一个Pet
    #[oai(path = "/pet/:pet_id", method = "delete")]
    async fn delete_pet(&self, #[oai(name = "pet_id", in = "path")] id: u64) -> Result<()> {
        todo!()
    }
  
    /// 根据ID查询Pet
    #[oai(path = "/pet", method = "get")]
    async fn find_pet_by_id(&self, #[oai(name = "status", in = "query")] id: u64) -> Result<Json<Pet>> {
        todo!()
    } 
  
    /// 根据状态查询Pet
    #[oai(path = "/pet/findByStatus", method = "get")]
    async fn find_pets_by_status(&self, #[oai(name = "status", in = "query")] status: Status) -> Result<Json<Vec<Pet>>> {
        todo!()
    }
}
```
