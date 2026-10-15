use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use futures_util::StreamExt;
use poem::{IntoEndpoint, endpoint::BoxEndpoint};
use prost::Message;
use prost_types::{
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
    FileDescriptorSet,
};
use v1::{server_reflection_request::MessageRequest, server_reflection_response::MessageResponse};

use crate::{Code, Request, Response, Service, Status, Streaming};

#[allow(unreachable_pub)]
#[allow(clippy::enum_variant_names)]
#[allow(clippy::derive_partial_eq_without_eq)]
mod v1 {
    include!(concat!(env!("OUT_DIR"), "/grpc.reflection.v1.rs"));
}

#[allow(unreachable_pub)]
#[allow(clippy::enum_variant_names)]
#[allow(clippy::derive_partial_eq_without_eq)]
mod v1alpha {
    include!(concat!(env!("OUT_DIR"), "/grpc.reflection.v1alpha.rs"));
}

pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = include_file_descriptor_set!("grpc-reflection.bin");

/// Used to split a `FileDescriptorSet` into opaque per-file bytes without
/// decoding their contents, so unknown fields / custom options survive the
/// round-trip.
#[derive(Clone, PartialEq, ::prost::Message)]
struct RawFileDescriptorSet {
    #[prost(bytes = "vec", repeated, tag = "1")]
    file: Vec<Vec<u8>>,
}

struct State {
    service_names: Vec<v1::ServiceResponse>,
    /// Decoded descriptors keyed by filename. Used for the symbol index, the
    /// transitive dependency walk, and as a fallback when raw bytes are absent.
    files: HashMap<String, Arc<FileDescriptorProto>>,
    /// Original wire bytes per filename, served verbatim to preserve unknown
    /// fields (custom options / extensions / newer descriptor fields).
    raw_files: HashMap<String, Vec<u8>>,
    /// Fully-qualified symbol -> filename.
    symbols: HashMap<String, String>,
    /// extendee -> (extension number -> defining filename).
    extensions: HashMap<String, HashMap<i32, String>>,
}

impl State {
    /// Bytes for `filename`, plus all transitive dependencies, deduplicated by
    /// file name. The original wire bytes are served verbatim (falling back to
    /// a re-encode only when a raw file is unavailable), so unknown fields
    /// / custom options survive the round-trip.
    fn file_descriptor_bytes(&self, filename: &str) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut queue = vec![filename.to_string()];

        while let Some(name) = queue.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }

            let bytes = match self.raw_files.get(&name).cloned() {
                Some(bytes) => bytes,
                None => match self.files.get(&name) {
                    Some(fd) => fd.clone().encode_to_vec(),
                    None => continue,
                },
            };
            out.push(bytes);

            if let Some(fd) = self.files.get(&name) {
                for dep in &fd.dependency {
                    queue.push(dep.clone());
                }
            }
        }

        out
    }

    #[allow(clippy::result_large_err)]
    fn file_by_filename(&self, filename: &str) -> Result<MessageResponse, Status> {
        if !self.files.contains_key(filename) && !self.raw_files.contains_key(filename) {
            return Err(
                Status::new(Code::NotFound).with_message(format!("file '{filename}' not found"))
            );
        }

        Ok(MessageResponse::FileDescriptorResponse(
            v1::FileDescriptorResponse {
                file_descriptor_proto: self.file_descriptor_bytes(filename),
            },
        ))
    }

    #[allow(clippy::result_large_err)]
    fn symbol_by_name(&self, symbol: &str) -> Result<MessageResponse, Status> {
        let filename = self.symbols.get(symbol).ok_or_else(|| {
            Status::new(Code::NotFound).with_message(format!("symbol '{symbol}' not found"))
        })?;

        Ok(MessageResponse::FileDescriptorResponse(
            v1::FileDescriptorResponse {
                file_descriptor_proto: self.file_descriptor_bytes(filename),
            },
        ))
    }

    #[allow(clippy::result_large_err)]
    fn file_containing_extension(
        &self,
        request: &v1::ExtensionRequest,
    ) -> Result<MessageResponse, Status> {
        let numbers = self
            .extensions
            .get(&request.containing_type)
            .ok_or_else(|| {
                Status::new(Code::NotFound).with_message(format!(
                    "no known extensions for type '{}'",
                    request.containing_type
                ))
            })?;
        let filename = numbers.get(&request.extension_number).ok_or_else(|| {
            Status::new(Code::NotFound).with_message(format!(
                "extension number {} for type '{}' not found",
                request.extension_number, request.containing_type
            ))
        })?;

        Ok(MessageResponse::FileDescriptorResponse(
            v1::FileDescriptorResponse {
                file_descriptor_proto: self.file_descriptor_bytes(filename),
            },
        ))
    }

    #[allow(clippy::result_large_err)]
    fn all_extension_numbers_of_type(
        &self,
        containing_type: &str,
    ) -> Result<MessageResponse, Status> {
        let numbers = self.extensions.get(containing_type).ok_or_else(|| {
            Status::new(Code::NotFound)
                .with_message(format!("no known extensions for type '{containing_type}'"))
        })?;
        let mut extension_number: Vec<i32> = numbers.keys().copied().collect();
        extension_number.sort_unstable();

        Ok(MessageResponse::AllExtensionNumbersResponse(
            v1::ExtensionNumberResponse {
                base_type_name: containing_type.to_string(),
                extension_number,
            },
        ))
    }

    fn list_services(&self) -> MessageResponse {
        MessageResponse::ListServicesResponse(v1::ListServiceResponse {
            service: self.service_names.clone(),
        })
    }
}

