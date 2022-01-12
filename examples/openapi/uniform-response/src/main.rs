use poem::{listener::TcpListener, Error, Route, Server};
use poem_openapi::{
    error::ParseJsonError,
    payload::Json,
    types::{ParseFromJSON, ToJSON},
    ApiResponse, Object, OpenApi, OpenApiService,
};
use tokio::sync::Mutex;

const ERRCODE_NOT_FOUND: i32 = -1;
const ERRCODE_INVALID_REQUEST: i32 = -2;
const ERRCODE_UNKNOWN: i32 = -10000;

#[derive(Object, Clone)]
struct Resource {
    a: i32,
    b: String,
}

#[derive(Object)]
#[oai(inline)]
struct ResponseObject<T: ParseFromJSON + ToJSON + Send + Sync> {
    code: i32,
    msg: String,
    data: Option<T>,
}

impl<T: ParseFromJSON + ToJSON + Send + Sync> ResponseObject<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: 0,
            msg: "OK".to_string(),
            data: Some(data),
        }
    }

    pub fn not_found() -> Self {
        Self {
            code: ERRCODE_NOT_FOUND,
            msg: "Not found".to_string(),
            data: None,
        }
    }
}

#[derive(ApiResponse)]
#[oai(bad_request_handler = "my_bad_request_handler")]
enum MyResponse<T: ParseFromJSON + ToJSON + Send + Sync> {
    #[oai(status = 200)]
    Ok(Json<ResponseObject<T>>),
}

fn my_bad_request_handler<T: ParseFromJSON + ToJSON + Send + Sync>(err: Error) -> MyResponse<T> {
    if err.is::<ParseJsonError>() {
        MyResponse::Ok(Json(ResponseObject {
            code: ERRCODE_INVALID_REQUEST,
            msg: err.to_string(),
            data: None,
        }))
    } else {
        MyResponse::Ok(Json(ResponseObject {
            code: ERRCODE_UNKNOWN,
            msg: err.to_string(),
            data: None,
        }))
    }
}

struct Api {
    resource: Mutex<Option<Resource>>,
}

#[OpenApi]
impl Api {
    #[oai(path = "/resource", method = "get")]
    async fn get(&self) -> MyResponse<Resource> {
        let res = self.resource.lock().await;
        match &*res {
            Some(resource) => MyResponse::Ok(Json(ResponseObject::ok(resource.clone()))),
            None => MyResponse::Ok(Json(ResponseObject::not_found())),
        }
    }

    #[oai(path = "/resource", method = "put")]
    async fn put(&self, obj: Json<Resource>) -> MyResponse<bool> {
        *self.resource.lock().await = Some(obj.0);
        MyResponse::Ok(Json(ResponseObject::ok(true)))
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let api_service = OpenApiService::new(
        Api {
            resource: Default::default(),
        },
        "Hello World",
        "1.0",
    )
    .server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}
