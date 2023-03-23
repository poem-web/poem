use tera::Tera;

use crate::{
    templates::Template,
    error::{InternalServerError, IntoResult},
    web::Html,
    Endpoint, Middleware, Request, Result,
    Response, IntoResponse,
};

/// Tera template with context.
pub type TeraTemplate = Template<tera::Context>;

/// Tera templates middleware.
pub struct TeraEngine {
    tera: Tera,
}

impl TeraEngine {
    /// Create a new instance of `TeraEngine`, containing all the parsed
    /// templates found in the glob The errors are already handled.
    ///
    /// ```no_run
    /// use poem::templates::tera::TeraEngine;
    ///
    /// let tera = TeraEngine::from_glob("templates/**/*")
    ///     .expect("Failed to load templates");
    /// ```
    pub fn from_glob(glob: &str) -> tera::Result<Self> {
        Ok(Self {
            tera: Tera::new(glob)?
        })
    }

    /// Create a new instance of `TeraEngine`, containing all the parsed
    /// templates found in the directory.
    ///
    /// ```no_run
    /// use poem::templates::tera::TeraEngine;
    ///
    /// let tera = TeraEngine::from_directory("templates")
    ///     .expect("Failed to load templates");
    /// ```
    pub fn from_directory(template_directory: &str) -> tera::Result<Self> {
        Self::from_glob(&format!("{template_directory}/**/*"))
    }

    /// Create a new instance of `TeraEngine`, using a provided `Tera`
    /// instance.
    ///
    /// ```no_run
    /// use poem::templates::tera::{TeraEngine, Tera};
    ///
    /// let mut tera = Tera::new("templates/**/*").expect("Failed to parse templates");
    ///
    /// tera.autoescape_on(vec![".html", ".sql"]);
    /// let engine = TeraEngine::custom(tera);
    /// ```
    pub fn custom(tera: Tera) -> Self {
        Self { tera }
    }
}

impl Default for TeraEngine {
    fn default() -> Self {
        Self::from_directory("templates")
            .expect("Failed to load templates")
    }
}

impl<E: Endpoint> Middleware<E> for TeraEngine {
    type Output = TeraEndpoint<E>;

    fn transform(&self, inner: E) -> Self::Output {
        Self::Output {
            tera: self.tera.clone(),
            inner,
            transformers: Vec::new(),
        }
    }
}

/// Tera templates endpoint.
pub struct TeraEndpoint<E> {
    tera: Tera,
    inner: E,
    transformers: Vec<fn(&mut Tera, &mut Request)>,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for TeraEndpoint<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let mut tera = self.tera.clone();

        for transformer in &self.transformers {
            transformer(&mut tera, &mut req);
        }

        let response = self.inner.call(req).await?.into_response();

        match response.extensions().get::<TeraTemplate>() {
            Some(template) => {
                let result = tera.render(&template.name, &template.context);

                if let Err(e) = &result {
                    tracing::debug!("Tera Rendering error: {e:?}");
                    tracing::error!("Failed to render Tera template: {e}");
                }

                result.map(|s| Html(s).into_response())
                    .map_err(InternalServerError)
            },
            None => {
                // todo: double check if we should always error here
                tracing::error!("Missing template response");

                response.into_result()
            }
        }
    }
}

impl<E: Endpoint> TeraEndpoint<E> {
    /// Add a transformer that apply changes to each tera instances (for
    /// instance, registering a dynamic filter) before passing tera to
    /// request handlers
    ///
    /// ```no_run
    /// use poem::{Route, EndpointExt, templates::tera::TeraEngine};
    ///
    /// let app = Route::new()
    ///     .with(TeraEngine::default())
    ///     .using(|tera, req| println!("{tera:?}\n{req:?}"));
    /// ```
    pub fn using(mut self, transformer: fn(&mut Tera, &mut Request)) -> Self {
        self.transformers.push(transformer);
        self
    }

    /// Toggle live reloading. Defaults to enabled for debug and
    /// disabled for release builds.
    ///
    /// ```no_run
    /// use poem::{Route, EndpointExt, templates::tera::TeraEngine};
    ///
    /// let app = Route::new()
    ///     .with(TeraEngine::default())
    ///     .with_live_reloading(true);
    /// ```
    pub fn with_live_reloading(self, live_reloading: bool) -> Self {
        tracing::debug!("Live Reloading for Tera templates is enabled");

        todo!();

        self
    }
}
