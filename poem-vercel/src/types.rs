use base64::display::Base64Display;
use poem::http::{HeaderMap, Method};
use serde::{de::Error as _, ser::Error as _, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Deserialize, Debug)]
pub(crate) struct VercelEvent {
    #[serde(rename = "Action")]
    pub(crate) action: String,
    #[serde(deserialize_with = "deserialize_event_body")]
    pub(crate) body: VercelRequest,
}

fn deserialize_event_body<'de, D>(deserializer: D) -> Result<VercelRequest, D::Error>
where
    D: Deserializer<'de>,
{
    serde_json::from_str(&String::deserialize(deserializer)?).map_err(D::Error::custom)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VercelRequest {
    pub(crate) host: String,
    pub(crate) path: String,
    #[serde(with = "http_serde::method")]
    pub(crate) method: Method,
    #[serde(with = "http_serde::header_map")]
    pub(crate) headers: HeaderMap,
    pub(crate) body: Option<String>,
    pub(crate) encoding: Option<String>,
}

#[derive(Debug)]
pub(crate) enum VercelBody {
    /// An empty body
    Empty,
    /// A body containing string data
    Text(String),
    /// A body containing binary data
    Binary(Vec<u8>),
}

impl<'a> Serialize for VercelBody {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            VercelBody::Text(data) => serializer
                .serialize_str(::std::str::from_utf8(data.as_ref()).map_err(S::Error::custom)?),
            VercelBody::Binary(data) => {
                serializer.collect_str(&Base64Display::with_config(data, base64::STANDARD))
            }
            VercelBody::Empty => serializer.serialize_unit(),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VercelResponse {
    pub(crate) status_code: u16,
    #[serde(
        skip_serializing_if = "HeaderMap::is_empty",
        with = "http_serde::header_map"
    )]
    pub(crate) headers: HeaderMap,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<VercelBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) encoding: Option<String>,
}
