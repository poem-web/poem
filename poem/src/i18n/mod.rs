//! Internationalization related types.
//!
//! # Load resources from file system
//!
//! ```no_run
//! use poem::i18n::I18NResources;
//!
//! let resources = I18NResources::builder()
//!     .add_path("/resources")
//!     .build()
//!     .unwrap();
//! ```
//!
//! # Load resources from string
//!
//! ```
//! use poem::i18n::I18NResources;
//!
//! let en_us = r#"
//! hello-world = Hello world!
//! welcome = Welcome { $name }!
//! "#;
//!
//! let zh_cn = r#"
//! hello-world = 你好！
//! welcome = 欢迎 { $name }！
//! "#;
//!
//! let resources = I18NResources::builder()
//!     .add_ftl("en-US", en_us)
//!     .add_ftl("zh-CN", zh_cn)
//!     .build()
//!     .unwrap();
//! ```
//!
//! # Negotiation
//!
//! ```no_run
//! use poem::i18n::I18NResources;
//! use unic_langid::{langid, langids};
//!
//! let resources = I18NResources::builder()
//!     .add_path("/resources")
//!     .build()
//!     .unwrap();
//!
//! let bundle = resources.negotiate_languages(&langids!("zh-CN", "en-US", "fr"));
//!
//! // get text
//! let hello_world = bundle.text("hello-world").unwrap();
//!
//! // get text with arguments
//! let welcome = bundle
//!     .text_with_args("welcome", (("name", "sunli"),))
//!     .unwrap();
//! ```
//!
//! # Use extractor
//!
//! See also: [`crate::i18n::Locale`]

mod args;
mod locale;
mod resources;

pub use fluent_langneg::NegotiationStrategy;
pub use unic_langid;

pub use self::{
    args::I18NArgs,
    locale::Locale,
    resources::{I18NBundle, I18NResources, I18NResourcesBuilder},
};
