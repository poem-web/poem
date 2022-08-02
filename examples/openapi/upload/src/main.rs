use std::collections::HashMap;

use poem::{error::BadRequest, listener::TcpListener, Result, Route, Server};
use poem_openapi::{
    param::Path,
    payload::{Attachment, AttachmentType, Json},
    types::multipart::Upload,
    ApiResponse, Multipart, Object, OpenApi, OpenApiService,
};
use tokio::sync::Mutex;

#[derive(Debug, Object, Clone)]
struct File {
    name: String,
    desc: Option<String>,
    content_type: Option<String>,
    filename: Option<String>,
    data: Vec<u8>,
}

#[derive(Debug, ApiResponse)]
enum GetFileResponse {
    #[oai(status = 200)]
    Ok(Attachment<Vec<u8>>),
    /// File not found
    #[oai(status = 404)]
    NotFound,
}

struct Status {
    id: u64,
    files: HashMap<u64, File>,
}

#[derive(Debug, Multipart)]
struct UploadPayload {
    name: String,
    desc: Option<String>,
    file: Upload,
}

struct Api {
    status: Mutex<Status>,
}

#[OpenApi]
impl Api {
    /// Upload file
    #[oai(path = "/files", method = "post")]
    async fn upload(&self, upload: UploadPayload) -> Result<Json<u64>> {
        let mut status = self.status.lock().await;
        let id = status.id;
        status.id += 1;

        let file = File {
            name: upload.name,
            desc: upload.desc,
            content_type: upload.file.content_type().map(ToString::to_string),
            filename: upload.file.file_name().map(ToString::to_string),
            data: upload.file.into_vec().await.map_err(BadRequest)?,
        };
        status.files.insert(id, file);
        Ok(Json(id))
    }

    /// Get file
    #[oai(path = "/files/:id", method = "get")]
    async fn get(&self, id: Path<u64>) -> GetFileResponse {
        let status = self.status.lock().await;
        match status.files.get(&id) {
            Some(file) => {
                let mut attachment =
                    Attachment::new(file.data.clone()).attachment_type(AttachmentType::Attachment);
                if let Some(filename) = &file.filename {
                    attachment = attachment.filename(filename);
                }
                GetFileResponse::Ok(attachment)
            }
            None => GetFileResponse::NotFound,
        }
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
            status: Mutex::new(Status {
                id: 1,
                files: Default::default(),
            }),
        },
        "Upload Files",
        "1.0",
    )
    .server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(Route::new().nest("/api", api_service).nest("/", ui))
        .await
}
