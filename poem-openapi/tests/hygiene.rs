#![no_implicit_prelude]
#![allow(dead_code)]

use ::poem_openapi;
use ::std::boxed::Box;

#[derive(::poem_openapi::Enum, Debug, Eq, PartialEq)]
enum MyEnum {
    A,
    B,
    C,
}

#[derive(::poem_openapi::Object, Debug, Eq, PartialEq)]
struct CreateUser {
    user: ::std::string::String,
    password: ::std::string::String,
}

#[derive(::poem_openapi::ApiRequest, Debug, Eq, PartialEq)]
enum CreateUserRequest {
    /// CreateByJson
    CreateByJson(::poem_openapi::payload::Json<CreateUser>),
    /// CreateByPlainText
    CreateByPlainText(::poem_openapi::payload::PlainText<::std::string::String>),
}

#[derive(::poem_openapi::ApiResponse)]
#[oai(bad_request_handler = "bad_request_handler")]
enum CreateUserResponse {
    /// Returns when the user is successfully created.
    #[oai(status = 200)]
    Ok,
    /// Returns when the user already exists.
    #[oai(status = 409)]
    UserAlreadyExists,
    /// Returns when the request parameters is incorrect.
    #[oai(status = 400)]
    BadRequest(::poem_openapi::payload::PlainText<::std::string::String>),
}

fn bad_request_handler(err: ::poem_openapi::ParseRequestError) -> CreateUserResponse {
    CreateUserResponse::BadRequest(::poem_openapi::payload::PlainText(::std::format!(
        "error: {}",
        ::std::string::ToString::to_string(&err)
    )))
}

#[derive(Default)]
struct Api {
    users: ::tokio::sync::Mutex<
        ::std::collections::HashMap<::std::string::String, ::std::string::String>,
    >,
}

#[::poem_openapi::OpenApi]
impl Api {
    /// Create a new user
    ///
    /// A
    /// B
    ///
    /// C
    #[oai(path = "/users", method = "post")]
    #[allow(unused_variables)]
    async fn create_user(
        &self,
        /// api key
        key: poem_openapi::param::Query<::std::string::String>,
        #[oai(name = "X-API-TOKEN", deprecated)] api_token: poem_openapi::param::Header<
            ::std::option::Option<::std::string::String>,
        >,
        req: CreateUserRequest,
    ) -> CreateUserResponse {
        let mut users = self.users.lock().await;

        match req {
            CreateUserRequest::CreateByJson(req) => {
                if users.contains_key(&req.0.user) {
                    return CreateUserResponse::UserAlreadyExists;
                }
                users.insert(req.0.user, req.0.password);
                CreateUserResponse::Ok
            }
            CreateUserRequest::CreateByPlainText(req) => {
                let s = ::std::iter::Iterator::collect::<::std::vec::Vec<_>>(req.0.split(':'));
                if s.len() != 2 {
                    return CreateUserResponse::BadRequest(poem_openapi::payload::PlainText(
                        ::std::string::ToString::to_string("invalid plain text request"),
                    ));
                }

                if users.contains_key(s[0]) {
                    return CreateUserResponse::UserAlreadyExists;
                }
                users.insert(
                    ::std::string::ToString::to_string(s[0]),
                    ::std::string::ToString::to_string(s[1]),
                );
                CreateUserResponse::Ok
            }
        }
    }
}

#[derive(::poem_openapi::Multipart, Debug, Eq, PartialEq)]
#[oai(rename_all = "UPPERCASE")]
struct A {
    name: ::std::string::String,
    file: ::poem_openapi::types::Binary,
}

#[derive(::poem_openapi::Object, Debug, PartialEq)]
struct A1 {
    v1: i32,
    v2: ::std::string::String,
}

#[derive(::poem_openapi::Object, Debug, PartialEq)]
struct B1 {
    v3: f32,
}

#[derive(::poem_openapi::OneOf, Debug, PartialEq)]
#[oai(property_name = "type")]
enum MyOneOf {
    A(A1),
    B(B1),
}

#[derive(::poem_openapi::Tags)]
#[oai(rename_all = "camelCase")]
enum MyTags {
    UserOperations,
    PetOperations,
}

#[derive(::poem_openapi::SecurityScheme)]
#[oai(type = "basic")]
struct BasicSecurityScheme(::poem_openapi::auth::Basic);

#[derive(::poem_openapi::SecurityScheme)]
#[oai(type = "bearer")]
struct MyBearerScheme(::poem_openapi::auth::Bearer);

#[derive(::poem_openapi::SecurityScheme)]
#[oai(type = "api_key", key_name = "X-API-Key", in = "header")]
struct MySecuritySchemeInHeader(::poem_openapi::auth::ApiKey);

#[derive(::poem_openapi::SecurityScheme)]
#[oai(type = "api_key", key_name = "key", in = "query")]
struct MySecuritySchemeInQuery(::poem_openapi::auth::ApiKey);

#[derive(::poem_openapi::SecurityScheme)]
#[oai(type = "api_key", key_name = "key", in = "cookie")]
struct MySecuritySchemeInCookie(::poem_openapi::auth::ApiKey);

#[derive(::poem_openapi::OAuthScopes)]
enum GithubScopes {
    Read,
    Write,
}

#[derive(::poem_openapi::SecurityScheme)]
#[oai(
    type = "oauth2",
    flows(
        implicit(
            authorization_url = "https://test.com/authorize",
            scopes = "GithubScopes"
        ),
        password(token_url = "https://test.com/token"),
        client_credentials(token_url = "https://test.com/token"),
        authorization_code(
            authorization_url = "https://test.com/authorize",
            token_url = "https://test.com/token"
        ),
    )
)]
struct OAuth2SecurityScheme(::poem_openapi::auth::Bearer);

#[test]
fn test_hygiene() {}
