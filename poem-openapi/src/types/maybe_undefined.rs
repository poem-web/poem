use std::{borrow::Cow, ops::Deref};

use poem::{http::HeaderValue, web::Field as PoemField};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::{
    registry::{MetaSchemaRef, Registry},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

/// Similar to `Option`, but it has three states, `undefined`, `null` and `x`.
///
/// # Example
///
/// ```
/// use poem::{test::TestClient, IntoEndpoint};
/// use poem_openapi::{payload::Json, types::MaybeUndefined, Object, OpenApi, OpenApiService};
/// use tokio::sync::Mutex;
/// use serde_json::json;
///
/// #[derive(Object, Clone, Default)]
/// struct Resource {
///     attr1: Option<i32>,
///     attr2: Option<String>,
/// }
///
/// #[derive(Object)]
/// struct UpdateResourceRequest {
///     attr1: MaybeUndefined<i32>,
///     attr2: MaybeUndefined<String>,
/// }
///
/// struct Api {
///     resource: Mutex<Resource>,
/// }
///
/// #[OpenApi]
/// impl Api {
///     #[oai(path = "/get", method = "get")]
///     async fn get_resource(&self) -> Json<Resource> {
///         Json(self.resource.lock().await.clone())
///     }
///
///     #[oai(path = "/put", method = "put")]
///     async fn update_resource(&self, req: Json<UpdateResourceRequest>) {
///         let mut resource = self.resource.lock().await;
///
///         match req.0.attr1 {
///             MaybeUndefined::Null => resource.attr1 = None,
///             MaybeUndefined::Value(value) => resource.attr1 = Some(value),
///             MaybeUndefined::Undefined => {}
///         }
///
///         match req.0.attr2 {
///             MaybeUndefined::Null => resource.attr2 = None,
///             MaybeUndefined::Value(value) => resource.attr2 = Some(value),
///             MaybeUndefined::Undefined => {}
///         }
///     }
/// }
///
/// let api_service = OpenApiService::new(
///     Api {
///         resource: Default::default(),
///     },
///     "Test",
///     "1.0",
/// );
///
/// let cli = TestClient::new(api_service.into_endpoint());
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// cli.get("/get").send().await.assert_json(json!({"attr1": null, "attr2": null}));
///
/// cli.put("/put").body_json(&json!({"attr1": 100i32})).send().await.assert_status_is_ok();
/// cli.get("/get").send().await.assert_json(json!({"attr1": 100i32, "attr2": null}));
///
/// cli.put("/put").body_json(&json!({"attr1": null, "attr2": "abc"})).send().await.assert_status_is_ok();
/// cli.get("/get").send().await.assert_json(json!({"attr1": null, "attr2": "abc"}));
/// # });
/// ```
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum MaybeUndefined<T> {
    /// Undefined
    Undefined,
    /// Null
    Null,
    /// Value
    Value(T),
}

impl<T> Default for MaybeUndefined<T> {
    fn default() -> Self {
        Self::Undefined
    }
}

impl<T> From<T> for MaybeUndefined<T> {
    fn from(value: T) -> Self {
        MaybeUndefined::Value(value)
    }
}

impl<T> IntoIterator for MaybeUndefined<T> {
    type Item = T;
    type IntoIter = std::option::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.take().into_iter()
    }
}

impl<T> MaybeUndefined<T> {
    /// Create a `MaybeUndefined<T>` from `Option<T>`, returns
    /// `MaybeUndefined::Undefined` if this value is none.
    pub fn from_opt_undefined(value: Option<T>) -> Self {
        match value {
            Some(value) => MaybeUndefined::Value(value),
            None => MaybeUndefined::Undefined,
        }
    }

    /// Create a `MaybeUndefined<T>` from `Option<T>`, returns
    /// `MaybeUndefined::Null` if this value is none.
    pub fn from_opt_null(value: Option<T>) -> Self {
        match value {
            Some(value) => MaybeUndefined::Value(value),
            None => MaybeUndefined::Null,
        }
    }

    /// Returns true if the `MaybeUndefined<T>` is undefined.
    #[inline]
    pub const fn is_undefined(&self) -> bool {
        matches!(self, MaybeUndefined::Undefined)
    }

    /// Returns true if the `MaybeUndefined<T>` is null.
    #[inline]
    pub const fn is_null(&self) -> bool {
        matches!(self, MaybeUndefined::Null)
    }

    /// Returns true if the `MaybeUndefined<T>` contains value.
    #[inline]
    pub const fn is_value(&self) -> bool {
        matches!(self, MaybeUndefined::Value(_))
    }

