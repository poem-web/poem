use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt;
use poem::{endpoint::BoxEndpoint, IntoEndpoint};
use prost::Message;
use prost_types::{DescriptorProto, EnumDescriptorProto, FileDescriptorProto, FileDescriptorSet};
use proto::{
    server_reflection_request::MessageRequest, server_reflection_response::MessageResponse,
};

use crate::{include_file_descriptor_set, Code, Request, Response, Service, Status, Streaming};

#[allow(private_in_public, unreachable_pub)]
#[allow(clippy::enum_variant_names)]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/grpc.reflection.v1alpha.rs"));
}

pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = include_file_descriptor_set!("grpc-reflection.bin");

struct State {
    service_names: Vec<proto::ServiceResponse>,
    files: HashMap<String, Arc<FileDescriptorProto>>,
    symbols: HashMap<String, Arc<FileDescriptorProto>>,
}

impl State {
    fn file_by_filename(&self, filename: &str) -> Result<MessageResponse, Status> {
        match self.files.get(filename) {
            None => {
                Err(Status::new(Code::NotFound)
                    .with_message(format!("file '{}' not found", filename)))
            }
            Some(fd) => {
                let mut encoded_fd = Vec::new();
                if fd.clone().encode(&mut encoded_fd).is_err() {
                    return Err(Status::new(Code::Internal).with_message("encoding error"));
                }

                Ok(MessageResponse::FileDescriptorResponse(
                    proto::FileDescriptorResponse {
                        file_descriptor_proto: vec![encoded_fd],
                    },
                ))
            }
        }
    }

    fn symbol_by_name(&self, symbol: &str) -> Result<MessageResponse, Status> {
        match self.symbols.get(symbol) {
            None => {
                Err(Status::new(Code::NotFound)
                    .with_message(format!("symbol '{}' not found", symbol)))
            }
            Some(fd) => {
                let mut encoded_fd = Vec::new();
                if fd.clone().encode(&mut encoded_fd).is_err() {
                    return Err(Status::new(Code::Internal).with_message("encoding error"));
                };

                Ok(MessageResponse::FileDescriptorResponse(
                    proto::FileDescriptorResponse {
                        file_descriptor_proto: vec![encoded_fd],
                    },
                ))
            }
        }
    }

    fn list_services(&self) -> MessageResponse {
        MessageResponse::ListServicesResponse(proto::ListServiceResponse {
            service: self.service_names.clone(),
        })
    }
}

/// A service that serve for reflection
struct ServerReflectionService {
    state: Arc<State>,
}

#[poem::async_trait]
impl proto::ServerReflection for ServerReflectionService {
    async fn server_reflection_info(
        &self,
        request: Request<Streaming<proto::ServerReflectionRequest>>,
    ) -> Result<Response<Streaming<proto::ServerReflectionResponse>>, Status> {
        let mut request_stream = request.into_inner();
        let state = self.state.clone();

        Ok(Response::new(Streaming::new(async_stream::try_stream! {
            while let Some(req) = request_stream.next().await.transpose()? {
                let resp = match &req.message_request {
                    Some(MessageRequest::FileByFilename(filename)) => state.file_by_filename(filename),
                    Some(MessageRequest::FileContainingSymbol(symbol)) => state.symbol_by_name(symbol),
                    Some(MessageRequest::FileContainingExtension(_) | MessageRequest::AllExtensionNumbersOfType(_)) => Err(Status::new(Code::Unimplemented)),
                    Some(MessageRequest::ListServices(_)) => Ok(state.list_services()),
                    None => Err(Status::new(Code::InvalidArgument)),
                }?;

                yield proto::ServerReflectionResponse {
                    valid_host: req.host.clone(),
                    original_request: Some(req.clone()),
                    message_response: Some(resp),
                };
            }
        })))
    }
}

/// A builder for creating reflection service
#[derive(Debug, Default)]
pub struct Reflection {
    file_descriptor_sets: Vec<FileDescriptorSet>,
    service_names: Vec<String>,
    symbols: HashMap<String, Arc<FileDescriptorProto>>,
}

impl Reflection {
    /// Create a `ReflectionBuilder`
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a file descriptor set
    pub fn add_file_descriptor_set(mut self, data: &[u8]) -> Self {
        self.file_descriptor_sets
            .push(FileDescriptorSet::decode(data).expect("valid file descriptor sets"));
        self
    }

    /// Build a reflection service
    pub fn build(
        self,
    ) -> impl IntoEndpoint<Endpoint = BoxEndpoint<'static, poem::Response>> + Service {
        let mut this = self.add_file_descriptor_set(FILE_DESCRIPTOR_SET);

        let fd_iter = std::mem::take(&mut this.file_descriptor_sets)
            .into_iter()
            .flat_map(|fds| fds.file.into_iter());
        let mut files = HashMap::new();

        for fd in fd_iter {
            let fd = Arc::new(fd);

            match fd.name.clone() {
                Some(filename) => {
                    files.insert(filename, fd.clone());
                }
                None => panic!("missing file name"),
            }

            let prefix = fd.package.as_deref().unwrap_or_default();

            for proto in &fd.message_type {
                this.process_message(fd.clone(), prefix, proto);
            }

            for proto in &fd.enum_type {
                this.process_enum(fd.clone(), prefix, proto);
            }

            for service in &fd.service {
                let service_name = qualified_name(prefix, "service", service.name.as_deref());
                this.service_names.push(service_name.clone());
                this.symbols.insert(service_name.clone(), fd.clone());

                for method in &service.method {
                    let method_name =
                        qualified_name(&service_name, "method", method.name.as_deref());
                    this.symbols.insert(method_name, fd.clone());
                }
            }
        }

        proto::ServerReflectionServer::new(ServerReflectionService {
            state: Arc::new(State {
                service_names: this
                    .service_names
                    .into_iter()
                    .map(|name| proto::ServiceResponse { name })
                    .collect(),
                files,
                symbols: this.symbols,
            }),
        })
    }

    fn process_message(
        &mut self,
        fd: Arc<FileDescriptorProto>,
        prefix: &str,
        msg: &DescriptorProto,
    ) {
        let message_name = qualified_name(prefix, "message", msg.name.as_deref());
        self.symbols.insert(message_name.clone(), fd.clone());

        for nested in &msg.nested_type {
            self.process_message(fd.clone(), &message_name, nested);
        }

        for e in &msg.enum_type {
            self.process_enum(fd.clone(), &message_name, e);
        }

        for field in &msg.field {
            let field_name = qualified_name(prefix, "field", field.name.as_deref());
            self.symbols.insert(field_name, fd.clone());
        }

        for oneof in &msg.oneof_decl {
            let oneof_name = qualified_name(prefix, "oneof", oneof.name.as_deref());
            self.symbols.insert(oneof_name, fd.clone());
        }
    }

    fn process_enum(
        &mut self,
        fd: Arc<FileDescriptorProto>,
        prefix: &str,
        e: &EnumDescriptorProto,
    ) {
        let enum_name = qualified_name(prefix, "enum", e.name.as_deref());
        self.symbols.insert(enum_name.clone(), fd.clone());

        for value in &e.value {
            let value_name = qualified_name(&enum_name, "enum value", value.name.as_deref());
            self.symbols.insert(value_name, fd.clone());
        }
    }
}

fn qualified_name(prefix: &str, ty: &str, name: Option<&str>) -> String {
    match name {
        Some(name) if !prefix.is_empty() => format!("{}.{}", prefix, name),
        Some(name) => name.to_string(),
        None => panic!("missing {} name", ty),
    }
}
