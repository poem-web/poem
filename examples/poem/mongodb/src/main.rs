use std::io;

use futures::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    Client, Collection,
};
use poem::{
    handler,
    listener::TcpListener,
    middleware::AddData,
    route,
    route::get,
    web::{Data, Json},
    EndpointExt, Server,
};
use serde::Deserialize;

#[handler]
async fn get_users(collection: Data<&Collection<Document>>) -> Json<serde_json::Value> {
    let cursor = collection.find(None, None).await.unwrap();
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
        .insert_one(
            doc! {
                "name": &req.name,
                "email": &req.email,
                "age": req.age
            },
            None,
        )
        .await
        .unwrap();
    let result = collection
        .find_one(doc! {"_id": result.inserted_id}, None)
        .await
        .unwrap();

    Json(serde_json::json!(result))
}

#[tokio::main]
async fn main() -> io::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let mongodb = Client::with_uri_str("mongodb://127.0.0.1:27017")
        .await
        .unwrap()
        .database("test");
    let collection = mongodb.collection::<Document>("user");

    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server
        .run(
            route()
                .at("/user", get(get_users).post(create_user))
                .with(AddData::new(collection)),
        )
        .await?;

    Ok(())
}