    /// Returns `None` if the the `MaybeUndefined<T>` is
    /// `undefined` or `null`, otherwise returns `Some(&T)`.
    #[inline]
    pub const fn value(&self) -> Option<&T> {
        match self {
            MaybeUndefined::Value(value) => Some(value),
            _ => None,
        }
    }

    /// Returns `None` if the the `MaybeUndefined<T>` is
    /// `undefined` or `null`, otherwise returns `Some(&mut T)`.
    #[inline]
    pub fn value_mut(&mut self) -> Option<&mut T> {
        match self {
            MaybeUndefined::Value(value) => Some(value),
            _ => None,
        }
    }

    /// Converts the `MaybeUndefined<T>` to `Option<T>`.
    #[inline]
    pub fn take(self) -> Option<T> {
        match self {
            MaybeUndefined::Value(value) => Some(value),
            _ => None,
        }
    }

    /// Converts from `&MaybeUndefined<T>` to `MaybeUndefined<&T>`.
    #[inline]
    pub const fn as_ref(&self) -> MaybeUndefined<&T> {
        match self {
            MaybeUndefined::Undefined => MaybeUndefined::Undefined,
            MaybeUndefined::Null => MaybeUndefined::Null,
            MaybeUndefined::Value(value) => MaybeUndefined::Value(value),
        }
    }

    /// Converts the `MaybeUndefined<T>` to `Option<Option<T>>`.
    #[inline]
    pub const fn as_opt_ref(&self) -> Option<Option<&T>> {
        match self {
            MaybeUndefined::Undefined => None,
            MaybeUndefined::Null => Some(None),
            MaybeUndefined::Value(value) => Some(Some(value)),
        }
    }

    /// Converts the `MaybeUndefined<T>` to `Option<Option<&U>>`.
    #[inline]
    pub fn as_opt_deref<U>(&self) -> Option<Option<&U>>
    where
        U: ?Sized,
        T: Deref<Target = U>,
    {
        match self {
            MaybeUndefined::Undefined => None,
            MaybeUndefined::Null => Some(None),
            MaybeUndefined::Value(value) => Some(Some(value.deref())),
        }
    }

    /// Returns `true` if the `MaybeUndefined<T>` contains the given value.
    #[inline]
    pub fn contains_value<U>(&self, x: &U) -> bool
    where
        U: PartialEq<T>,
    {
        match self {
            MaybeUndefined::Value(y) => x == y,
            _ => false,
        }
    }

    /// Returns `true` if the `MaybeUndefined<T>` contains the given nullable
    /// value.
    #[inline]
    pub fn contains<U>(&self, x: &Option<U>) -> bool
    where
        U: PartialEq<T>,
    {
        match self {
            MaybeUndefined::Value(y) => matches!(x, Some(v) if v == y),
            MaybeUndefined::Null => matches!(x, None),
            MaybeUndefined::Undefined => false,
        }
    }

    /// Maps a `MaybeUndefined<T>` to `MaybeUndefined<U>` by applying a function
    /// to the contained nullable value
    #[inline]
    pub fn map<U, F: FnOnce(Option<T>) -> Option<U>>(self, f: F) -> MaybeUndefined<U> {
        match self {
            MaybeUndefined::Value(v) => match f(Some(v)) {
                Some(v) => MaybeUndefined::Value(v),
                None => MaybeUndefined::Null,
            },
            MaybeUndefined::Null => match f(None) {
                Some(v) => MaybeUndefined::Value(v),
                None => MaybeUndefined::Null,
            },
            MaybeUndefined::Undefined => MaybeUndefined::Undefined,
        }
    }

    /// Maps a `MaybeUndefined<T>` to `MaybeUndefined<U>` by applying a function
    /// to the contained value
    #[inline]
    pub fn map_value<U, F: FnOnce(T) -> U>(self, f: F) -> MaybeUndefined<U> {
        match self {
            MaybeUndefined::Value(v) => MaybeUndefined::Value(f(v)),
            MaybeUndefined::Null => MaybeUndefined::Null,
            MaybeUndefined::Undefined => MaybeUndefined::Undefined,
        }
    }

    /// Update `value` if the `MaybeUndefined<T>` is not undefined.
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem_openapi::types::MaybeUndefined;
    ///
    /// let mut value = None;
    ///
    /// MaybeUndefined::Value(10i32).update_to(&mut value);
    /// assert_eq!(value, Some(10));
    ///
    /// MaybeUndefined::Undefined.update_to(&mut value);
    /// assert_eq!(value, Some(10));
    ///
    /// MaybeUndefined::Null.update_to(&mut value);
    /// assert_eq!(value, None);
    /// ```
    pub fn update_to(self, value: &mut Option<T>) {
        match self {
            MaybeUndefined::Value(new) => *value = Some(new),
            MaybeUndefined::Null => *value = None,
            MaybeUndefined::Undefined => {}
        };
    }
}