/// A service that serves reflection using the canonical `grpc.reflection.v1`
/// wire types (the current, non-deprecated protocol).
struct V1ReflectionService {
    state: Arc<State>,
}

impl v1::ServerReflection for V1ReflectionService {
    async fn server_reflection_info(
        &self,
        request: Request<Streaming<v1::ServerReflectionRequest>>,
    ) -> Result<Response<Streaming<v1::ServerReflectionResponse>>, Status> {
        let mut request_stream = request.into_inner();
        let state = self.state.clone();

        Ok(Response::new(Streaming::new(async_stream::try_stream! {
            while let Some(req) = request_stream.next().await.transpose()? {
                let resp = handle_request(&state, &req)?;
                yield v1::ServerReflectionResponse {
                    valid_host: req.host.clone(),
                    original_request: Some(req.clone()),
                    message_response: Some(resp),
                };
            }
        })))
    }
}

/// `grpc.reflection.v1alpha` service. v1 and v1alpha are wire-identical, so a
/// v1alpha request is lifted to v1, processed, and the v1 response is lowered
/// back to v1alpha by re-encoding. v1 is the canonical/internal representation.
struct V1alphaReflectionService {
    state: Arc<State>,
}

impl v1alpha::ServerReflection for V1alphaReflectionService {
    async fn server_reflection_info(
        &self,
        request: Request<Streaming<v1alpha::ServerReflectionRequest>>,
    ) -> Result<Response<Streaming<v1alpha::ServerReflectionResponse>>, Status> {
        let mut request_stream = request.into_inner();
        let state = self.state.clone();

        Ok(Response::new(Streaming::new(async_stream::try_stream! {
            while let Some(req) = request_stream.next().await.transpose()? {
                let v1_req = req_v1alpha_to_v1(&req);
                let resp = handle_request(&state, &v1_req)?;
                let v1_resp = v1::ServerReflectionResponse {
                    valid_host: v1_req.host.clone(),
                    original_request: Some(v1_req.clone()),
                    message_response: Some(resp),
                };
                yield resp_v1_to_v1alpha(v1_resp);
            }
        })))
    }
}

/// Since `grpc.reflection.v1alpha` and `grpc.reflection.v1` are wire-identical,
/// a request/response can be converted losslessly by re-encoding.
fn req_v1alpha_to_v1(req: &v1alpha::ServerReflectionRequest) -> v1::ServerReflectionRequest {
    v1::ServerReflectionRequest::decode(req.encode_to_vec().as_slice())
        .expect("reflection request conversion")
}

fn resp_v1_to_v1alpha(resp: v1::ServerReflectionResponse) -> v1alpha::ServerReflectionResponse {
    v1alpha::ServerReflectionResponse::decode(resp.encode_to_vec().as_slice())
        .expect("reflection response conversion")
}

#[allow(clippy::result_large_err)]
fn handle_request(
    state: &State,
    req: &v1::ServerReflectionRequest,
) -> Result<MessageResponse, Status> {
    match &req.message_request {
        Some(MessageRequest::FileByFilename(filename)) => state.file_by_filename(filename),
        Some(MessageRequest::FileContainingSymbol(symbol)) => state.symbol_by_name(symbol),
        Some(MessageRequest::FileContainingExtension(extension)) => {
            state.file_containing_extension(extension)
        }
        Some(MessageRequest::AllExtensionNumbersOfType(containing_type)) => {
            state.all_extension_numbers_of_type(containing_type)
        }
        Some(MessageRequest::ListServices(_)) => Ok(state.list_services()),
        None => Err(Status::new(Code::InvalidArgument)),
    }
}

