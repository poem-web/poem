use poem_openapi::{
    registry::{MetaSchema, Registry},
    types::{ParseFromJSON, ToJSON, Type},
    Object,
};
use serde_json::json;

fn get_meta<T: Type>() -> MetaSchema {
    let mut registry = Registry::new();
    T::register(&mut registry);
    registry.schemas.remove(&*T::NAME.to_string()).unwrap()
}

#[test]
fn rename() {
    #[derive(Object)]
    #[oai(name = "Abc")]
    struct Obj {
        a: i32,
    }
    assert_eq!(Obj::NAME.to_string(), "Abc");
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

    assert_eq!(<Obj<i32, i64>>::NAME.to_string(), "Obj_i32_i64");
    let meta = get_meta::<Obj<i32, i64>>();
    assert_eq!(meta.properties[0].1.unwrap_inline().ty, "integer");
    assert_eq!(meta.properties[0].1.unwrap_inline().format, Some("int32"));

    assert_eq!(meta.properties[1].1.unwrap_inline().ty, "integer");
    assert_eq!(meta.properties[1].1.unwrap_inline().format, Some("int64"));

    assert_eq!(<Obj<f32, f64>>::NAME.to_string(), "Obj_f32_f64");
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
fn default() {
    #[derive(Object, Debug, Eq, PartialEq, Default)]
    #[oai(default)]
    struct Obj {
        a: i32,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.default, Some(json!(Obj { a: 0 })));

    assert_eq!(Obj::parse_from_json(json!(null)).unwrap(), Obj { a: 0 });
    assert_eq!(
        Obj::parse_from_json(json!({ "a": 88 })).unwrap(),
        Obj { a: 88 }
    );
}

#[test]
fn default_func() {
    #[derive(Object, Debug, Eq, PartialEq)]
    #[oai(default = "default_obj")]
    struct Obj {
        a: i32,
    }

    fn default_obj() -> Obj {
        Obj { a: 88 }
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.default, Some(json!(Obj { a: 88 })));

    assert_eq!(Obj::parse_from_json(json!(null)).unwrap(), Obj { a: 88 });
    assert_eq!(
        Obj::parse_from_json(json!({ "a": 99 })).unwrap(),
        Obj { a: 99 }
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
