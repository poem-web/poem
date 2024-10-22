use geo_types::*;

use crate::types::Type;

trait GeoJson {
    type Coordinates: Type;
}

macro_rules! impl_geojson_types {
    ($geometry:tt, $name:literal, $coordinates:ty) => {
        impl<T: CoordNum + crate::types::Type> GeoJson for $geometry<T> {
            type Coordinates = $coordinates;
        }

        impl crate::types::Type for $geometry {
            const IS_REQUIRED: bool = true;
            type RawValueType = Self;
            type RawElementValueType = Self;

            fn name() -> ::std::borrow::Cow<'static, str> {
                concat!("GeoJSON_", $name).into()
            }

            fn schema_ref() -> crate::registry::MetaSchemaRef {
                crate::registry::MetaSchemaRef::Reference(Self::name().into_owned())
            }

            fn register(registry: &mut crate::registry::Registry) {
                registry.create_schema::<Self, _>(Self::name().into_owned(), |registry| {
                    String::register(registry);
                    <<Self as GeoJson>::Coordinates>::register(registry);
                    crate::registry::MetaSchema {
                        required: vec!["type", "coordinates"],
                        properties: vec![
                            ("type", String::schema_ref()),
                            (
                                "coordinates",
                                <<Self as GeoJson>::Coordinates>::schema_ref(),
                            ),
                        ],
                        ..crate::registry::MetaSchema::new("object")
                    }
                })
            }

            fn as_raw_value(&self) -> Option<&Self::RawValueType> {
                Some(self)
            }

            fn raw_element_iter<'a>(
                &'a self,
            ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
                Box::new(IntoIterator::into_iter(self.as_raw_value()))
            }
        }

        impl crate::types::ParseFromJSON for $geometry {
            fn parse_from_json(
                value: Option<::serde_json::Value>,
            ) -> Result<Self, crate::types::ParseError<Self>> {
                let value = value.ok_or(crate::types::ParseError::expected_input())?;
                Self::try_from(geojson::Geometry::try_from(value)?).map_err(Into::into)
            }
        }

        impl crate::types::ToJSON for $geometry {
            fn to_json(&self) -> Option<::serde_json::Value> {
                Some(
                    ::serde_json::Map::<String, ::serde_json::Value>::from(
                        &geojson::Geometry::from(self),
                    )
                    .into(),
                )
            }
        }
    };
}

impl_geojson_types!(Point, "Point", [T; 2]);
impl_geojson_types!(MultiPoint, "MultiPoint", Vec<[T; 2]>);
impl_geojson_types!(LineString, "LineString", Vec<[T; 2]>);
impl_geojson_types!(MultiLineString, "MultiLineString", Vec<Vec<[T; 2]>>);
impl_geojson_types!(Polygon, "Polygon", Vec<Vec<[T; 2]>>);
impl_geojson_types!(MultiPolygon, "MultiPolygon", Vec<Vec<Vec<[T; 2]>>>);

#[cfg(test)]
mod tests {
    use geo_types::Point;

    use crate::types::{ParseFromJSON, ToJSON};

    fn point_geo() -> Point {
        Point::new(1.0, 2.0)
    }

    fn point_json() -> serde_json::Value {
        serde_json::json!({
            "type": "Point",
            "coordinates": [1.0, 2.0]
        })
    }

    #[test]
    fn serializes_geo_to_json() {
        assert_eq!(point_json(), point_geo().to_json().unwrap())
    }

    #[test]
    fn deserializes_json_to_geo() {
        assert_eq!(
            Point::parse_from_json(Some(point_json())).unwrap(),
            point_geo()
        )
    }
}