/// Composite endpoint returned by [`Reflection::build`], serving both the
/// `grpc.reflection.v1.ServerReflection` and
/// `grpc.reflection.v1alpha.ServerReflection` services from the same state.
struct ReflectionServices(poem::Route);

impl Service for ReflectionServices {
    const NAME: &'static str = "";
}

impl IntoEndpoint for ReflectionServices {
    type Endpoint = BoxEndpoint<'static, poem::Response>;

    fn into_endpoint(self) -> Self::Endpoint {
        use poem::endpoint::EndpointExt;
        self.0.boxed()
    }
}

/// A builder for creating reflection service
#[derive(Debug, Default)]
pub struct Reflection {
    file_descriptor_sets: Vec<FileDescriptorSet>,
    /// Original wire bytes per file name, captured at registration time.
    raw_files: HashMap<String, Vec<u8>>,
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

        // Keep the per-file original bytes so unknown fields survive serving.
        // As in `build_state`, the first registration of a file name wins.
        for bytes in RawFileDescriptorSet::decode(data)
            .expect("valid file descriptor sets")
            .file
        {
            if let Ok(proto) = FileDescriptorProto::decode(bytes.as_slice())
                && let Some(name) = proto.name
            {
                self.raw_files.entry(name).or_insert(bytes);
            }
        }

        self
    }

    /// Build a reflection service
    pub fn build(
        self,
    ) -> impl IntoEndpoint<Endpoint = BoxEndpoint<'static, poem::Response>> + Service {
        let this = self.add_file_descriptor_set(FILE_DESCRIPTOR_SET);
        let state = Arc::new(build_state(this));

        let route = poem::Route::new()
            .nest(
                "/grpc.reflection.v1.ServerReflection",
                v1::ServerReflectionServer::new(V1ReflectionService {
                    state: state.clone(),
                }),
            )
            .nest(
                "/grpc.reflection.v1alpha.ServerReflection",
                v1alpha::ServerReflectionServer::new(V1alphaReflectionService { state }),
            );

        ReflectionServices(route)
    }
}

fn build_state(mut this: Reflection) -> State {
    let fd_iter = std::mem::take(&mut this.file_descriptor_sets)
        .into_iter()
        .flat_map(|fds| fds.file.into_iter());
    let mut files = HashMap::with_capacity(fd_iter.size_hint().0);
    let mut symbols = HashMap::new();
    let mut extensions: HashMap<String, HashMap<i32, String>> = HashMap::new();
    let mut service_names = Vec::new();

    for fd in fd_iter {
        let filename = fd
            .name
            .clone()
            .unwrap_or_else(|| panic!("missing file name"));
        // Sets often share files (e.g. each one built with its imports), and a
        // name identifies one file: the first registration wins, and later
        // copies are neither served nor indexed.
        if files.contains_key(&filename) {
            continue;
        }
        let fd = Arc::new(fd);
        files.insert(filename.clone(), fd.clone());

        let prefix = fd.package.as_deref().unwrap_or_default();

        for message in &fd.message_type {
            process_message(&filename, prefix, message, &mut symbols);
        }

        for enum_ in &fd.enum_type {
            process_enum(&filename, prefix, enum_, &mut symbols);
        }

        for service in &fd.service {
            let service_name = qualified_name(prefix, "service", service.name.as_deref());
            if !service_names.contains(&service_name) {
                service_names.push(service_name.clone());
            }
            symbols.insert(service_name.clone(), filename.clone());

            for method in &service.method {
                let method_name = qualified_name(&service_name, "method", method.name.as_deref());
                symbols.insert(method_name, filename.clone());
            }
        }

        for ext in &fd.extension {
            let ext_name = qualified_name(prefix, "extension", ext.name.as_deref());
            symbols.insert(ext_name, filename.clone());
            index_extension(&filename, ext, &mut extensions);
        }
        for message in &fd.message_type {
            index_nested_extensions(&filename, message, &mut extensions);
        }
    }

    State {
        service_names: service_names
            .into_iter()
            .map(|name| v1::ServiceResponse { name })
            .collect(),
        files,
        raw_files: this.raw_files,
        symbols,
        extensions,
    }
}

