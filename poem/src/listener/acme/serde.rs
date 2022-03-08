use std::{
    fmt::{self, Debug, Formatter},
    ops::Deref,
};

use http::Uri;
use serde::{de::Error, Deserialize, Deserializer};

pub(crate) struct SerdeUri(pub(crate) Uri);

impl<'de> Deserialize<'de> for SerdeUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse::<Uri>()
            .map(SerdeUri)
            .map_err(|err| D::Error::custom(err.to_string()))
    }
}

impl Debug for SerdeUri {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Deref for SerdeUri {
    type Target = Uri;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
