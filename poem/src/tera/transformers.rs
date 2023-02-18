pub mod filters {
    use std::{collections::HashMap, borrow::Cow};
    use tera::{self, Tera, Value, Filter};
    use fluent::{
        types::{FluentNumber, FluentNumberOptions},
        FluentValue,
    };
    use crate::{Request, i18n::Locale, FromRequestSync};

    pub struct TranslateFilter {
        locale: Locale
    }

    impl Filter for TranslateFilter {
        fn filter(&self, id: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
            if args.len() == 0 {
                self.locale.text(id.as_str().unwrap())
            } else {
                let mut fluent_args = HashMap::new();
                for (key, value) in args {
                    fluent_args.insert(
                        key.as_str(),
                        match value {
                            Value::Null => FluentValue::None,
                            Value::Number(val) => FluentValue::Number(FluentNumber::new(
                                val.as_f64().unwrap(),
                                FluentNumberOptions::default(),
                            )),
                            Value::String(val) => FluentValue::String(Cow::Owned(val.clone())),
                            _ => FluentValue::Error,
                        },
                    );
                }
                self.locale
                    .text_with_args(id.as_str().unwrap(), fluent_args)
            }
            .map(|str| Value::String(str))
            .map_err(|err| tera::Error::msg(err))
        }
    }

    #[cfg(feature = "i18n")]
    #[cfg_attr(docsrs, doc(cfg(feature = "i18n")))]
    pub fn translate(tera: &mut Tera, req: &mut Request) {
        tera.register_filter("translate", TranslateFilter {
            locale: Locale::from_request_without_body_sync(req).unwrap()
        });
    }

}