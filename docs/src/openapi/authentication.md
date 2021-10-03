# Authentication

The OpenApi specification defines `apikey`, `basic`, `bearer`, `oauth2` and `openIdConnect` authentication modes, which
describe the authentication parameters required for the specified operation.

**Note: The main purpose of authentication information is to allow `Swagger UI` to correctly execute the authentication 
process when testing the API.**

The following example is to log in with `Github` and provide an operation to get all public repositories.

```rust
use poem_openapi::{
    SecurityScheme, SecurityScope, OpenApi,
    auth::Bearer,
};

#[derive(OAuthScopes)]
enum GithubScope {
    /// access to public repositories.
    #[oai(rename = "public_repo")]
    PublicRepo,

    /// access to read a user's profile data.
    #[oai(rename = "read:user")]
    ReadUser,
}

/// Github authorization
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
        #[oai(auth("GithubScope::PublicRepo"))] auth: GithubAuthorization,
    ) -> Result<PlainText<String>> {
        // Use the token in GithubAuthorization to obtain all public repositories from Github.
        todo!()
    }
}
```

For the complete example, please refer to [Auth Example](https://github.com/poem-web/poem/tree/master/examples/openapi/auth`).
