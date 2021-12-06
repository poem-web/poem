use poem_openapi::{
    registry::{MetaSchema, MetaSchemaRef, Registry},
    types::{ParseFromJSON, ToJSON, Type},
    Enum, Object,
};
use serde_json::json;

fn get_meta<T: Type>() -> MetaSchema {
    let mut registry = Registry::new();
    T::register(&mut registry);
    registry.schemas.remove(&*T::name()).unwrap()
}

#[test]
fn rename() {
    #[derive(Object)]
    #[oai(rename = "Abc")]
    struct Obj {
        a: i32,
    }
    assert_eq!(Obj::name(), "Abc");
}

#[test]
fn rename_all() {
    #[derive(Object)]
    #[oai(rename_all = "camelCase")]
    struct Obj {
        create_user: i32,
        delete_user: i32,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.properties[0].0, "createUser");
    assert_eq!(meta.properties[1].0, "deleteUser");
}
#[test]
fn concretes() {
    #[derive(Object)]
    #[oai(
        concrete(name = "Obj_i32_i64", params(i32, i64)),
        concrete(name = "Obj_f32_f64", params(f32, f64))
    )]
    struct Obj<T1: ParseFromJSON + ToJSON, T2: ParseFromJSON + ToJSON> {
        create_user: T1,
        delete_user: T2,
    }

    assert_eq!(<Obj<i32, i64>>::name(), "Obj_i32_i64");
    let meta = get_meta::<Obj<i32, i64>>();
    assert_eq!(meta.properties[0].1.unwrap_inline().ty, "integer");
    assert_eq!(meta.properties[0].1.unwrap_inline().format, Some("int32"));

    assert_eq!(meta.properties[1].1.unwrap_inline().ty, "integer");
    assert_eq!(meta.properties[1].1.unwrap_inline().format, Some("int64"));

    assert_eq!(<Obj<f32, f64>>::name(), "Obj_f32_f64");
    let meta = get_meta::<Obj<f32, f64>>();
    assert_eq!(meta.properties[0].1.unwrap_inline().ty, "number");
    assert_eq!(meta.properties[0].1.unwrap_inline().format, Some("float32"));

    assert_eq!(meta.properties[1].1.unwrap_inline().ty, "number");
    assert_eq!(meta.properties[1].1.unwrap_inline().format, Some("float64"));
}

#[test]
fn deprecated() {
    #[derive(Object)]
    struct Obj {
        a: i32,
    }

    let meta = get_meta::<Obj>();
    assert!(!meta.deprecated);

    #[derive(Object)]
    #[oai(deprecated)]
    struct ObjDeprecated {
        a: i32,
    }

    let meta = get_meta::<ObjDeprecated>();
    assert!(meta.deprecated);
}

#[test]
fn read_only_all() {
    #[derive(Debug, Object, PartialEq)]
    #[oai(read_only_all)]
    struct Obj {
        id: i32,
        value: i32,
    }

    let meta = get_meta::<Obj>();
    let field_id_schema = meta.properties[0].1.unwrap_inline();
    let field_value_schema = meta.properties[1].1.unwrap_inline();
    assert!(field_id_schema.read_only);
    assert!(!field_id_schema.write_only);
    assert!(field_value_schema.read_only);
    assert!(!field_value_schema.write_only);

    assert_eq!(
        serde_json::to_value(Obj { id: 99, value: 100 }).unwrap(),
        serde_json::json!({
            "id": 99,
            "value": 100,
        })
    );

    assert_eq!(
        serde_json::from_value::<Obj>(serde_json::json!({
            "id": 99,
            "value": 100,
        }))
        .unwrap_err()
        .to_string(),
        r#"failed to parse "Obj": properties `id` is read only."#,
    );
}

