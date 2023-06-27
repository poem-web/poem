use tera::Tera;

use super::Flavor;

use crate::{
    templates::Template,
    error::{InternalServerError, IntoResult},
    web::Html,
    Endpoint, Middleware, Request, Result,
    Response, IntoResponse,
};

#[cfg(feature = "live_reloading")]
use crate::templates::live_reloading::{ Watcher, LiveReloading };

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
            tera: Flavor::Immutable(self.tera.clone()),
            inner,
            transformers: Vec::new(),
        }
    }
}

/// Tera templates endpoint.
pub struct TeraEndpoint<E> {
    tera: Flavor,
    inner: E,
    transformers: Vec<Box<dyn Fn(&mut Tera, &mut Request) + Send + Sync + 'static>>,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for TeraEndpoint<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let mut tera = match &self.tera {
            Flavor::Immutable(t) => t.clone(),

            #[cfg(feature = "live_reloading")]
            Flavor::LiveReload { tera, watcher } => {
                let lock = if watcher.needs_reload() {
                    tracing::info!("Detected changes to templates, reloading...");

                    let mut lock = tera.write().await;

                    if let Err(e) = lock.full_reload() {
                        tracing::error!("Failed to reload templates: {e}");
                        tracing::debug!("Reload templates error: {e:?}");

                        return Err(InternalServerError(e));
                    }

                    lock.downgrade()
                } else {
                    tera.read().await
                };

                lock.clone()
            }
        };

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
                // todo: this destroys the type
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
    pub fn using<F>(mut self, transformer: F) -> Self where
        F: Fn(&mut Tera, &mut Request) + Send + Sync + 'static
    {
        self.transformers.push(Box::new(transformer));
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
    ///     .with_live_reloading(LiveReloading::Disabled);
    /// ```

    #[cfg(feature = "live_reloading")]
    #[cfg_attr(docsrs, doc(cfg(feature = "live_reloading")))]
    pub fn with_live_reloading(mut self, live_reloading: LiveReloading) -> Self {
        self.tera = match (self.tera, live_reloading) {
            #[cfg(debug_assertions)]
            (Flavor::Immutable(tera), LiveReloading::Debug(path)) => {
                tracing::debug!("Live reloading for Tera templates is enabled");

                Flavor::LiveReload { tera: tokio::sync::RwLock::new(tera), watcher: Watcher::new(path) }
            },

            (Flavor::Immutable(tera), LiveReloading::Enabled(path)) => {
                tracing::debug!("Live reloading for Tera templates is enabled");

                Flavor::LiveReload { tera: tokio::sync::RwLock::new(tera), watcher: Watcher::new(path) }
            },

            #[cfg(not(debug_assertions))]
            (Flavor::LiveReload { tera, .. }, LiveReloading::Debug(_)) => {
                tracing::debug!("Live reloading for Tera templates is disabled");

                Flavor::Immutable(tera.into_inner())
            },

            (Flavor::LiveReload { tera, .. }, LiveReloading::Disabled) => {
                tracing::debug!("Live reloading for Tera templates is disabled");

                Flavor::Immutable(tera.into_inner())
            },

            // todo: enable changing watch path

            (tera, _) => tera
        };

        self
    }
}
