use std::{borrow::Cow, collections::HashMap};

use fluent::{FluentArgs, FluentValue};

/// Parameters for formatting the message.
#[derive(Default)]
pub struct I18NArgs<'a>(pub(crate) FluentArgs<'a>);

impl<'a> I18NArgs<'a> {
    /// Append a KV pair to arguments.
    #[must_use]
    pub fn set<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<Cow<'a, str>>,
        V: Into<FluentValue<'a>>,
    {
        self.0.set(key, value);
        self
    }
}

impl<'a, K, V> From<HashMap<K, V>> for I18NArgs<'a>
where
    K: Into<Cow<'a, str>>,
    V: Into<FluentValue<'a>>,
{
    fn from(map: HashMap<K, V>) -> Self {
        let mut args = FluentArgs::new();
        for (key, value) in map {
            args.set(key, value);
        }
        Self(args)
    }
}

macro_rules! impl_from_tuples {
    (($head_key:ident, $head_value:ident), $(($key:ident, $value:ident),)*) => {
        impl<'a, $head_key, $head_value, $($key, $value),*> From<(($head_key, $head_value), $(($key, $value)),*)> for I18NArgs<'a>
        where
            $head_key: Into<Cow<'a, str>>,
            $head_value: Into<FluentValue<'a>>,
            $(
            $key: Into<Cow<'a, str>>,
            $value: Into<FluentValue<'a>>,
            )*
        {
            #[allow(non_snake_case)]
            fn from((($head_key, $head_value), $(($key, $value)),*): (($head_key, $head_value), $(($key, $value)),*)) -> Self {
                let mut args = FluentArgs::new();
                args.set($head_key, $head_value);
                $(
                args.set($key, $value);
                )*
                Self(args)
            }
        }

        impl_from_tuples!($(($key, $value),)*);
    };

    () => {}
}

#[rustfmt::skip]
impl_from_tuples!(
    (K1, V1), (K2, V2), (K3, V3), (K4, V4), (K5, V5), (K6, V6), (K7, V7), (K8, V8),
    (K9, V9), (K10, V10), (K11, V11), (K12, V12), (K13, V13), (K14, V14), (K15, V15), (K16, V16),
);