#[test]
fn write_only_all() {
    #[derive(Debug, Object, PartialEq)]
    #[oai(write_only_all)]
    struct Obj {
        id: i32,
        value: i32,
    }

    let meta = get_meta::<Obj>();
    let field_id_schema = meta.properties[0].1.unwrap_inline();
    let field_value_schema = meta.properties[1].1.unwrap_inline();
    assert!(!field_id_schema.read_only);
    assert!(field_id_schema.write_only);
    assert!(!field_value_schema.read_only);
    assert!(field_value_schema.write_only);

    assert_eq!(
        serde_json::from_value::<Obj>(serde_json::json!({
            "id": 99,
            "value": 100,
        }))
        .unwrap(),
        Obj { id: 99, value: 100 }
    );

    assert_eq!(
        serde_json::to_value(Obj { id: 99, value: 100 }).unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn field_skip() {
    #[derive(Object, Debug, Eq, PartialEq)]
    struct Obj {
        a: i32,
        #[oai(skip)]
        b: i32,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.properties.len(), 1);

    assert_eq!(
        Obj::parse_from_json(json!({
            "a": 10,
        }))
        .unwrap(),
        Obj { a: 10, b: 0 }
    );

    assert_eq!(
        Obj { a: 10, b: 0 }.to_json(),
        json!({
            "a": 10,
        })
    );
}

#[test]
fn field_rename() {
    #[derive(Object)]
    struct Obj {
        #[oai(rename = "b")]
        a: i32,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.properties[0].0, "b");
}

#[test]
fn recursive_register() {
    #[derive(Object)]
    struct A {
        a: i32,
        b: B,
    }

    #[derive(Object)]
    struct B {
        c: i64,
    }

    let mut registry = Registry::default();
    A::register(&mut registry);

    let meta_a = registry.schemas.remove("A").unwrap();
    let meta_b = registry.schemas.remove("B").unwrap();

    assert_eq!(meta_a.properties[0].0, "a");
    assert_eq!(meta_a.properties[0].1.unwrap_inline().ty, "integer");
    assert_eq!(meta_a.properties[0].1.unwrap_inline().format, Some("int32"));
    assert_eq!(meta_a.properties[1].1.unwrap_reference(), "B");

    assert_eq!(meta_b.properties[0].0, "c");
    assert_eq!(meta_b.properties[0].1.unwrap_inline().ty, "integer");
    assert_eq!(meta_b.properties[0].1.unwrap_inline().format, Some("int64"));
}

#[test]
fn description() {
    /// A
    ///
    /// AB
    /// CDE
    #[derive(Object)]
    struct Obj {
        a: i32,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.title, Some("A"));
    assert_eq!(meta.description, Some("AB\nCDE"));
}

#[test]
fn field_description() {
    #[derive(Object)]
    struct Obj {
        /// A
        ///
        /// AB
        /// CDE
        a: i32,
    }

    let meta = get_meta::<Obj>();
    let field_meta = meta.properties[0].1.unwrap_inline();
    assert_eq!(field_meta.title, Some("A"));
    assert_eq!(field_meta.description, Some("AB\nCDE"));
}

#[test]
fn field_default() {
    #[derive(Object, Debug, Eq, PartialEq)]
    struct Obj {
        #[oai(default)]
        a: i32,
        #[oai(default = "default_b")]
        b: i32,
        #[oai(default = "default_c")]
        c: Option<i32>,
    }

    fn default_b() -> i32 {
        100
    }

    fn default_c() -> Option<i32> {
        Some(200)
    }

    let meta = get_meta::<Obj>();

    let field_meta = meta.properties[0].1.unwrap_inline();
    assert_eq!(field_meta.default, Some(json!(0)));

    let field_meta = meta.properties[1].1.unwrap_inline();
    assert_eq!(field_meta.default, Some(json!(100)));

    let field_meta = meta.properties[2].1.unwrap_inline();
    assert_eq!(field_meta.default, Some(json!(200)));

    assert_eq!(
        Obj::parse_from_json(json!({
            "a": 1,
        }))
        .unwrap(),
        Obj {
            a: 1,
            b: 100,
            c: Some(200)
        }
    );

    assert_eq!(
        Obj::parse_from_json(json!({})).unwrap(),
        Obj {
            a: 0,
            b: 100,
            c: Some(200)
        }
    );

    assert_eq!(
        Obj::parse_from_json(json!({
            "a": 33,
            "b": 44,
            "c": 55,
        }))
        .unwrap(),
        Obj {
            a: 33,
            b: 44,
            c: Some(55)
        }
    );
}

#[test]
fn serde() {
    #[derive(Object, Debug, Eq, PartialEq)]
    struct Obj {
        a: i32,
    }

    assert_eq!(
        serde_json::to_value(&Obj { a: 10 }).unwrap(),
        json!({ "a": 10 })
    );
    assert_eq!(
        serde_json::from_value::<Obj>(json!({ "a": 10 })).unwrap(),
        Obj { a: 10 }
    );
}

#[test]
fn serde_generic() {
    #[derive(Object, Debug, Eq, PartialEq)]
    #[oai(concrete(name = "Obj", params(i32)))]
    struct Obj<T: ParseFromJSON + ToJSON> {
        a: T,
    }

    assert_eq!(
        serde_json::to_value(&Obj::<i32> { a: 10 }).unwrap(),
        json!({ "a": 10 })
    );
    assert_eq!(
        serde_json::from_value::<Obj<i32>>(json!({ "a": 10 })).unwrap(),
        Obj { a: 10 }
    );
}

#[test]
fn read_only() {
    #[derive(Debug, Object, PartialEq)]
    struct Obj {
        #[oai(read_only)]
        id: i32,
        value: i32,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.properties[0].0, "id");
    assert!(meta.properties[0].1.unwrap_inline().read_only);

    assert_eq!(
        serde_json::from_value::<Obj>(serde_json::json!({
            "value": 100,
        }))
        .unwrap(),
        Obj { id: 0, value: 100 }
    );

    assert_eq!(
        serde_json::to_value(Obj { id: 99, value: 100 }).unwrap(),
        serde_json::json!({
            "id": 99,
            "value": 100,
        })
    );

    assert_eq!(
        serde_json::from_value::<Obj>(serde_json::json!({
            "id": 99,
            "value": 100,
        }))
        .unwrap_err()
        .to_string(),
        r#"failed to parse "Obj": properties `id` is read only."#,
    );
}

#[test]
fn write_only() {
    #[derive(Debug, Object, PartialEq)]
    struct Obj {
        id: i32,
        #[oai(write_only)]
        value: i32,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.properties[1].0, "value");
    assert!(meta.properties[1].1.unwrap_inline().write_only);

    assert_eq!(
        serde_json::from_value::<Obj>(serde_json::json!({
            "id": 99,
            "value": 100,
        }))
        .unwrap(),
        Obj { id: 99, value: 100 }
    );

    assert_eq!(
        serde_json::to_value(Obj { id: 99, value: 100 }).unwrap(),
        serde_json::json!({
            "id": 99,
        })
    );
}

#[test]
fn inline_fields() {
    #[derive(Object)]
    struct Obj {
        /// Inner Obj
        #[oai(default)]
        inner_obj: InlineObj,
        /// Inner Enum
        #[oai(default)]
        inner_enum: InlineEnum,
    }

    #[derive(Object)]
    struct InlineObj {
        v: i32,
    }

    impl Default for InlineObj {
        fn default() -> Self {
            Self { v: 100 }
        }
    }

    #[derive(Enum)]
    enum InlineEnum {
        A,
        B,
        C,
    }

    impl Default for InlineEnum {
        fn default() -> Self {
            Self::B
        }
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.properties[0].0, "innerObj");

    let meta_inner_obj = meta.properties[0].1.unwrap_inline();
    assert_eq!(
        meta_inner_obj.all_of[0],
        MetaSchemaRef::Reference("InlineObj")
    );
    assert_eq!(
        meta_inner_obj.all_of[1],
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            title: Some("Inner Obj"),
            default: Some(serde_json::json!({
                "v": 100,
            })),
            ..MetaSchema::ANY
        }))
    );

    let meta_inner_enum = meta.properties[1].1.unwrap_inline();
    assert_eq!(
        meta_inner_enum.all_of[0],
        MetaSchemaRef::Reference("InlineEnum")
    );
    assert_eq!(
        meta_inner_enum.all_of[1],
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            title: Some("Inner Enum"),
            default: Some(serde_json::json!("B")),
            ..MetaSchema::ANY
        }))
    );
}

#[test]
fn inline() {
    #[derive(Object)]
    #[oai(inline)]
    struct Obj {
        a: i32,
    }

    let schema_ref = Obj::schema_ref();
    let meta: &MetaSchema = schema_ref.unwrap_inline();
    assert_eq!(meta.properties[0].0, "a");

    #[derive(Object)]
    #[oai(inline)]
    struct ObjGeneric<T: ParseFromJSON + ToJSON> {
        a: T,
    }

    let schema_ref = ObjGeneric::<String>::schema_ref();
    let meta: &MetaSchema = schema_ref.unwrap_inline();
    assert_eq!(meta.properties[0].0, "a");
    assert_eq!(meta.properties[0].1.unwrap_inline().ty, "string");
}

#[test]
#[should_panic]
fn duplicate_name() {
    #[derive(Object)]
    struct ObjA {
        value1: i32,
    }

    mod t {
        use super::*;

        #[derive(Object)]
        pub struct ObjA {
            value2: i32,
        }
    }

    let mut registry = Registry::new();
    ObjA::register(&mut registry);
    t::ObjA::register(&mut registry);
}
