use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use fluent::{FluentMessage, FluentResource};
use intl_memoizer::concurrent::IntlLangMemoizer;
use smallvec::SmallVec;
use unic_langid::{langid, LanguageIdentifier};

use crate::{error::I18NError, Result};

type FluentBundle = fluent::bundle::FluentBundle<FluentResource, IntlLangMemoizer>;

use fluent_langneg::NegotiationStrategy;

use crate::i18n::I18NArgs;

struct InnerResources {
    available_languages: Vec<LanguageIdentifier>,
    bundles: HashMap<LanguageIdentifier, Arc<FluentBundle>>,
    default_language: LanguageIdentifier,
    strategy: NegotiationStrategy,
}

/// I18N resources builder.
pub struct I18NResourcesBuilder {
    paths: Vec<PathBuf>,
    resources: Vec<(String, String)>,
    default_language: LanguageIdentifier,
    strategy: NegotiationStrategy,
}

impl I18NResourcesBuilder {
    /// Add resources directory.
    ///
    /// The resource directory contains multiple language directories, each of
    /// which can contain multiple FTL files for that language.
    ///
    /// ```text
    /// /resources
    ///     /en-US
    ///         simple.ftl
    ///         errors.ftl
    ///     /zh-CN
    ///         simple.ftl
    ///         errors.ftl
    /// ```
    #[must_use]
    pub fn add_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.paths.push(path.into());
        self
    }

    /// Add FTL(Fluent Translation List) for the specified language.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::i18n::I18NResources;
    ///
    /// let resources = I18NResources::builder()
    ///     .add_ftl("en-US", "hello-world = Hello world!")
    ///     .add_ftl("zh-CN", "hello-world = 你好世界！")
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn add_ftl(mut self, language: impl Into<String>, ftl: impl Into<String>) -> Self {
        self.resources.push((language.into(), ftl.into()));
        self
    }

    /// Set the default language when the language negotiation fails.
    ///
    /// NOTE: This language ID matches exactly the language id contained in the
    /// resources.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::i18n::I18NResources;
    /// use unic_langid::langid;
    ///
    /// let resources = I18NResources::builder()
    ///     .add_ftl("en-US", "hello-world = Hello world!")
    ///     .add_ftl("zh-CN", "hello-world = 你好世界！")
    ///     .default_language(langid!("en-US"))
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn default_language(mut self, language: LanguageIdentifier) -> Self {
        self.default_language = language;
        self
    }

    /// Sets the negotiation strategy.
    ///
    /// Default is [`NegotiationStrategy::Filtering`].
    #[must_use]
    pub fn negotiation_strategy(mut self, strategy: NegotiationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Consumes this builder and returns a [`I18NResources`] object.
    pub fn build(self) -> Result<I18NResources, I18NError> {
        let mut bundles = HashMap::new();

        for path in self.paths {
            load_resources_from_path(&mut bundles, path)?;
        }

        for (language, ftl) in self.resources {
            let language = LanguageIdentifier::from_str(&language)?;
            let resource = FluentResource::try_new(ftl)
                .map_err(|(_, errors)| I18NError::FluentParser(errors))?;

            bundles
                .entry(language.clone())
                .or_insert_with(|| FluentBundle::new_concurrent(vec![language]))
                .add_resource(resource)
                .map_err(I18NError::Fluent)?;
        }

        Ok(I18NResources {
            inner: Arc::new(InnerResources {
                available_languages: bundles.keys().cloned().collect(),
                bundles: bundles
                    .into_iter()
                    .map(|(key, value)| (key, Arc::new(value)))
                    .collect(),
                default_language: self.default_language,
                strategy: self.strategy,
            }),
        })
    }
}