fn process_message(
    filename: &str,
    prefix: &str,
    msg: &DescriptorProto,
    symbols: &mut HashMap<String, String>,
) {
    let message_name = qualified_name(prefix, "message", msg.name.as_deref());
    symbols.insert(message_name.clone(), filename.to_string());

    for nested in &msg.nested_type {
        process_message(filename, &message_name, nested, symbols);
    }

    for e in &msg.enum_type {
        process_enum(filename, &message_name, e, symbols);
    }

    for field in &msg.field {
        let field_name = qualified_name(&message_name, "field", field.name.as_deref());
        symbols.insert(field_name, filename.to_string());
    }

    for oneof in &msg.oneof_decl {
        let oneof_name = qualified_name(&message_name, "oneof", oneof.name.as_deref());
        symbols.insert(oneof_name, filename.to_string());
    }

    for ext in &msg.extension {
        let ext_name = qualified_name(&message_name, "extension", ext.name.as_deref());
        symbols.insert(ext_name, filename.to_string());
    }
}

fn process_enum(
    filename: &str,
    prefix: &str,
    e: &EnumDescriptorProto,
    symbols: &mut HashMap<String, String>,
) {
    let enum_name = qualified_name(prefix, "enum", e.name.as_deref());
    symbols.insert(enum_name.clone(), filename.to_string());

    // Enum values are siblings of their enum (C++ scoping): `pkg.VALUE`, not
    // `pkg.Enum.VALUE`.
    for value in &e.value {
        let value_name = qualified_name(prefix, "enum value", value.name.as_deref());
        symbols.insert(value_name, filename.to_string());
    }
}

fn index_extension(
    filename: &str,
    ext: &FieldDescriptorProto,
    extensions: &mut HashMap<String, HashMap<i32, String>>,
) {
    if let (Some(extendee), Some(number)) = (ext.extendee.as_deref(), ext.number) {
        extensions
            .entry(extendee.to_string())
            .or_default()
            .insert(number, filename.to_string());
    }
}

fn index_nested_extensions(
    filename: &str,
    msg: &DescriptorProto,
    extensions: &mut HashMap<String, HashMap<i32, String>>,
) {
    for ext in &msg.extension {
        index_extension(filename, ext, extensions);
    }
    for nested in &msg.nested_type {
        index_nested_extensions(filename, nested, extensions);
    }
}

