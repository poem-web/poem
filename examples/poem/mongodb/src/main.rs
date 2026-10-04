use std::io;

use futures::TryStreamExt;
use mongodb::{
    Client, Collection,
    bson::{Document, doc},
};
use poem::{
    EndpointExt, Route, Server, get, handler,
    listener::TcpListener,
    middleware::AddData,
    web::{Data, Json},
};
use serde::Deserialize;

#[handler]
async fn get_users(collection: Data<&Collection<Document>>) -> Json<serde_json::Value> {
    let cursor = collection.find(doc! {}).await.unwrap();
    let result = cursor.try_collect::<Vec<Document>>().await.unwrap();

    Json(serde_json::json!(result))
}

#[derive(Deserialize)]
struct InsertableUser {
    name: String,
    email: String,
    age: u32,
}

#[handler]
async fn create_user(
    collection: Data<&Collection<Document>>,
    req: Json<InsertableUser>,
) -> Json<serde_json::Value> {
    let result = collection
        .insert_one(doc! {
            "name": &req.name,
            "email": &req.email,
            "age": req.age
        })
        .await
        .unwrap();
    let result = collection
        .find_one(doc! {"_id": result.inserted_id})
        .await
        .unwrap();

    Json(serde_json::json!(result))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let filter = match std::env::var_os("RUST_LOG") {
        Some(_) => tracing_subscriber::EnvFilter::from_default_env(),
        None => tracing_subscriber::EnvFilter::new("poem=debug"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mongodb = Client::with_uri_str("mongodb://127.0.0.1:27017")
        .await
        .unwrap()
        .database("test");
    let collection = mongodb.collection::<Document>("user");

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(
            Route::new()
                .at("/user", get(get_users).post(create_user))
                .with(AddData::new(collection)),
        )
        .await
}
