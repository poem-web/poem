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

/// Names of the files already served on one reflection stream.
type SentFiles = HashSet<String>;

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
    ///
    /// Dependencies already in `sent` (served earlier on the same reflection
    /// stream) are skipped, as the reflection protocol allows; `filename`
    /// itself is always included since it was asked for. Everything served is
    /// recorded in `sent`.
    fn file_descriptor_bytes(&self, filename: &str, sent: &mut SentFiles) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut queue = vec![filename.to_string()];

        while let Some(name) = queue.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }

            if name == filename || !sent.contains(&name) {
                let Some(bytes) = self.encoded_file(&name) else {
                    continue;
                };
                out.push(bytes);
                sent.insert(name.clone());
            }

            // Walk through already-sent files too: cheap, and doesn't rely on
            // their dependencies having been sent with them.
            if let Some(fd) = self.files.get(&name) {
                for dep in &fd.dependency {
                    queue.push(dep.clone());
                }
            }
        }

        out
    }

    fn encoded_file(&self, name: &str) -> Option<Vec<u8>> {
        self.raw_files
            .get(name)
            .cloned()
            .or_else(|| self.files.get(name).map(|fd| fd.encode_to_vec()))
    }

    fn file_by_filename(
        &self,
        filename: &str,
        sent: &mut SentFiles,
    ) -> Result<MessageResponse, Status> {
        if !self.files.contains_key(filename) && !self.raw_files.contains_key(filename) {
            return Err(
                Status::new(Code::NotFound).with_message(format!("file '{filename}' not found"))
            );
        }

        Ok(MessageResponse::FileDescriptorResponse(
            v1::FileDescriptorResponse {
                file_descriptor_proto: self.file_descriptor_bytes(filename, sent),
            },
        ))
    }

    fn symbol_by_name(
        &self,
        symbol: &str,
        sent: &mut SentFiles,
    ) -> Result<MessageResponse, Status> {
        let filename = self.symbols.get(symbol).ok_or_else(|| {
            Status::new(Code::NotFound).with_message(format!("symbol '{symbol}' not found"))
        })?;

        Ok(MessageResponse::FileDescriptorResponse(
            v1::FileDescriptorResponse {
                file_descriptor_proto: self.file_descriptor_bytes(filename, sent),
            },
        ))
    }

    fn file_containing_extension(
        &self,
        request: &v1::ExtensionRequest,
        sent: &mut SentFiles,
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
                file_descriptor_proto: self.file_descriptor_bytes(filename, sent),
            },
        ))
    }

    fn all_extension_numbers_of_type(
        &self,
        containing_type: &str,
    ) -> Result<MessageResponse, Status> {
        // A known type without extensions gets an empty list; only an unknown
        // type is an error.
        let mut extension_number: Vec<i32> = match self.extensions.get(containing_type) {
            Some(numbers) => numbers.keys().copied().collect(),
            None if self.symbols.contains_key(containing_type) => Vec::new(),
            None => {
                return Err(Status::new(Code::NotFound)
                    .with_message(format!("type '{containing_type}' not found")));
            }
        };
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
            let mut sent = SentFiles::new();
            while let Some(req) = request_stream.next().await.transpose()? {
                let resp = handle_request(&state, &req, &mut sent)?;
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
            let mut sent = SentFiles::new();
            while let Some(req) = request_stream.next().await.transpose()? {
                let v1_req = req_v1alpha_to_v1(&req);
                let resp = handle_request(&state, &v1_req, &mut sent)?;
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

/// Answers one request on a reflection stream. A failed lookup is reported
/// in-band as an `ErrorResponse` so the stream stays usable, while a request
/// without a `message_request` ends the stream with `InvalidArgument`.
fn handle_request(
    state: &State,
    req: &v1::ServerReflectionRequest,
    sent: &mut SentFiles,
) -> Result<MessageResponse, Status> {
    let result = match &req.message_request {
        Some(MessageRequest::FileByFilename(filename)) => state.file_by_filename(filename, sent),
        Some(MessageRequest::FileContainingSymbol(symbol)) => state.symbol_by_name(symbol, sent),
        Some(MessageRequest::FileContainingExtension(extension)) => {
            state.file_containing_extension(extension, sent)
        }
        Some(MessageRequest::AllExtensionNumbersOfType(containing_type)) => {
            state.all_extension_numbers_of_type(containing_type)
        }
        Some(MessageRequest::ListServices(_)) => Ok(state.list_services()),
        None => {
            return Err(Status::new(Code::InvalidArgument).with_message("empty reflection request"));
        }
    };
    Ok(result.unwrap_or_else(|status| {
        MessageResponse::ErrorResponse(v1::ErrorResponse {
            error_code: status.code().as_u16().into(),
            error_message: status.message().unwrap_or_default().to_string(),
        })
    }))
}

/// Composite endpoint returned by [`Reflection::build`], serving both the
/// `grpc.reflection.v1.ServerReflection` and
/// `grpc.reflection.v1alpha.ServerReflection` services from the same state.
struct ReflectionServices(poem::Route);

impl Service for ReflectionServices {
    const NAME: &'static str = "";
    const METHODS: &'static [(&'static str, crate::MethodType)] = &[
        <v1::ServerReflectionServer<V1ReflectionService> as Service>::METHODS[0],
        <v1alpha::ServerReflectionServer<V1alphaReflectionService> as Service>::METHODS[0],
    ];
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
    /// Allow-list set by [`Reflection::advertise_services`], in order.
    service_names: Option<Vec<String>>,
    /// `Service::NAME`s mounted next to reflection, set by
    /// [`RouteGrpc::with_reflection`](crate::RouteGrpc::with_reflection).
    mounted: Option<Vec<String>>,
}

impl Reflection {
    /// Fully-qualified names of the services [`build`](Self::build) mounts:
    /// `grpc.reflection.v1.ServerReflection` and
    /// `grpc.reflection.v1alpha.ServerReflection`.
    ///
    /// Include them in [`advertise_services`](Self::advertise_services) to
    /// keep reflection itself listed.
    pub const SERVICE_NAMES: &'static [&'static str] = &[
        <v1::ServerReflectionServer<V1ReflectionService> as Service>::NAME,
        <v1alpha::ServerReflectionServer<V1alphaReflectionService> as Service>::NAME,
    ];

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

    /// Advertise only these fully-qualified service names (e.g.
    /// `helloworld.Greeter`) in `ListServices`, in the given order.
    ///
    /// Without this, `ListServices` advertises every available service: those
    /// defined in the registered file descriptor sets or, with
    /// [`RouteGrpc::with_reflection`](crate::RouteGrpc::with_reflection),
    /// those mounted on the router. This narrows that list down to the given
    /// names; a name that is not available is skipped. Calling it again
    /// replaces the previous list.
    ///
    /// **The reflection service itself is no longer listed unless it is named
    /// too**, via [`Reflection::SERVICE_NAMES`].
    ///
    /// This only affects `ListServices`: symbol and file lookups still resolve
    /// against all registered descriptors.
    ///
    /// ```
    /// use poem_grpc::Reflection;
    /// # // An empty set; a real one comes from `poem_grpc_build`.
    /// # const FILE_DESCRIPTOR_SET: &[u8] = &[];
    ///
    /// // Lists only `helloworld.Greeter` (as long as the registered
    /// // descriptors define it), without the reflection service.
    /// let reflection = Reflection::new()
    ///     .add_file_descriptor_set(FILE_DESCRIPTOR_SET)
    ///     .advertise_services(["helloworld.Greeter"]);
    ///
    /// // Lists `helloworld.Greeter` and the reflection service.
    /// let reflection = Reflection::new()
    ///     .add_file_descriptor_set(FILE_DESCRIPTOR_SET)
    ///     .advertise_services(
    ///         ["helloworld.Greeter"]
    ///             .into_iter()
    ///             .chain(Reflection::SERVICE_NAMES.iter().copied()),
    ///     );
    /// ```
    pub fn advertise_services<I>(mut self, names: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let mut advertised: Vec<String> = Vec::new();
        for name in names {
            let name = name.into();
            if !advertised.contains(&name) {
                advertised.push(name);
            }
        }
        self.service_names = Some(advertised);
        self
    }

    /// Restricts the available services to `mounted` plus reflection itself.
    pub(crate) fn mounted_on_router<'a>(
        mut self,
        mounted: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        self.mounted = Some(
            mounted
                .into_iter()
                .chain(Self::SERVICE_NAMES.iter().copied())
                .map(str::to_owned)
                .collect(),
        );
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
                format!(
                    "/{}",
                    <v1::ServerReflectionServer<V1ReflectionService> as Service>::NAME
                ),
                v1::ServerReflectionServer::new(V1ReflectionService {
                    state: state.clone(),
                }),
            )
            .nest(
                format!(
                    "/{}",
                    <v1alpha::ServerReflectionServer<V1alphaReflectionService> as Service>::NAME
                ),
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

    // Only services the descriptors define can be described, so a mounted
    // service without a registered descriptor (or mounted under a name that
    // isn't fully qualified) is not available.
    let available = match this.mounted {
        Some(mounted) => {
            let mut available: Vec<String> = Vec::new();
            for name in mounted {
                if service_names.contains(&name) && !available.contains(&name) {
                    available.push(name);
                }
            }
            available
        }
        None => service_names,
    };
    let advertised = match this.service_names {
        Some(allowed) => allowed
            .into_iter()
            .filter(|name| available.contains(name))
            .collect(),
        None => available,
    };

    State {
        service_names: advertised
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
        // protoc writes fully-qualified type references with a leading dot
        // (`.google.protobuf.FieldOptions`); requests name the type without it.
        let extendee = extendee.strip_prefix('.').unwrap_or(extendee);
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

    #[test]
    fn method_table_matches_generated_services() {
        let generated = [
            <v1::ServerReflectionServer<V1ReflectionService> as Service>::METHODS,
            <v1alpha::ServerReflectionServer<V1alphaReflectionService> as Service>::METHODS,
        ]
        .concat();
        assert_eq!(ReflectionServices::METHODS, generated.as_slice());
    }

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
        let resp = state
            .symbol_by_name("api.Outer", &mut SentFiles::new())
            .unwrap();
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
        // As written by protoc: fully qualified with a leading dot.
        ext.extendee = Some(".google.protobuf.FieldOptions".into());
        api.extension.push(ext);

        let state = reflection_state(vec![api]);

        let resp = state
            .file_containing_extension(
                &v1::ExtensionRequest {
                    containing_type: "google.protobuf.FieldOptions".into(),
                    extension_number: 50001,
                },
                &mut SentFiles::new(),
            )
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
        let MessageResponse::FileDescriptorResponse(resp) =
            state.symbol_by_name(symbol, &mut SentFiles::new()).ok()?
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
        let MessageResponse::FileDescriptorResponse(resp) = state
            .file_by_filename("shared.proto", &mut SentFiles::new())
            .unwrap()
        else {
            panic!("expected file descriptor response");
        };
        let served = FileDescriptorProto::decode(resp.file_descriptor_proto[0].as_slice()).unwrap();
        assert_eq!(served.service[0].name(), "Users");
    }

    /// `dep.proto` <- `a.proto` (a.A) and `dep.proto` <- `b.proto` (b.B).
    /// `a.proto` also extends `dep.Shared` with field 100.
    fn shared_dependency_files() -> Vec<FileDescriptorProto> {
        let file = |name: &str, package: &str, deps: &[&str], msg: &str| {
            let mut fd = file_proto();
            fd.name = Some(name.into());
            fd.package = Some(package.into());
            fd.dependency = deps.iter().map(|dep| dep.to_string()).collect();
            let mut message = msg_proto();
            message.name = Some(msg.into());
            fd.message_type.push(message);
            fd
        };
        let mut a = file("a.proto", "a", &["dep.proto"], "A");
        let mut ext = field_proto();
        ext.name = Some("a_ext".into());
        ext.number = Some(100);
        ext.extendee = Some(".dep.Shared".into());
        a.extension.push(ext);
        vec![
            file("dep.proto", "dep", &[], "Shared"),
            a,
            file("b.proto", "b", &["dep.proto"], "B"),
        ]
    }

    /// File names in a response: the requested file first (clients treat it as
    /// the answer), then its dependencies in sorted order.
    fn served_names(resp: MessageResponse) -> Vec<String> {
        let MessageResponse::FileDescriptorResponse(resp) = resp else {
            panic!("expected file descriptor response");
        };
        let mut names: Vec<_> = resp
            .file_descriptor_proto
            .iter()
            .map(|bytes| {
                FileDescriptorProto::decode(bytes.as_slice())
                    .unwrap()
                    .name()
                    .to_string()
            })
            .collect();
        names[1..].sort();
        names
    }

    #[test]
    fn skips_dependencies_already_sent_on_the_stream() {
        let state = reflection_state(shared_dependency_files());
        let mut sent = SentFiles::new();

        let first = state.symbol_by_name("a.A", &mut sent).unwrap();
        assert_eq!(served_names(first), ["a.proto", "dep.proto"]);
        let second = state.symbol_by_name("b.B", &mut sent).unwrap();
        assert_eq!(
            served_names(second),
            ["b.proto"],
            "dep.proto was already sent"
        );
    }

    #[test]
    fn requested_file_is_always_sent() {
        let state = reflection_state(shared_dependency_files());
        let mut sent = SentFiles::new();

        state.symbol_by_name("a.A", &mut sent).unwrap();
        // Asked for explicitly, so served again even though it was sent.
        let again = state.file_by_filename("dep.proto", &mut sent).unwrap();
        assert_eq!(served_names(again), ["dep.proto"]);
        let again = state.file_by_filename("a.proto", &mut sent).unwrap();
        assert_eq!(served_names(again), ["a.proto"]);
    }

    #[derive(Debug, Clone, Copy)]
    enum Version {
        V1,
        V1alpha,
    }

    /// Opens one reflection stream per entry of `streams`, in turn, all on the
    /// same server, and returns each stream's responses (lifted to v1).
    async fn responses_per_stream(
        version: Version,
        streams: Vec<Vec<MessageRequest>>,
    ) -> Vec<Vec<MessageResponse>> {
        let route = crate::RouteGrpc::new().add_service(
            Reflection::new()
                .add_file_descriptor_set(&set_bytes(shared_dependency_files()))
                .build(),
        );
        let cli = crate::client::GrpcClient::from_endpoint(route);

        let mut out = Vec::new();
        for requests in streams {
            let requests = requests
                .into_iter()
                .map(|request| v1::ServerReflectionRequest {
                    host: String::new(),
                    message_request: Some(request),
                });
            let responses: Vec<v1::ServerReflectionResponse> = match version {
                Version::V1 => {
                    let requests = Streaming::new(futures_util::stream::iter(requests.map(Ok)));
                    cli.bidirectional_streaming(
                        "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
                        crate::codec::ProstCodec::<
                            v1::ServerReflectionRequest,
                            v1::ServerReflectionResponse,
                        >::default(),
                        Request::new(requests),
                    )
                    .await
                    .unwrap()
                    .into_inner()
                    .map(Result::unwrap)
                    .collect()
                    .await
                }
                Version::V1alpha => {
                    let requests =
                        requests.map(|req| {
                            Ok(v1alpha::ServerReflectionRequest::decode(
                                req.encode_to_vec().as_slice(),
                            )
                            .unwrap())
                        });
                    cli.bidirectional_streaming(
                        "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
                        crate::codec::ProstCodec::<
                            v1alpha::ServerReflectionRequest,
                            v1alpha::ServerReflectionResponse,
                        >::default(),
                        Request::new(Streaming::new(futures_util::stream::iter(requests))),
                    )
                    .await
                    .unwrap()
                    .into_inner()
                    .map(|resp| {
                        v1::ServerReflectionResponse::decode(
                            resp.unwrap().encode_to_vec().as_slice(),
                        )
                        .unwrap()
                    })
                    .collect()
                    .await
                }
            };
            out.push(
                responses
                    .into_iter()
                    .map(|resp| resp.message_response.unwrap())
                    .collect(),
            );
        }
        out
    }

    fn symbol(name: &str) -> MessageRequest {
        MessageRequest::FileContainingSymbol(name.into())
    }

    #[tokio::test]
    async fn dedup_is_scoped_to_one_stream() {
        for version in [Version::V1, Version::V1alpha] {
            let streams = responses_per_stream(
                version,
                vec![vec![symbol("a.A"), symbol("b.B")], vec![symbol("b.B")]],
            )
            .await;
            let served: Vec<Vec<Vec<String>>> = streams
                .into_iter()
                .map(|responses| responses.into_iter().map(served_names).collect())
                .collect();
            assert_eq!(
                served,
                [
                    // Same stream: dep.proto is sent once.
                    vec![vec!["a.proto", "dep.proto"], vec!["b.proto"]],
                    // A new stream on the same server starts from scratch.
                    vec![vec!["b.proto", "dep.proto"]],
                ],
                "{version:?}"
            );
        }
    }

    #[tokio::test]
    async fn every_request_kind_is_answered_on_one_stream() {
        for version in [Version::V1, Version::V1alpha] {
            let [responses] = <[_; 1]>::try_from(
                responses_per_stream(
                    version,
                    vec![vec![
                        MessageRequest::FileByFilename("b.proto".into()),
                        MessageRequest::FileContainingExtension(v1::ExtensionRequest {
                            containing_type: "dep.Shared".into(),
                            extension_number: 100,
                        }),
                        MessageRequest::AllExtensionNumbersOfType("dep.Shared".into()),
                        MessageRequest::FileByFilename("missing.proto".into()),
                        MessageRequest::ListServices(String::new()),
                    ]],
                )
                .await,
            )
            .unwrap();
            let [by_name, by_ext, numbers, missing, list] = <[_; 5]>::try_from(responses).unwrap();

            assert_eq!(
                served_names(by_name),
                ["b.proto", "dep.proto"],
                "{version:?}"
            );
            // dep.proto was already sent with b.proto.
            assert_eq!(served_names(by_ext), ["a.proto"], "{version:?}");
            let MessageResponse::AllExtensionNumbersResponse(numbers) = numbers else {
                panic!("{version:?}: expected extension numbers response");
            };
            assert_eq!(numbers.extension_number, [100]);
            let MessageResponse::ErrorResponse(err) = missing else {
                panic!("{version:?}: expected error response");
            };
            assert_eq!(err.error_code, i32::from(Code::NotFound.as_u16()));
            // Still answered after the error.
            assert!(
                matches!(list, MessageResponse::ListServicesResponse(_)),
                "{version:?}"
            );
        }
    }

    #[test]
    fn unregistered_dependency_is_skipped() {
        // A set built without `--include_imports` lacks the imported files.
        let mut api = file_proto();
        api.name = Some("api.proto".into());
        api.dependency = vec!["absent.proto".into()];
        let state = reflection_state(vec![api]);
        let resp = state
            .file_by_filename("api.proto", &mut SentFiles::new())
            .unwrap();
        assert_eq!(served_names(resp), ["api.proto"]);
    }

    #[tokio::test]
    async fn errors_are_reported_in_band_without_ending_the_stream() {
        for version in [Version::V1, Version::V1alpha] {
            let [responses] = <[_; 1]>::try_from(
                responses_per_stream(
                    version,
                    vec![vec![symbol("missing.Missing"), symbol("a.A")]],
                )
                .await,
            )
            .unwrap();
            let [first, second] = <[_; 2]>::try_from(responses).unwrap();
            let MessageResponse::ErrorResponse(err) = first else {
                panic!("{version:?}: expected error response");
            };
            assert_eq!(err.error_code, i32::from(Code::NotFound.as_u16()));
            assert_eq!(err.error_message, "symbol 'missing.Missing' not found");
            assert_eq!(
                served_names(second),
                ["a.proto", "dep.proto"],
                "{version:?}"
            );
        }
    }

    #[test]
    fn empty_request_is_rejected() {
        let state = reflection_state(shared_dependency_files());
        let req = v1::ServerReflectionRequest {
            host: String::new(),
            message_request: None,
        };
        let err = handle_request(&state, &req, &mut SentFiles::new()).unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    #[test]
    fn extension_numbers_of_message_without_extensions_is_empty() {
        let state = reflection_state(shared_dependency_files());
        let MessageResponse::AllExtensionNumbersResponse(numbers) =
            state.all_extension_numbers_of_type("a.A").unwrap()
        else {
            panic!("expected extension numbers response");
        };
        assert_eq!(numbers.base_type_name, "a.A");
        assert!(numbers.extension_number.is_empty());

        let err = state
            .all_extension_numbers_of_type("a.Missing")
            .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    fn service(name: &str) -> prost_types::ServiceDescriptorProto {
        prost_types::ServiceDescriptorProto {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    /// `api.proto` (api.Api, api.Hidden) and `other.proto` (other.Other).
    fn two_files() -> Vec<FileDescriptorProto> {
        let mut api = file_proto();
        api.name = Some("api.proto".into());
        api.package = Some("api".into());
        api.service.push(service("Api"));
        api.service.push(service("Hidden"));
        let mut other = file_proto();
        other.name = Some("other.proto".into());
        other.package = Some("other".into());
        other.service.push(service("Other"));
        vec![api, other]
    }

    fn listed(state: &State) -> Vec<String> {
        let MessageResponse::ListServicesResponse(list) = state.list_services() else {
            panic!("expected list services response");
        };
        list.service.into_iter().map(|svc| svc.name).collect()
    }

    #[test]
    fn advertises_every_service_by_default() {
        let state = reflection_state(two_files());
        assert_eq!(listed(&state), ["api.Api", "api.Hidden", "other.Other"]);
    }

    #[test]
    fn advertise_services_only_affects_list_services() {
        let state = build_state(
            Reflection::new()
                .add_file_descriptor_set(&set_bytes(two_files()))
                .advertise_services(["api.Api"]),
        );
        assert_eq!(listed(&state), ["api.Api"]);

        // Unadvertised services and their files still resolve, and the served
        // bytes are untouched.
        assert!(
            state
                .symbol_by_name("api.Hidden", &mut SentFiles::new())
                .is_ok()
        );
        assert!(
            state
                .symbol_by_name("other.Other", &mut SentFiles::new())
                .is_ok()
        );
        assert!(
            state
                .file_by_filename("other.proto", &mut SentFiles::new())
                .is_ok()
        );
        let MessageResponse::FileDescriptorResponse(resp) = state
            .file_by_filename("api.proto", &mut SentFiles::new())
            .unwrap()
        else {
            panic!("expected file descriptor response");
        };
        let api = FileDescriptorProto::decode(resp.file_descriptor_proto[0].as_slice()).unwrap();
        assert_eq!(api.service.len(), 2);
    }

    #[test]
    fn advertise_services_keeps_given_order_and_skips_unknown_names() {
        let state = build_state(
            Reflection::new()
                .add_file_descriptor_set(&set_bytes(two_files()))
                .advertise_services(["other.Other", "missing.Service", "api.Api", "other.Other"]),
        );
        assert_eq!(listed(&state), ["other.Other", "api.Api"]);
    }

    #[test]
    fn advertise_services_replaces_previous_list() {
        let state = build_state(
            Reflection::new()
                .add_file_descriptor_set(&set_bytes(two_files()))
                .advertise_services(["api.Api"])
                .advertise_services(["other.Other"]),
        );
        assert_eq!(listed(&state), ["other.Other"]);
    }

    /// Like `Reflection::build`, which also registers the built-in
    /// descriptors (reflection and health).
    fn built_state(reflection: Reflection) -> State {
        build_state(reflection.add_file_descriptor_set(FILE_DESCRIPTOR_SET))
    }

    #[test]
    fn mounted_services_narrowed_by_advertise_services() {
        // Reflection itself is not advertised unless it was named too.
        let state = built_state(
            Reflection::new()
                .add_file_descriptor_set(&set_bytes(two_files()))
                .advertise_services(["api.Hidden", "api.Api"])
                .mounted_on_router(["api.Api", "other.Other"]),
        );
        assert_eq!(listed(&state), ["api.Api"]);
    }

    #[test]
    fn mounted_with_nothing_registered_lists_only_reflection() {
        let state = built_state(
            Reflection::new()
                .add_file_descriptor_set(&set_bytes(two_files()))
                .mounted_on_router([]),
        );
        assert_eq!(listed(&state), Reflection::SERVICE_NAMES);
    }

    #[test]
    fn mounted_services_without_descriptors_are_not_listed() {
        // `internal.Admin` has no registered descriptor, and `Api` is how
        // `api.Api` is named when generated with `disable_package_emission`:
        // neither could be described, so neither is listed.
        let state = built_state(
            Reflection::new()
                .add_file_descriptor_set(&set_bytes(two_files()))
                .mounted_on_router(["api.Api", "internal.Admin", "Api"]),
        );
        let mut expected = vec!["api.Api"];
        expected.extend(Reflection::SERVICE_NAMES);
        assert_eq!(listed(&state), expected);
    }

    type ReflectionCodec =
        crate::codec::ProstCodec<v1::ServerReflectionRequest, v1::ServerReflectionResponse>;

    async fn ask_route(
        route: crate::RouteGrpc,
        request: MessageRequest,
    ) -> Result<MessageResponse, Status> {
        let cli = crate::client::GrpcClient::from_endpoint(route);
        let req = v1::ServerReflectionRequest {
            host: String::new(),
            message_request: Some(request),
        };
        let stream = Streaming::new(futures_util::stream::iter([Ok(req)]));
        let resp = cli
            .bidirectional_streaming(
                "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
                ReflectionCodec::default(),
                Request::new(stream),
            )
            .await?;
        let msg = resp.into_inner().next().await.unwrap()?;
        Ok(msg.message_response.unwrap())
    }

    async fn list_via_route_in_order(route: crate::RouteGrpc) -> Vec<String> {
        let resp = ask_route(route, MessageRequest::ListServices(String::new()))
            .await
            .unwrap();
        let MessageResponse::ListServicesResponse(list) = resp else {
            panic!("expected list services response");
        };
        list.service.into_iter().map(|svc| svc.name).collect()
    }

    async fn list_via_route(route: crate::RouteGrpc) -> Vec<String> {
        let mut names = list_via_route_in_order(route).await;
        names.sort();
        names
    }

    /// A mounted service named `NAME` that no registered descriptor defines.
    struct Undescribed<const V1: bool>;

    impl Service for Undescribed<false> {
        const NAME: &'static str = "internal.Admin";
    }

    impl Service for Undescribed<true> {
        const NAME: &'static str = "grpc.reflection.v1.ServerReflection";
    }

    impl<const V1: bool> IntoEndpoint for Undescribed<V1> {
        type Endpoint = BoxEndpoint<'static, poem::Response>;

        fn into_endpoint(self) -> Self::Endpoint {
            use poem::EndpointExt;
            poem::endpoint::make_sync(|_| poem::Response::default()).boxed()
        }
    }

    fn plus_reflection_names(mut names: Vec<&str>) -> Vec<String> {
        names.extend(Reflection::SERVICE_NAMES);
        let mut names: Vec<_> = names.into_iter().map(str::to_string).collect();
        names.sort();
        names
    }

    const HEALTH: &str = "grpc.health.v1.Health";

    #[tokio::test]
    async fn route_reflection_lists_only_registered_services() {
        // `grpc.health.v1.Health` is in the built-in descriptors either way;
        // it is only listed once it is actually mounted.
        let route = crate::RouteGrpc::new().with_reflection(Reflection::new());
        assert_eq!(list_via_route(route).await, plus_reflection_names(vec![]));

        let route = crate::RouteGrpc::new()
            .add_service(crate::health_service().0)
            .with_reflection(Reflection::new());
        assert_eq!(
            list_via_route(route).await,
            plus_reflection_names(vec![HEALTH])
        );
    }

    #[tokio::test]
    async fn route_reflection_skips_mounted_services_without_descriptors() {
        let route = crate::RouteGrpc::new()
            .add_service(Undescribed::<false>)
            .add_service(crate::health_service().0)
            .with_reflection(Reflection::new());
        assert_eq!(
            list_via_route(route).await,
            plus_reflection_names(vec![HEALTH])
        );
    }

    #[tokio::test]
    async fn route_reflection_follows_advertise_services_order() {
        let &[v1, v1alpha] = Reflection::SERVICE_NAMES else {
            unreachable!()
        };
        let route = crate::RouteGrpc::new()
            .add_service(crate::health_service().0)
            .with_reflection(Reflection::new().advertise_services([v1alpha, HEALTH, v1]));
        assert_eq!(list_via_route_in_order(route).await, [v1alpha, HEALTH, v1]);
    }

    #[tokio::test]
    async fn route_reflection_still_resolves_unlisted_symbols() {
        let route = crate::RouteGrpc::new().with_reflection(Reflection::new());
        let resp = ask_route(route, MessageRequest::FileContainingSymbol(HEALTH.into())).await;
        assert!(matches!(
            resp,
            Ok(MessageResponse::FileDescriptorResponse(_))
        ));
    }

    #[tokio::test]
    async fn route_reflection_is_order_independent() {
        let route = crate::RouteGrpc::new()
            .with_reflection(Reflection::new())
            .add_service(crate::health_service().0);
        assert_eq!(
            list_via_route(route).await,
            plus_reflection_names(vec![HEALTH])
        );
    }

    #[tokio::test]
    async fn add_service_reflection_keeps_listing_everything() {
        // Existing usage is unchanged without `with_reflection`.
        let route = crate::RouteGrpc::new().add_service(Reflection::new().build());
        assert_eq!(
            list_via_route(route).await,
            plus_reflection_names(vec![HEALTH])
        );
    }

    #[tokio::test]
    async fn route_reflection_lists_itself_only_if_named() {
        // With `advertise_services`, only named services that are mounted are
        // advertised, reflection included.
        let route = crate::RouteGrpc::new()
            .add_service(crate::health_service().0)
            .with_reflection(Reflection::new().advertise_services([HEALTH]));
        assert_eq!(list_via_route(route).await, [HEALTH]);

        let reflection = Reflection::new().advertise_services(
            [HEALTH]
                .into_iter()
                .chain(Reflection::SERVICE_NAMES.iter().copied()),
        );
        let route = crate::RouteGrpc::new()
            .add_service(crate::health_service().0)
            .with_reflection(reflection);
        assert_eq!(
            list_via_route(route).await,
            plus_reflection_names(vec![HEALTH])
        );
    }

    #[test]
    #[should_panic(expected = "a service with an empty `Service::NAME`")]
    fn reflection_added_both_ways_panics() {
        let route = crate::RouteGrpc::new()
            .add_service(Reflection::new().build())
            .with_reflection(Reflection::new());
        let _ = route.into_endpoint();
    }

    #[test]
    #[should_panic(
        expected = "`grpc.reflection.v1.ServerReflection` is also added via `add_service`"
    )]
    fn service_named_like_reflection_panics() {
        let route = crate::RouteGrpc::new()
            .add_service(Undescribed::<true>)
            .with_reflection(Reflection::new());
        let _ = route.into_endpoint();
    }
}