impl<T: Deref> MaybeUndefined<T> {
    /// Converts from `MaybeUndefined<T>` (or `&MaybeUndefined<T>`) to
    /// `MaybeUndefined<&T::Target>`.
    #[inline]
    pub fn as_deref(&self) -> MaybeUndefined<&T::Target> {
        match self {
            MaybeUndefined::Undefined => MaybeUndefined::Undefined,
            MaybeUndefined::Null => MaybeUndefined::Null,
            MaybeUndefined::Value(value) => MaybeUndefined::Value(value.deref()),
        }
    }
}

impl<T: Type> Type for MaybeUndefined<T> {
    const IS_REQUIRED: bool = false;

    type RawValueType = T::RawValueType;

    type RawElementValueType = T::RawElementValueType;

    fn name() -> Cow<'static, str> {
        format!("optional<{}>", T::name()).into()
    }

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        match self {
            MaybeUndefined::Value(value) => value.as_raw_value(),
            _ => None,
        }
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        match self {
            MaybeUndefined::Value(value) => value.raw_element_iter(),
            _ => Box::new(std::iter::empty()),
        }
    }

    #[inline]
    fn is_none(&self) -> bool {
        match self {
            MaybeUndefined::Undefined | MaybeUndefined::Null => true,
            MaybeUndefined::Value(_) => false,
        }
    }
}

impl<T: ParseFromJSON> ParseFromJSON for MaybeUndefined<T> {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        match value {
            Some(Value::Null) => Ok(MaybeUndefined::Null),
            Some(value) => Ok(MaybeUndefined::Value(
                T::parse_from_json(Some(value)).map_err(ParseError::propagate)?,
            )),
            None => Ok(MaybeUndefined::Undefined),
        }
    }
}

impl<T: ParseFromParameter> ParseFromParameter for MaybeUndefined<T> {
    fn parse_from_parameter(_value: &str) -> ParseResult<Self> {
        unreachable!()
    }

    fn parse_from_parameters<I: IntoIterator<Item = A>, A: AsRef<str>>(
        iter: I,
    ) -> ParseResult<Self> {
        let mut iter = iter.into_iter().peekable();

        if iter.peek().is_none() {
            return Ok(MaybeUndefined::Undefined);
        }

        T::parse_from_parameters(iter)
            .map_err(ParseError::propagate)
            .map(MaybeUndefined::Value)
    }
}

#[poem::async_trait]
impl<T: ParseFromMultipartField> ParseFromMultipartField for MaybeUndefined<T> {
    async fn parse_from_multipart(value: Option<PoemField>) -> ParseResult<Self> {
        match value {
            Some(value) => T::parse_from_multipart(Some(value))
                .await
                .map_err(ParseError::propagate)
                .map(MaybeUndefined::Value),
            None => Ok(MaybeUndefined::Undefined),
        }
    }
}

impl<T: ToJSON> ToJSON for MaybeUndefined<T> {
    fn to_json(&self) -> Option<Value> {
        match self {
            MaybeUndefined::Value(value) => value.to_json(),
            MaybeUndefined::Undefined => None,
            MaybeUndefined::Null => Some(Value::Null),
        }
    }
}

impl<T: ToHeader> ToHeader for MaybeUndefined<T> {
    fn to_header(&self) -> Option<HeaderValue> {
        match self {
            MaybeUndefined::Value(value) => value.to_header(),
            _ => None,
        }
    }
}

impl<T, E> MaybeUndefined<Result<T, E>> {
    /// Transposes a `MaybeUndefined` of a [`Result`] into a [`Result`] of a
    /// `MaybeUndefined`.
    ///
    /// [`MaybeUndefined::Undefined`] will be mapped to
    /// [`Ok`]`(`[`MaybeUndefined::Undefined`]`)`. [`MaybeUndefined::Null`]
    /// will be mapped to [`Ok`]`(`[`MaybeUndefined::Null`]`)`.
    /// [`MaybeUndefined::Value`]`(`[`Ok`]`(_))` and
    /// [`MaybeUndefined::Value`]`(`[`Err`]`(_))` will be mapped to
    /// [`Ok`]`(`[`MaybeUndefined::Value`]`(_))` and [`Err`]`(_)`.
    #[inline]
    pub fn transpose(self) -> Result<MaybeUndefined<T>, E> {
        match self {
            MaybeUndefined::Undefined => Ok(MaybeUndefined::Undefined),
            MaybeUndefined::Null => Ok(MaybeUndefined::Null),
            MaybeUndefined::Value(Ok(v)) => Ok(MaybeUndefined::Value(v)),
            MaybeUndefined::Value(Err(e)) => Err(e),
        }
    }
}