fn qualified_name(prefix: &str, ty: &str, name: Option<&str>) -> String {
    match name {
        Some(name) if !prefix.is_empty() => format!("{prefix}.{name}"),
        Some(name) => name.to_string(),
        None => panic!("missing {ty} name"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_proto() -> FieldDescriptorProto {
        FieldDescriptorProto {
            name: None,
            number: None,
            label: None,
            r#type: None,
            type_name: None,
            extendee: None,
            default_value: None,
            oneof_index: None,
            json_name: None,
            options: None,
            proto3_optional: None,
        }
    }

    fn msg_proto() -> DescriptorProto {
        DescriptorProto {
            name: None,
            field: Vec::new(),
            extension: Vec::new(),
            nested_type: Vec::new(),
            enum_type: Vec::new(),
            extension_range: Vec::new(),
            oneof_decl: Vec::new(),
            options: None,
            reserved_range: Vec::new(),
            reserved_name: Vec::new(),
        }
    }

    fn file_proto() -> FileDescriptorProto {
        FileDescriptorProto {
            name: None,
            package: None,
            dependency: Vec::new(),
            public_dependency: Vec::new(),
            weak_dependency: Vec::new(),
            message_type: Vec::new(),
            enum_type: Vec::new(),
            service: Vec::new(),
            extension: Vec::new(),
            options: None,
            source_code_info: None,
            syntax: None,
        }
    }

    fn set_bytes(files: Vec<FileDescriptorProto>) -> Vec<u8> {
        FileDescriptorSet { file: files }.encode_to_vec()
    }

    fn reflection_state(files: Vec<FileDescriptorProto>) -> State {
        build_state(Reflection::new().add_file_descriptor_set(&set_bytes(files)))
    }

    #[test]
    fn preserves_unknown_fields_in_raw_descriptor() {
        // A FileDescriptorProto with a known `name` (tag 1) and an extra
        // unknown field (tag 200, varint) that prost does not model.
        let mut inner = FileDescriptorProto {
            name: Some("x.proto".into()),
            ..file_proto()
        }
        .encode_to_vec();
        // Unknown field 200 (varint) = 123456.
        inner.extend_from_slice(&[0xC0, 0x0C, 0xC0, 0xC4, 0x07]);

        // Wrap in a FileDescriptorSet (field 1, length-delimited).
        let mut data = Vec::new();
        data.push(0x0A);
        let mut len = inner.len();
        loop {
            let mut byte = (len & 0x7f) as u8;
            len >>= 7;
            if len != 0 {
                byte |= 0x80;
            }
            data.push(byte);
            if len == 0 {
                break;
            }
        }
        data.extend_from_slice(&inner);

        let state = build_state(Reflection::new().add_file_descriptor_set(&data));
        assert_eq!(state.raw_files.get("x.proto").unwrap(), &inner);
    }

    #[test]
    fn returns_transitive_dependencies() {
        let mut dep = file_proto();
        dep.name = Some("dep.proto".into());
        dep.package = Some("dep".into());
        let mut inner = msg_proto();
        inner.name = Some("Inner".into());
        dep.message_type.push(inner);

        let mut api = file_proto();
        api.name = Some("api.proto".into());
        api.package = Some("api".into());
        api.dependency.push("dep.proto".into());
        let mut outer = msg_proto();
        outer.name = Some("Outer".into());
        let mut name_field = field_proto();
        name_field.name = Some("name".into());
        name_field.number = Some(1);
        outer.field.push(name_field);
        api.message_type.push(outer);

        let state = reflection_state(vec![dep, api]);
        let resp = state.symbol_by_name("api.Outer").unwrap();
        let MessageResponse::FileDescriptorResponse(fd_response) = resp else {
            panic!("expected file descriptor response");
        };
        // api.proto + dep.proto
        assert_eq!(fd_response.file_descriptor_proto.len(), 2);
    }

    #[test]
    fn answers_extension_queries() {
        let mut api = file_proto();
        api.name = Some("api.proto".into());
        api.package = Some("api".into());
        let mut ext = field_proto();
        ext.name = Some("my_field_opt".into());
        ext.number = Some(50001);
        ext.extendee = Some("google.protobuf.FieldOptions".into());
        api.extension.push(ext);

        let state = reflection_state(vec![api]);

        let resp = state
            .file_containing_extension(&v1::ExtensionRequest {
                containing_type: "google.protobuf.FieldOptions".into(),
                extension_number: 50001,
            })
            .unwrap();
        let MessageResponse::FileDescriptorResponse(fd_response) = resp else {
            panic!("expected file descriptor response");
        };
        assert_eq!(fd_response.file_descriptor_proto.len(), 1);

        let resp = state
            .all_extension_numbers_of_type("google.protobuf.FieldOptions")
            .unwrap();
        let MessageResponse::AllExtensionNumbersResponse(numbers) = resp else {
            panic!("expected extension number response");
        };
        assert_eq!(numbers.extension_number, vec![50001]);
    }

    /// The file `symbol_by_name` resolves `symbol` to, if any.
    fn defining_file(state: &State, symbol: &str) -> Option<String> {
        let MessageResponse::FileDescriptorResponse(resp) = state.symbol_by_name(symbol).ok()?
        else {
            panic!("expected file descriptor response");
        };
        let fd = FileDescriptorProto::decode(resp.file_descriptor_proto[0].as_slice()).unwrap();
        Some(fd.name().to_string())
    }

    /// The built-in descriptors, as compiled by protoc.
    fn built_in_state() -> State {
        build_state(Reflection::new().add_file_descriptor_set(FILE_DESCRIPTOR_SET))
    }

    #[test]
    fn indexes_symbols_with_protobuf_scoping() {
        let state = built_in_state();
        let health = Some("grpc/health/v1/health.proto".to_string());
        // Field, and a value of the enum nested in the same message: values
        // are siblings of their enum.
        assert_eq!(
            defining_file(&state, "grpc.health.v1.HealthCheckResponse.status"),
            health
        );
        assert_eq!(
            defining_file(&state, "grpc.health.v1.HealthCheckResponse.SERVING"),
            health
        );
        // Oneof.
        assert_eq!(
            defining_file(
                &state,
                "grpc.reflection.v1.ServerReflectionRequest.message_request"
            ),
            Some("grpc/reflection/v1/reflection.proto".to_string())
        );
        // Names that don't exist under protobuf scoping.
        for missing in [
            "grpc.health.v1.status",
            "grpc.health.v1.HealthCheckResponse.ServingStatus.SERVING",
            "grpc.reflection.v1.message_request",
        ] {
            assert_eq!(defining_file(&state, missing), None, "{missing}");
        }
    }

    #[test]
    fn indexes_top_level_enum_values_and_extensions() {
        let mut api = file_proto();
        api.name = Some("api.proto".into());
        api.package = Some("api".into());
        api.enum_type.push(EnumDescriptorProto {
            name: Some("Color".into()),
            value: vec![prost_types::EnumValueDescriptorProto {
                name: Some("RED".into()),
                ..Default::default()
            }],
            ..Default::default()
        });
        let mut top_ext = field_proto();
        top_ext.name = Some("top_opt".into());
        top_ext.number = Some(50001);
        top_ext.extendee = Some(".google.protobuf.FieldOptions".into());
        api.extension.push(top_ext);
        let mut holder = msg_proto();
        holder.name = Some("Holder".into());
        let mut nested_ext = field_proto();
        nested_ext.name = Some("nested_opt".into());
        nested_ext.number = Some(50002);
        nested_ext.extendee = Some(".google.protobuf.FieldOptions".into());
        holder.extension.push(nested_ext);
        api.message_type.push(holder);

        let state = reflection_state(vec![api]);
        let api = Some("api.proto".to_string());
        assert_eq!(defining_file(&state, "api.RED"), api);
        assert_eq!(defining_file(&state, "api.top_opt"), api);
        assert_eq!(defining_file(&state, "api.Holder.nested_opt"), api);
        assert_eq!(defining_file(&state, "api.Color.RED"), None);
    }

    #[test]
    fn field_named_like_a_message_does_not_shadow_it() {
        let mut settings = file_proto();
        settings.name = Some("settings.proto".into());
        settings.package = Some("api".into());
        let mut msg = msg_proto();
        msg.name = Some("Settings".into());
        settings.message_type.push(msg);

        let mut config = file_proto();
        config.name = Some("config.proto".into());
        config.package = Some("api".into());
        config.dependency = vec!["settings.proto".into()];
        let mut msg = msg_proto();
        msg.name = Some("Config".into());
        let mut field = field_proto();
        field.name = Some("Settings".into());
        msg.field.push(field);
        config.message_type.push(msg);

        let state = reflection_state(vec![settings, config]);
        assert_eq!(
            defining_file(&state, "api.Settings"),
            Some("settings.proto".into())
        );
        assert_eq!(
            defining_file(&state, "api.Config.Settings"),
            Some("config.proto".into())
        );
    }

    #[test]
    fn user_file_named_like_a_built_in_one_is_kept() {
        // A user's own `health.proto`, registered before the built-in set (as
        // `Reflection::build` does).
        let mut health = file_proto();
        health.name = Some("health.proto".into());
        health.package = Some("myapp".into());
        health.service.push(prost_types::ServiceDescriptorProto {
            name: Some("Prober".into()),
            ..Default::default()
        });
        let state = build_state(
            Reflection::new()
                .add_file_descriptor_set(&set_bytes(vec![health]))
                .add_file_descriptor_set(FILE_DESCRIPTOR_SET),
        );
        assert_eq!(
            defining_file(&state, "myapp.Prober"),
            Some("health.proto".into())
        );
        assert_eq!(
            defining_file(&state, "grpc.health.v1.Health"),
            Some("grpc/health/v1/health.proto".into())
        );
    }

    #[test]
    fn files_shared_between_sets_are_registered_once() {
        // Each set carries its imports, so the same file often appears in
        // several; the first copy wins and its services are listed once.
        let mut shared = file_proto();
        shared.name = Some("shared.proto".into());
        shared.package = Some("shared".into());
        shared.service.push(prost_types::ServiceDescriptorProto {
            name: Some("Users".into()),
            ..Default::default()
        });
        let mut changed = shared.clone();
        changed.service[0].name = Some("Renamed".into());

        let state = build_state(
            Reflection::new()
                .add_file_descriptor_set(&set_bytes(vec![shared.clone()]))
                .add_file_descriptor_set(&set_bytes(vec![shared]))
                .add_file_descriptor_set(&set_bytes(vec![changed])),
        );
        let names: Vec<_> = state
            .service_names
            .iter()
            .map(|svc| svc.name.as_str())
            .collect();
        assert_eq!(names, ["shared.Users"]);
        assert_eq!(defining_file(&state, "shared.Renamed"), None);
        let MessageResponse::FileDescriptorResponse(resp) =
            state.file_by_filename("shared.proto").unwrap()
        else {
            panic!("expected file descriptor response");
        };
        let served = FileDescriptorProto::decode(resp.file_descriptor_proto[0].as_slice()).unwrap();
        assert_eq!(served.service[0].name(), "Users");
    }
}
