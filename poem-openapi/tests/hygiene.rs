#![no_implicit_prelude]

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
        err
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
        #[oai(name = "key", in = "query", desc = "api key")] key: ::std::string::String,
        #[oai(name = "X-API-TOKEN", in = "header", deprecated)] api_token: ::std::option::Option<
            ::std::string::String,
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

#[test]
fn test_hygiene() {}