impl<T: Serialize> Serialize for MaybeUndefined<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MaybeUndefined::Value(value) => value.serialize(serializer),
            _ => serializer.serialize_none(),
        }
    }
}

impl<'de, T> Deserialize<'de> for MaybeUndefined<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<MaybeUndefined<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => MaybeUndefined::Value(value),
            None => MaybeUndefined::Null,
        })
    }
}

impl<T> From<MaybeUndefined<T>> for Option<Option<T>> {
    fn from(maybe_undefined: MaybeUndefined<T>) -> Self {
        match maybe_undefined {
            MaybeUndefined::Undefined => None,
            MaybeUndefined::Null => Some(None),
            MaybeUndefined::Value(value) => Some(Some(value)),
        }
    }
}

impl<T> From<Option<Option<T>>> for MaybeUndefined<T> {
    fn from(value: Option<Option<T>>) -> Self {
        match value {
            Some(Some(value)) => Self::Value(value),
            Some(None) => Self::Null,
            None => Self::Undefined,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::*;
    use crate::Object;

    #[test]
    fn test_maybe_undefined_serde() {
        assert_eq!(
            serde_json::to_value(MaybeUndefined::Value(100i32)).unwrap(),
            json!(100)
        );

        assert_eq!(
            serde_json::from_value::<MaybeUndefined<i32>>(json!(100)).unwrap(),
            MaybeUndefined::Value(100)
        );
        assert_eq!(
            serde_json::from_value::<MaybeUndefined<i32>>(json!(null)).unwrap(),
            MaybeUndefined::Null
        );

        #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
        struct A {
            a: MaybeUndefined<i32>,
        }

        assert_eq!(
            serde_json::to_value(&A {
                a: MaybeUndefined::Value(100i32)
            })
            .unwrap(),
            json!({"a": 100})
        );

        assert_eq!(
            serde_json::to_value(&A {
                a: MaybeUndefined::Null,
            })
            .unwrap(),
            json!({ "a": null })
        );

        assert_eq!(
            serde_json::to_value(&A {
                a: MaybeUndefined::Undefined,
            })
            .unwrap(),
            json!({ "a": null })
        );

        assert_eq!(
            serde_json::from_value::<A>(json!({"a": 100})).unwrap(),
            A {
                a: MaybeUndefined::Value(100i32)
            }
        );

        assert_eq!(
            serde_json::from_value::<A>(json!({ "a": null })).unwrap(),
            A {
                a: MaybeUndefined::Null
            }
        );

        assert_eq!(
            serde_json::from_value::<A>(json!({})).unwrap(),
            A {
                a: MaybeUndefined::Null
            }
        );
    }

    #[test]
    fn test_maybe_undefined_to_nested_option() {
        assert_eq!(Option::<Option<i32>>::from(MaybeUndefined::Undefined), None);

        assert_eq!(
            Option::<Option<i32>>::from(MaybeUndefined::Null),
            Some(None)
        );

        assert_eq!(
            Option::<Option<i32>>::from(MaybeUndefined::Value(42)),
            Some(Some(42))
        );
    }

    #[test]
    fn test_as_opt_ref() {
        let mut value = MaybeUndefined::<String>::Undefined;
        let mut r = value.as_opt_ref();
        assert_eq!(r, None);

        value = MaybeUndefined::Null;
        r = value.as_opt_ref();
        assert_eq!(r, Some(None));

        value = MaybeUndefined::Value("abc".to_string());
        r = value.as_opt_ref();
        assert_eq!(r, Some(Some(&"abc".to_string())));
    }

    #[test]
    fn test_as_opt_deref() {
        let mut value = MaybeUndefined::<String>::Undefined;
        let mut r = value.as_opt_deref();
        assert_eq!(r, None);

        value = MaybeUndefined::Null;
        r = value.as_opt_deref();
        assert_eq!(r, Some(None));

        value = MaybeUndefined::Value("abc".to_string());
        r = value.as_opt_deref();
        assert_eq!(r, Some(Some("abc")));
    }

    #[test]
    fn test_contains_value() {
        let test = "abc";

        let mut value: MaybeUndefined<String> = MaybeUndefined::Undefined;
        assert!(!value.contains_value(&test));

        value = MaybeUndefined::Null;
        assert!(!value.contains_value(&test));

        value = MaybeUndefined::Value("abc".to_string());
        assert!(value.contains_value(&test));
    }

    #[test]
    fn test_contains() {
        let test = Some("abc");
        let none: Option<&str> = None;

        let mut value: MaybeUndefined<String> = MaybeUndefined::Undefined;
        assert!(!value.contains(&test));
        assert!(!value.contains(&none));

        value = MaybeUndefined::Null;
        assert!(!value.contains(&test));
        assert!(value.contains(&none));

        value = MaybeUndefined::Value("abc".to_string());
        assert!(value.contains(&test));
        assert!(!value.contains(&none));
    }

    #[test]
    fn test_map_value() {
        let mut value: MaybeUndefined<i32> = MaybeUndefined::Undefined;
        assert_eq!(value.map_value(|v| v > 2), MaybeUndefined::Undefined);

        value = MaybeUndefined::Null;
        assert_eq!(value.map_value(|v| v > 2), MaybeUndefined::Null);

        value = MaybeUndefined::Value(5);
        assert_eq!(value.map_value(|v| v > 2), MaybeUndefined::Value(true));
    }

    #[test]
    fn test_map() {
        let mut value: MaybeUndefined<i32> = MaybeUndefined::Undefined;
        assert_eq!(value.map(|v| Some(v.is_some())), MaybeUndefined::Undefined);

        value = MaybeUndefined::Null;
        assert_eq!(
            value.map(|v| Some(v.is_some())),
            MaybeUndefined::Value(false)
        );

        value = MaybeUndefined::Value(5);
        assert_eq!(
            value.map(|v| Some(v.is_some())),
            MaybeUndefined::Value(true)
        );
    }

    #[test]
    fn test_transpose() {
        let mut value: MaybeUndefined<Result<i32, &'static str>> = MaybeUndefined::Undefined;
        assert_eq!(value.transpose(), Ok(MaybeUndefined::Undefined));

        value = MaybeUndefined::Null;
        assert_eq!(value.transpose(), Ok(MaybeUndefined::Null));

        value = MaybeUndefined::Value(Ok(5));
        assert_eq!(value.transpose(), Ok(MaybeUndefined::Value(5)));

        value = MaybeUndefined::Value(Err("error"));
        assert_eq!(value.transpose(), Err("error"));
    }

    #[test]
    fn test_parse_from_json() {
        assert_eq!(
            MaybeUndefined::<i32>::parse_from_json(Some(json!(100))).unwrap(),
            MaybeUndefined::Value(100)
        );

        assert_eq!(
            MaybeUndefined::<i32>::parse_from_json(Some(json!(null))).unwrap(),
            MaybeUndefined::Null
        );

        assert_eq!(
            MaybeUndefined::<i32>::parse_from_json(None).unwrap(),
            MaybeUndefined::Undefined
        );

        #[derive(Debug, Object, PartialEq)]
        #[oai(internal)]
        struct MyObj {
            a: MaybeUndefined<i32>,
        }

        assert_eq!(
            MyObj::parse_from_json(Some(json!({
                "a": 100,
            })))
            .unwrap(),
            MyObj {
                a: MaybeUndefined::Value(100)
            }
        );

        assert_eq!(
            MyObj::parse_from_json(Some(json!({
                "a": null,
            })))
            .unwrap(),
            MyObj {
                a: MaybeUndefined::Null
            }
        );

        assert_eq!(
            MyObj::parse_from_json(Some(json!({}))).unwrap(),
            MyObj {
                a: MaybeUndefined::Undefined
            }
        );
    }

    #[test]
    fn test_to_json() {
        assert_eq!(
            MaybeUndefined::<i32>::Value(100).to_json(),
            Some(json!(100))
        );
        assert_eq!(MaybeUndefined::<i32>::Null.to_json(), Some(json!(null)));
        assert_eq!(MaybeUndefined::<i32>::Undefined.to_json(), None);

        #[derive(Debug, Object, PartialEq)]
        #[oai(internal)]
        struct MyObj {
            a: MaybeUndefined<i32>,
        }

        assert_eq!(
            MyObj {
                a: MaybeUndefined::Value(100)
            }
            .to_json(),
            Some(json!({
                "a": 100,
            }))
        );

        assert_eq!(
            MyObj {
                a: MaybeUndefined::Null
            }
            .to_json(),
            Some(json!({
                "a": null,
            }))
        );

        assert_eq!(
            MyObj {
                a: MaybeUndefined::Undefined
            }
            .to_json(),
            Some(json!({}))
        );
    }
}
