use crate::{
    IntoResponse, Response,
    http::StatusCode,
};

/// An engine-agnostic template to be rendered.
///
/// This response type requires a templating engine middleware
/// to work correctly. Missing the middleware will return
/// `500 Internal Server Error`.
///
/// ```
/// use poem::{
///     templates::Template,
///     ctx, handler,
///     web::Path,
/// };
///
/// #[handler]
/// fn hello(Path(name): Path<String>) -> Template {
///     Template::render("index.html.tera", &ctx! { "name": &name })
/// }
/// ```
pub struct Template<C> {
    /// Path to the template.
    pub name: String,
    /// Template context. This is used
    /// by engines for additional data.
    pub context: C,
}

impl<C> Template<C> {
    /// Renders the template.
    pub fn render(name: impl Into<String>, context: C) -> Self {
        Self {
            name: name.into(),
            context,
        }
    }
}

impl<C: Send + Sync + 'static> IntoResponse for Template<C> {
    fn into_response(self) -> Response {
        // At this stage, we respond with an internal server error,
        // as we have not yet built the template.
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)

            // We add this as an extension so that it can be
            // accessed by the endpoint to actually render
            // the template.
            .extension(self)

            .finish()
    }
}