fn load_resources_from_path(
    bundles: &mut HashMap<LanguageIdentifier, FluentBundle>,
    path: impl AsRef<Path>,
) -> Result<(), I18NError> {
    let path = path.as_ref();
    let languages = std::fs::read_dir(path)?;

    for res in languages {
        let language_dir = res?;

        let language = match language_dir
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| LanguageIdentifier::from_str(name).ok())
        {
            Some(language_path) => language_path,
            None => continue,
        };

        let resources = std::fs::read_dir(language_dir.path())?;

        for res in resources {
            let resource_path = res?;

            tracing::debug!(path = ?resource_path.path(), "load fluent resource");

            let resource = FluentResource::try_new(std::fs::read_to_string(resource_path.path())?)
                .map_err(|(_, errors)| I18NError::FluentParser(errors))?;

            bundles
                .entry(language.clone())
                .or_insert_with(|| FluentBundle::new_concurrent(vec![language.clone()]))
                .add_resource(resource)
                .map_err(I18NError::Fluent)?;
        }
    }

    Ok(())
}

/// A resource for translating natural language.
#[derive(Clone)]
pub struct I18NResources {
    inner: Arc<InnerResources>,
}

impl I18NResources {
    /// Create a resources builder.
    pub fn builder() -> I18NResourcesBuilder {
        I18NResourcesBuilder {
            paths: vec![],
            resources: vec![],
            default_language: langid!("en-US"),
            strategy: NegotiationStrategy::Filtering,
        }
    }

    /// Negotiate the language according to the input language id list and
    /// return the [`I18NBundle`].
    pub fn negotiate_languages(&self, languages: &[impl AsRef<LanguageIdentifier>]) -> I18NBundle {
        let resolved_languages = fluent_langneg::negotiate_languages(
            languages,
            &self.inner.available_languages,
            Some(&self.inner.default_language),
            self.inner.strategy,
        );

        I18NBundle(
            resolved_languages
                .into_iter()
                .filter_map(|language| self.inner.bundles.get(language))
                .cloned()
                .collect(),
        )
    }
}

/// A collection of localization messages.
pub struct I18NBundle(SmallVec<[Arc<FluentBundle>; 8]>);

impl I18NBundle {
    fn message(&self, id: impl AsRef<str>) -> Result<(&FluentBundle, FluentMessage), I18NError> {
        let id = id.as_ref();
        for bundle in &self.0 {
            if let Some(message) = bundle.get_message(id) {
                return Ok((bundle, message));
            }
        }
        Err(I18NError::FluentMessageNotFound { id: id.to_string() })
    }

    /// Gets the text with arguments.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::i18n::I18NResources;
    /// use unic_langid::langid;
    ///
    /// let resources = I18NResources::builder()
    ///     .add_ftl(
    ///         "en-US",
    ///         "input-parse-error = Error: Could not parse input `{ $input }`. Reason: { $reason }",
    ///     )
    ///     .build()
    ///     .unwrap();
    /// let bundle = resources.negotiate_languages(&[langid!("en-US")]);
    ///
    /// let err = "abc".parse::<i32>().unwrap_err();
    /// assert_eq!(
    ///     bundle
    ///         .text_with_args(
    ///             "input-parse-error",
    ///             (("input", "abc"), ("reason", err.to_string()))
    ///         )
    ///         .unwrap(),
    ///     "Error: Could not parse input `\u{2068}abc\u{2069}`. Reason: \u{2068}invalid digit found in string\u{2069}"
    /// );
    /// ```
    pub fn text_with_args<'a>(
        &self,
        id: impl AsRef<str>,
        args: impl Into<I18NArgs<'a>>,
    ) -> Result<String, I18NError> {
        let mut errors = Vec::new();
        let (bundle, message) = self.message(id.as_ref())?;
        let args = args.into();
        let value = message.value().ok_or(I18NError::FluentNoValue)?;
        let s = bundle.format_pattern(value, Some(&args.0), &mut errors);
        if !errors.is_empty() {
            return Err(I18NError::Fluent(errors));
        }
        Ok(s.into_owned())
    }

    /// Gets the text.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::i18n::I18NResources;
    /// use unic_langid::langid;
    ///
    /// let resources = I18NResources::builder()
    ///     .add_ftl("en-US", "hello-world = Hello world!")
    ///     .build()
    ///     .unwrap();
    /// let bundle = resources.negotiate_languages(&[langid!("en-US")]);
    ///
    /// assert_eq!(bundle.text("hello-world").unwrap(), "Hello world!");
    /// ```
    pub fn text(&self, id: impl AsRef<str>) -> Result<String, I18NError> {
        self.text_with_args(id, I18NArgs::default())
    }
}
