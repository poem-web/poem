# 定义API

下面定义一组API对宠物表进行增删改查的操作。

一个方法代表一个API操作，必须使用`path`和`method`属性指定操作的路径和方法。

方法的参数可以有多个，可以使用以下类型：

    - **poem_openapi::param::Query** 表示参数来自查询字符串

    - **poem_openapi::param::Header** 表示参数来自请求头

    - **poem_openapi::param::Path** 表示参数来自请求路径

    - **poem_openapi::param::Cookie** 表示参数来自Cookie

    - **poem_openapi::param::CookiePrivate** 表示参数来自加密的Cookie

    - **poem_openapi::param::CookieSigned** 表示参数来自签名后的Cookie

    - **poem_openapi::payload::Binary** 表示请求内容是二进制数据

    - **poem_openapi::payload::Json** 表示请求内容用Json编码

    - **poem_openapi::payload::PlainText** 表示请求内容是UTF8文本

    - **ApiRequest** 使用`ApiRequest`宏生成的请求体

    - **SecurityScheme** 使用`SecurityScheme`宏生成认证方法

    - **T: FromRequest** 使用Poem的提取器

返回值可以是任意实现了`ApiResponse`的类型。

```rust
use poem_api::{
    OpenApi,
    poem_api::payload::Json,
    param::{Path, Query},
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
    #[oai(path = "/pet/:id", method = "delete")]
    async fn delete_pet(&self, id: Path<u64>) -> Result<()> {
        todo!()
    }
  
    /// 根据ID查询Pet
    #[oai(path = "/pet", method = "get")]
    async fn find_pet_by_id(&self, id: Query<u64>) -> Result<Json<Pet>> {
        todo!()
    } 
  
    /// 根据状态查询Pet
    #[oai(path = "/pet/findByStatus", method = "get")]
    async fn find_pets_by_status(&self, status: Query<Status>) -> Result<Json<Vec<Pet>>> {
        todo!()
    }
}
```
