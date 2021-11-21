# 认证

OpenApi规范定义了`apikey`，`basic`，`bearer`，`oauth2`，`openIdConnect`五种认证模式，它们描述了指定的`API`接口需要的认证参数。

下面的例子是用`Github`登录，并提供一个获取所有公共仓库信息的接口。

```rust
use poem_openapi::{
    SecurityScheme, SecurityScope, OpenApi,
    auth::Bearer,
};

#[derive(OAuthScopes)]
enum GithubScope {
    /// 可访问公共仓库信息。
    #[oai(rename = "public_repo")]
    PublicRepo,

    /// 可访问用户的个人资料数据。
    #[oai(rename = "read:user")]
    ReadUser,
}

/// Github 认证
#[derive(SecurityScheme)]
#[oai(
    type = "oauth2",
    flows(authorization_code(
        authorization_url = "https://github.com/login/oauth/authorize",
        token_url = "https://github.com/login/oauth/token",
        scopes = "GithubScope",
    ))
)]
struct GithubAuthorization(Bearer);

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/repo", method = "get")]
    async fn repo_list(
        &self,
        #[oai(scope("GithubScope::PublicRepo"))] auth: GithubAuthorization,
    ) -> Result<PlainText<String>> {
        // 使用GithubAuthorization得到的token向Github获取所有公共仓库信息。
        todo!()
    }
}
```

完整的代码请参考[例子](https://github.com/poem-web/poem/tree/master/examples/openapi/auth-github)。

## 检查认证信息

您可以使用`checker`属性指定一个检查器函数来检查原始认证信息和将其转换为该函数的返回类型。 此函数必须返回`Option<T>`，如果检查失败则返回`None`。 

```rust
struct User {
    username: String,
}

/// ApiKey 认证
#[derive(SecurityScheme)]
#[oai(
    type = "api_key",
    key_name = "X-API-Key",
    in = "header",
    checker = "api_checker"
)]
struct MyApiKeyAuthorization(User);

async fn api_checker(req: &Request, api_key: ApiKey) -> Option<User> {
    let connection = req.data::<DbConnection>().unwrap();
    
    // 在数据库中检查
    todo!()
}
```

完整的代码请参考[例子](https://github.com/poem-web/poem/tree/master/examples/openapi/auth-apikey).