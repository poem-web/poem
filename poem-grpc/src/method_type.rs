/// The type of a GRPC method, as declared in the proto definition.
///
/// It is inserted into the request extensions, so middlewares and handlers
/// can tell streaming calls from unary ones. It is visible to:
///
/// - client middlewares of generated clients (`client_middleware` in the build
///   script, or `with` on the client);
/// - middlewares applied with
///   [`RouteGrpc::middleware`](crate::RouteGrpc::middleware);
/// - `server_middleware`s of generated servers in the build script;
/// - handlers, through [`Request::extensions`](crate::Request::extensions).
///
/// Middlewares wrapping the whole `RouteGrpc` (e.g. with `EndpointExt::with`)
/// do not see it. On the server side it is looked up by request path from
/// [`Service::METHODS`](crate::Service::METHODS) when entering each
/// `RouteGrpc` middleware and the generated server, so:
///
/// - requests to an unknown method carry no `MethodType`;
/// - a `server_middleware` rewriting the request path does not change the
///   `MethodType` seen by the `server_middleware`s inside it.
///
/// Extensions do not cross the network, and an in-process client
/// (`from_endpoint`) removes it before calling the server as well.
///
/// # Example
///
/// A timeout wrapped around `Endpoint::call` only covers the time until the
/// response headers are returned, which for client streaming calls includes
/// receiving the whole request stream:
///
/// ```
/// use poem::Request;
/// use poem_grpc::MethodType;
///
/// fn should_apply_timeout(req: &Request) -> bool {
///     !req.extensions()
///         .get::<MethodType>()
///         .is_some_and(MethodType::is_client_streaming)
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodType {
    /// One request message, one response message.
    Unary,
    /// A stream of request messages, one response message.
    ClientStreaming,
    /// One request message, a stream of response messages.
    ServerStreaming,
    /// A stream of request messages and a stream of response messages.
    BidirectionalStreaming,
}

impl MethodType {
    /// Returns `true` if the client sends a stream of messages.
    #[inline]
    pub fn is_client_streaming(&self) -> bool {
        matches!(self, Self::ClientStreaming | Self::BidirectionalStreaming)
    }

    /// Returns `true` if the server sends a stream of messages.
    #[inline]
    pub fn is_server_streaming(&self) -> bool {
        matches!(self, Self::ServerStreaming | Self::BidirectionalStreaming)
    }

    /// Returns `true` if either side sends a stream of messages.
    #[inline]
    pub fn is_streaming(&self) -> bool {
        !matches!(self, Self::Unary)
    }

    /// Returns the name of this method type, e.g. `"server_streaming"`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unary => "unary",
            Self::ClientStreaming => "client_streaming",
            Self::ServerStreaming => "server_streaming",
            Self::BidirectionalStreaming => "bidirectional_streaming",
        }
    }
}

/// Replaces the [`MethodType`] in the request extensions with `method_type`,
/// so a stale value (e.g. one inserted by an in-process client) never leaks
/// through.
pub(crate) fn set_method_type(req: &mut poem::Request, method_type: Option<MethodType>) {
    let extensions = req.extensions_mut();
    match (method_type, extensions.get_mut::<MethodType>()) {
        (Some(method_type), Some(current)) => *current = method_type,
        (Some(method_type), None) => {
            let _ = extensions.insert(method_type);
        }
        (None, _) => {
            let _ = extensions.remove::<MethodType>();
        }
    }
}
