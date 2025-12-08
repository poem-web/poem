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

                    // Create enum schema for the type field
                    let type_schema = crate::registry::MetaSchemaRef::Inline(Box::new(
                        crate::registry::MetaSchema {
                            enum_items: vec![::serde_json::Value::String($name.to_string())],
                            ..crate::registry::MetaSchema::new("string")
                        },
                    ));

                    // Create bbox schema (optional array of numbers with minItems: 4)
                    let bbox_schema = crate::registry::MetaSchemaRef::Inline(Box::new(
                        crate::registry::MetaSchema {
                            items: Some(Box::new(crate::registry::MetaSchemaRef::Inline(
                                Box::new(crate::registry::MetaSchema::new("number")),
                            ))),
                            min_items: Some(4),
                            ..crate::registry::MetaSchema::new("array")
                        },
                    ));

                    crate::registry::MetaSchema {
                        title: Some(concat!("GeoJSON ", $name).to_string()),
                        required: vec!["type", "coordinates"],
                        properties: vec![
                            ("type", type_schema),
                            (
                                "coordinates",
                                <<Self as GeoJson>::Coordinates>::schema_ref(),
                            ),
                            ("bbox", bbox_schema),
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

// Implementation for Geometry enum (union of all geometry types)
impl crate::types::Type for Geometry {
    const IS_REQUIRED: bool = true;
    type RawValueType = Self;
    type RawElementValueType = Self;

    fn name() -> ::std::borrow::Cow<'static, str> {
        "GeoJSON_Geometry".into()
    }

    fn schema_ref() -> crate::registry::MetaSchemaRef {
        crate::registry::MetaSchemaRef::Reference(Self::name().into_owned())
    }

    fn register(registry: &mut crate::registry::Registry) {
        registry.create_schema::<Self, _>(Self::name().into_owned(), |registry| {
            // Register all the individual geometry types
            Point::register(registry);
            LineString::register(registry);
            Polygon::register(registry);
            MultiPoint::register(registry);
            MultiLineString::register(registry);
            MultiPolygon::register(registry);

            crate::registry::MetaSchema {
                title: Some("GeoJSON Geometry".to_string()),
                one_of: vec![
                    Point::schema_ref(),
                    LineString::schema_ref(),
                    Polygon::schema_ref(),
                    MultiPoint::schema_ref(),
                    MultiLineString::schema_ref(),
                    MultiPolygon::schema_ref(),
                ],
                ..crate::registry::MetaSchema::ANY
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

impl crate::types::ParseFromJSON for Geometry {
    fn parse_from_json(
        value: Option<::serde_json::Value>,
    ) -> Result<Self, crate::types::ParseError<Self>> {
        let value = value.ok_or(crate::types::ParseError::expected_input())?;

        // Try to parse as a geojson::Geometry and convert to geo_types::Geometry
        let geojson_geom = geojson::Geometry::try_from(value).map_err(|e| {
            crate::types::ParseError::custom(format!("Invalid GeoJSON geometry: {}", e))
        })?;

        Self::try_from(&geojson_geom).map_err(Into::into)
    }
}

impl crate::types::ToJSON for Geometry {
    fn to_json(&self) -> Option<::serde_json::Value> {
        // Convert to geojson::Geometry and then to JSON
        let geojson_geom = geojson::Geometry::from(self);
        Some(::serde_json::Map::<String, ::serde_json::Value>::from(&geojson_geom).into())
    }
}

#[cfg(test)]
mod tests {
    use geo_types::{Geometry, LineString, Point};

    use crate::registry::{MetaSchemaRef, Registry};
    use crate::types::{ParseFromJSON, ToJSON, Type};

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

    #[test]
    fn schema_has_correct_structure() {
        let mut registry = Registry::new();
        Point::<f64>::register(&mut registry);

        let schema_name = Point::<f64>::name();
        let schema = registry.schemas.get(schema_name.as_ref()).unwrap();

        // Check title
        assert_eq!(schema.title, Some("GeoJSON Point".to_string()));

        // Check type
        assert_eq!(schema.ty, "object");

        // Check required fields
        assert_eq!(schema.required, vec!["type", "coordinates"]);

        // Check properties
        assert_eq!(schema.properties.len(), 3);
        assert_eq!(schema.properties[0].0, "type");
        assert_eq!(schema.properties[1].0, "coordinates");
        assert_eq!(schema.properties[2].0, "bbox");

        // Check type field has enum constraint
        if let MetaSchemaRef::Inline(type_schema) = &schema.properties[0].1 {
            assert_eq!(type_schema.ty, "string");
            assert_eq!(
                type_schema.enum_items,
                vec![serde_json::Value::String("Point".to_string())]
            );
        } else {
            panic!("type field should be inline schema");
        }

        // Check coordinates field uses minItems instead of minLength
        if let MetaSchemaRef::Inline(coords_schema) = &schema.properties[1].1 {
            assert_eq!(coords_schema.ty, "array");
            assert_eq!(coords_schema.min_items, Some(2));
            assert_eq!(coords_schema.max_items, Some(2));
            assert_eq!(coords_schema.min_length, None);
            assert_eq!(coords_schema.max_length, None);
        } else {
            panic!("coordinates field should be inline schema");
        }

        // Check bbox field structure
        if let MetaSchemaRef::Inline(bbox_schema) = &schema.properties[2].1 {
            assert_eq!(bbox_schema.ty, "array");
            assert_eq!(bbox_schema.min_items, Some(4));
            if let Some(items) = &bbox_schema.items {
                if let MetaSchemaRef::Inline(item_schema) = items.as_ref() {
                    assert_eq!(item_schema.ty, "number");
                } else {
                    panic!("bbox items should be inline schema");
                }
            } else {
                panic!("bbox should have items");
            }
        } else {
            panic!("bbox field should be inline schema");
        }
    }

    #[test]
    fn geometry_enum_serializes_point() {
        let point: Geometry = Geometry::Point(Point::new(1.0, 2.0));
        let json = point.to_json().unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "type": "Point",
                "coordinates": [1.0, 2.0]
            })
        );
    }

    #[test]
    fn geometry_enum_serializes_linestring() {
        let linestring: Geometry =
            Geometry::LineString(LineString::from(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]));
        let json = linestring.to_json().unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "type": "LineString",
                "coordinates": [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]
            })
        );
    }

    #[test]
    fn geometry_enum_deserializes_point() {
        let json = serde_json::json!({
            "type": "Point",
            "coordinates": [1.0, 2.0]
        });

        let geometry = Geometry::parse_from_json(Some(json)).unwrap();

        match geometry {
            Geometry::Point(p) => {
                assert_eq!(p.x(), 1.0);
                assert_eq!(p.y(), 2.0);
            }
            _ => panic!("Expected Point variant"),
        }
    }

    #[test]
    fn geometry_enum_deserializes_linestring() {
        let json = serde_json::json!({
            "type": "LineString",
            "coordinates": [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]
        });

        let geometry = Geometry::parse_from_json(Some(json)).unwrap();

        match geometry {
            Geometry::LineString(ls) => {
                assert_eq!(ls.0.len(), 3);
            }
            _ => panic!("Expected LineString variant"),
        }
    }

    #[test]
    fn geometry_enum_schema_uses_oneof() {
        let mut registry = Registry::new();
        Geometry::register(&mut registry);

        let schema_name = Geometry::name();
        let schema = registry.schemas.get(schema_name.as_ref()).unwrap();

        // Check title
        assert_eq!(schema.title, Some("GeoJSON Geometry".to_string()));

        // Check that it uses oneOf with all geometry types
        assert_eq!(schema.one_of.len(), 6);

        // Verify that the oneOf includes references to each geometry type
        let one_of_refs: Vec<String> = schema
            .one_of
            .iter()
            .filter_map(|ref_| match ref_ {
                MetaSchemaRef::Reference(name) => Some(name.clone()),
                _ => None,
            })
            .collect();

        assert!(one_of_refs.contains(&"GeoJSON_Point".to_string()));
        assert!(one_of_refs.contains(&"GeoJSON_LineString".to_string()));
        assert!(one_of_refs.contains(&"GeoJSON_Polygon".to_string()));
        assert!(one_of_refs.contains(&"GeoJSON_MultiPoint".to_string()));
        assert!(one_of_refs.contains(&"GeoJSON_MultiLineString".to_string()));
        assert!(one_of_refs.contains(&"GeoJSON_MultiPolygon".to_string()));
    }
}
