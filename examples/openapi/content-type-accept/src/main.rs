mod bcs_payload;

use std::sync::atomic::{AtomicU64, Ordering};

use bcs_payload::Bcs;
use poem::{listener::TcpListener, web::Accept, Result, Route, Server};
use poem_openapi::{
    payload::Json,
    types::{ParseFromJSON, ToJSON, Type},
    ApiRequest, ApiResponse, Object, OpenApi, OpenApiService, ResponseContent,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Clone, Deserialize, Object, Serialize)]
struct SubmitTransactionRequest {
    /// The address of the account submitting the transaction.
    sender: String,

    /// The sequence number of that account.
    sequence_number: u64,

    /// The payload of the transaction. Overly simplified to a String for the
    /// sake of this example.
    payload: String,
}

#[derive(Clone, Deserialize, Object, Serialize)]
struct CommittedTransaction {
    #[oai(flatten)]
    pub submit_transaction_request: SubmitTransactionRequest,

    /// The resulting version of the data store after the transaction was
    /// committed.
    pub version: u64,
}

#[derive(ApiRequest)]
enum MyRequest<T: ParseFromJSON + Send + Sync + Type + ToJSON + for<'b> Deserialize<'b>> {
    Json(Json<T>),
    Bcs(Bcs<T>),
}

impl<T: ParseFromJSON + Send + Sync + Type + ToJSON + for<'b> Deserialize<'b>> MyRequest<T> {
    fn unpack(self) -> T {
        match self {
            MyRequest::Json(Json(value)) => value,
            MyRequest::Bcs(Bcs(value)) => value,
        }
    }
}

#[derive(ResponseContent)]
enum MyResponseContent<T: ToJSON + Send + Sync + Serialize> {
    Json(Json<T>),
    Bcs(Bcs<T>),
}

#[derive(ApiResponse)]
enum MyResponse<T: ToJSON + Send + Sync + Serialize> {
    #[oai(status = 200)]
    Ok(MyResponseContent<T>),
}

struct Api {
    transactions: Mutex<Vec<CommittedTransaction>>,
    version: AtomicU64,
}

impl Api {
    fn new() -> Self {
        Self {
            transactions: Mutex::new(vec![]),
            version: AtomicU64::new(0),
        }
    }
}

fn create_response<T: ToJSON + Send + Sync + Serialize>(accept: &Accept, resp: T) -> MyResponse<T> {
    for mime in &accept.0 {
        match mime.as_ref() {
            "application/json" => return MyResponse::Ok(MyResponseContent::Json(Json(resp))),
            "application/x-bcs" => return MyResponse::Ok(MyResponseContent::Bcs(Bcs(resp))),
            _ => {}
        }
    }

    // default to Json
    MyResponse::Ok(MyResponseContent::Json(Json(resp)))
}

#[OpenApi]
impl Api {
    /// get_transaction
    ///
    /// Get the latest committed transaction.
    #[oai(path = "/transaction", method = "get")]
    async fn get(&self, accept: Accept) -> MyResponse<Option<CommittedTransaction>> {
        // TODO: Handle when transactions is empty.
        let transaction = self.transactions.lock().await.last().cloned();
        // Return BCS if the user requested it with Accept.
        create_response(&accept, transaction)
    }

    /// submit_transaction
    ///
    /// Submit a transaction. Returns the new version of the data store.
    #[oai(path = "/transaction", method = "put")]
    async fn put(
        &self,
        accept: Accept,
        request: MyRequest<SubmitTransactionRequest>,
    ) -> MyResponse<u64> {
        let committed_transaction = CommittedTransaction {
            submit_transaction_request: request.unpack(),
            version: self.version.fetch_add(1, Ordering::Relaxed),
        };
        self.transactions.lock().await.push(committed_transaction);
        // Return BCS if the user requested it with Accept.
        create_response(&accept, self.version.load(Ordering::Relaxed))
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }

    let api_service =
        OpenApiService::new(Api::new(), "Hello World", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();
    let yaml = api_service.spec_endpoint();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(
            Route::new()
                .nest("/api", api_service)
                .nest("/", ui)
                .nest("/spec", yaml),
        )
        .await
}
