use poem_openapi::{
    registry::{MetaExternalDocument, MetaSchema, MetaSchemaRef, Registry},
    types::{Example, ParseFromJSON, ToJSON, Type},
    Enum, NewType, Object, OpenApi,
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
fn generics() {
    #[derive(Object)]
    struct Obj<T1: ParseFromJSON + ToJSON, T2: ParseFromJSON + ToJSON> {
        create_user: T1,
        delete_user: T2,
    }

    assert_eq!(
        <Obj<i32, i64>>::name(),
        "Obj<integer(int32), integer(int64)>"
    );
    let meta = get_meta::<Obj<i32, i64>>();
    assert_eq!(meta.properties[0].1.unwrap_inline().ty, "integer");
    assert_eq!(meta.properties[0].1.unwrap_inline().format, Some("int32"));

    assert_eq!(meta.properties[1].1.unwrap_inline().ty, "integer");
    assert_eq!(meta.properties[1].1.unwrap_inline().format, Some("int64"));

    assert_eq!(
        <Obj<f32, f64>>::name(),
        "Obj<number(float), number(double)>"
    );
    let meta = get_meta::<Obj<f32, f64>>();
    assert_eq!(meta.properties[0].1.unwrap_inline().ty, "number");
    assert_eq!(meta.properties[0].1.unwrap_inline().format, Some("float"));

    assert_eq!(meta.properties[1].1.unwrap_inline().ty, "number");
    assert_eq!(meta.properties[1].1.unwrap_inline().format, Some("double"));
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
        Obj { id: 99, value: 100 }.to_json(),
        Some(serde_json::json!({
            "id": 99,
            "value": 100,
        }))
    );

    assert_eq!(
        Obj::parse_from_json(Some(serde_json::json!({
            "id": 99,
            "value": 100,
        })))
        .unwrap_err()
        .into_message(),
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
        Obj::parse_from_json(Some(serde_json::json!({
            "id": 99,
            "value": 100,
        })))
        .unwrap(),
        Obj { id: 99, value: 100 }
    );

    assert_eq!(
        Obj { id: 99, value: 100 }.to_json(),
        Some(serde_json::json!({}))
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
        Obj::parse_from_json(Some(json!({
            "a": 10,
        })))
        .unwrap(),
        Obj { a: 10, b: 0 }
    );

    assert_eq!(
        Obj { a: 10, b: 0 }.to_json(),
        Some(json!({
            "a": 10,
        }))
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
    assert_eq!(meta.description, Some("A\n\nAB\nCDE"));
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
    assert_eq!(field_meta.description, Some("A\n\nAB\nCDE"));
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
        Obj::parse_from_json(Some(json!({
            "a": 1,
        })))
        .unwrap(),
        Obj {
            a: 1,
            b: 100,
            c: Some(200)
        }
    );

    assert_eq!(
        Obj::parse_from_json(Some(json!({}))).unwrap(),
        Obj {
            a: 0,
            b: 100,
            c: Some(200)
        }
    );

    assert_eq!(
        Obj::parse_from_json(Some(json!({
            "a": 33,
            "b": 44,
            "c": 55,
        })))
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

    assert_eq!(Obj { a: 10 }.to_json(), Some(json!({ "a": 10 })));
    assert_eq!(
        Obj::parse_from_json(Some(json!({ "a": 10 }))).unwrap(),
        Obj { a: 10 }
    );
}

#[test]
fn serde_generic() {
    #[derive(Object, Debug, Eq, PartialEq)]
    struct Obj<T: ParseFromJSON + ToJSON> {
        a: T,
    }

    assert_eq!(Obj::<i32> { a: 10 }.to_json(), Some(json!({ "a": 10 })));
    assert_eq!(
        <Obj<i32>>::parse_from_json(Some(json!({ "a": 10 }))).unwrap(),
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
        Obj::parse_from_json(Some(serde_json::json!({
            "value": 100,
        })))
        .unwrap(),
        Obj { id: 0, value: 100 }
    );

    assert_eq!(
        Obj { id: 99, value: 100 }.to_json(),
        Some(serde_json::json!({
            "id": 99,
            "value": 100,
        }))
    );

    assert_eq!(
        Obj::parse_from_json(Some(serde_json::json!({
            "id": 99,
            "value": 100,
        })))
        .unwrap_err()
        .into_message(),
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
        Obj::parse_from_json(Some(serde_json::json!({
            "id": 99,
            "value": 100,
        })))
        .unwrap(),
        Obj { id: 99, value: 100 }
    );

    assert_eq!(
        Obj { id: 99, value: 100 }.to_json(),
        Some(serde_json::json!({
            "id": 99,
        }))
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
    assert_eq!(meta.properties[0].0, "inner_obj");

    let meta_inner_obj = meta.properties[0].1.unwrap_inline();
    assert_eq!(
        meta_inner_obj.all_of[0],
        MetaSchemaRef::Reference("InlineObj".to_string())
    );
    assert_eq!(
        meta_inner_obj.all_of[1],
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            description: Some("Inner Obj"),
            default: Some(serde_json::json!({
                "v": 100,
            })),
            ..MetaSchema::ANY
        }))
    );

    let meta_inner_enum = meta.properties[1].1.unwrap_inline();
    assert_eq!(
        meta_inner_enum.all_of[0],
        MetaSchemaRef::Reference("InlineEnum".to_string())
    );
    assert_eq!(
        meta_inner_enum.all_of[1],
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            description: Some("Inner Enum"),
            default: Some(serde_json::json!("B")),
            ..MetaSchema::ANY
        }))
    );
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

#[test]
fn deny_unknown_fields() {
    #[derive(Object, Debug, Eq, PartialEq)]
    #[oai(deny_unknown_fields)]
    struct Obj {
        a: i32,
        b: i32,
    }

    assert_eq!(
        Obj::parse_from_json(Some(json!({
            "a": 1,
            "b": 2,
        })))
        .unwrap(),
        Obj { a: 1, b: 2 }
    );

    assert_eq!(
        Obj::parse_from_json(Some(json!({
            "a": 1,
            "b": 2,
            "c": 3,
        })))
        .unwrap_err()
        .into_message(),
        "failed to parse \"Obj\": unknown field `c`."
    );
}

#[test]
fn required_fields() {
    #[derive(Object)]
    struct Obj {
        a: i32,
        #[oai(default)]
        b: i32,
        c: Option<i32>,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.required, vec!["a"]);
}

#[tokio::test]
async fn external_docs() {
    #[derive(Object)]
    #[oai(
        external_docs = "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
    )]
    struct Obj {
        a: i32,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(
        meta.external_docs,
        Some(MetaExternalDocument {
            url: "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
                .to_string(),
            description: None
        })
    );
}

#[test]
fn issue_171() {
    #[derive(NewType)]
    #[oai(from_parameter = false, to_header = false, from_multipart = false)]
    pub struct Schema(Vec<SchemaItem>);

    #[derive(Object)]
    #[oai(rename_all = "snake_case")]
    pub struct SchemaItem {
        pub properties: Option<Schema>,
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn a(&self, _item: poem_openapi::payload::Json<SchemaItem>) {}
    }

    let _ = poem_openapi::OpenApiService::new(Api, "a", "1.0").spec();
}

#[test]
fn flatten_field() {
    #[derive(Object, Debug, Eq, PartialEq)]
    struct Obj1 {
        a: i32,
        b: i32,
    }

    #[derive(Object, Debug, Eq, PartialEq)]
    struct Obj {
        #[oai(flatten)]
        obj1: Obj1,
        c: i32,
    }

    let meta = get_meta::<Obj>();
    assert_eq!(meta.required, vec!["a", "b", "c"]);

    assert_eq!(meta.properties[0].0, "a");
    assert_eq!(meta.properties[1].0, "b");
    assert_eq!(meta.properties[2].0, "c");

    let obj = Obj {
        obj1: Obj1 { a: 100, b: 200 },
        c: 300,
    };

    assert_eq!(obj.to_json(), Some(json!({"a": 100, "b": 200, "c": 300})));
    assert_eq!(
        Obj::parse_from_json(Some(json!({"a": 100, "b": 200, "c":
300})))
        .unwrap(),
        obj
    );
}

#[test]
fn remote() {
    mod remote_types {
        #[derive(Debug, Eq, PartialEq)]
        pub struct InternalMyObj {
            pub a: i32,
            pub b: String,
        }
    }

    #[derive(Debug, Object, Eq, PartialEq)]
    #[oai(remote = "remote_types::InternalMyObj")]
    struct MyObj {
        a: i32,
        b: String,
    }

    assert_eq!(
        Into::<MyObj>::into(remote_types::InternalMyObj {
            a: 100,
            b: "abc".to_string()
        }),
        MyObj {
            a: 100,
            b: "abc".to_string()
        }
    );

    assert_eq!(
        Into::<remote_types::InternalMyObj>::into(MyObj {
            a: 100,
            b: "abc".to_string()
        }),
        remote_types::InternalMyObj {
            a: 100,
            b: "abc".to_string()
        }
    );
}

#[test]
fn skip_serializing_if_is_none() {
    #[derive(Debug, Object, Eq, PartialEq)]
    #[oai(skip_serializing_if_is_none)]
    struct MyObj1 {
        a: i32,
        b: Option<i32>,
        c: Option<i32>,
    }

    let obj = MyObj1 {
        a: 100,
        b: None,
        c: Some(200),
    };
    assert_eq!(obj.to_json(), Some(json!({"a": 100, "c": 200})));

    #[derive(Debug, Object, Eq, PartialEq)]
    struct MyObj2 {
        #[oai(skip_serializing_if_is_none)]
        a: i32,
        #[oai(skip_serializing_if_is_none)]
        b: Option<i32>,
        #[oai(skip_serializing_if_is_none)]
        c: Option<i32>,
    }

    let obj = MyObj2 {
        a: 100,
        b: None,
        c: Some(200),
    };
    assert_eq!(obj.to_json(), Some(json!({"a": 100, "c": 200})));
}

#[test]
fn skip_serializing_if_is_empty() {
    #[derive(Debug, Object, Eq, PartialEq)]
    #[oai(skip_serializing_if_is_empty)]
    struct MyObj1 {
        a: Vec<i32>,
        b: Vec<i32>,
    }

    let obj = MyObj1 {
        a: vec![1, 2, 3],
        b: vec![],
    };
    assert_eq!(obj.to_json(), Some(json!({"a": [1, 2, 3]})));

    #[derive(Debug, Object, Eq, PartialEq)]
    struct MyObj2 {
        #[oai(skip_serializing_if_is_empty)]
        a: Vec<i32>,
        #[oai(skip_serializing_if_is_empty)]
        b: Vec<i32>,
    }

    let obj = MyObj2 {
        a: vec![1, 2, 3],
        b: vec![],
    };
    assert_eq!(obj.to_json(), Some(json!({"a": [1, 2, 3]})));
}

#[test]
fn skip_serializing_if() {
    fn check_i32(n: &i32) -> bool {
        *n == 100
    }

    #[derive(Debug, Object, Eq, PartialEq)]
    struct MyObj {
        #[oai(skip_serializing_if = "check_i32")]
        a: i32,
        #[oai(skip_serializing_if = "check_i32")]
        b: i32,
    }

    let obj = MyObj { a: 100, b: 200 };
    assert_eq!(obj.to_json(), Some(json!({"b": 200})));
}

#[test]
fn example() {
    #[derive(Object)]
    #[oai(example)]
    struct Obj {
        a: i32,
        b: String,
    }

    impl Example for Obj {
        fn example() -> Self {
            Obj {
                a: 100,
                b: "abc".to_string(),
            }
        }
    }

    let meta = get_meta::<Obj>();
    assert_eq!(
        meta.example,
        Some(json!({
            "a": 100,
            "b": "abc",
        }))
    );
}

#[test]
fn example_generics() {
    #[derive(Object)]
    #[oai(example)]
    struct Obj<T: ParseFromJSON + ToJSON> {
        value: T,
    }

    impl Example for Obj<i32> {
        fn example() -> Self {
            Obj { value: 100 }
        }
    }

    impl Example for Obj<String> {
        fn example() -> Self {
            Obj {
                value: "abc".to_string(),
            }
        }
    }

    let meta = get_meta::<Obj<i32>>();
    assert_eq!(
        meta.example,
        Some(json!({
            "value": 100,
        }))
    );
}

#[test]
fn object_default() {
    #[derive(Object, Debug, Eq, PartialEq)]
    #[oai(default)]
    struct Obj {
        a: i32,
        b: String,
    }

    impl Default for Obj {
        fn default() -> Self {
            Self {
                a: 100,
                b: "abc".to_string(),
            }
        }
    }

    let meta = get_meta::<Obj>();

    let field_meta = meta.properties[0].1.unwrap_inline();
    assert_eq!(field_meta.default, Some(json!(100)));

    let field_meta = meta.properties[1].1.unwrap_inline();
    assert_eq!(field_meta.default, Some(json!("abc")));

    assert_eq!(
        Obj::parse_from_json(Some(json!({}))).unwrap(),
        Obj {
            a: 100,
            b: "abc".to_string()
        }
    );

    assert_eq!(
        Obj::parse_from_json(Some(json!({
            "a": 1,
        })))
        .unwrap(),
        Obj {
            a: 1,
            b: "abc".to_string()
        }
    );

    assert_eq!(
        Obj::parse_from_json(Some(json!({
            "a": 300,
            "b": "def",
        })))
        .unwrap(),
        Obj {
            a: 300,
            b: "def".to_string()
        }
    );
}

#[test]
fn object_default_by_function() {
    #[derive(Object, Debug, Eq, PartialEq)]
    #[oai(default = "default_obj")]
    struct Obj {
        a: i32,
        b: String,
    }

    fn default_obj() -> Obj {
        Obj {
            a: 100,
            b: "abc".to_string(),
        }
    }

    let meta = get_meta::<Obj>();

    let field_meta = meta.properties[0].1.unwrap_inline();
    assert_eq!(field_meta.default, Some(json!(100)));

    let field_meta = meta.properties[1].1.unwrap_inline();
    assert_eq!(field_meta.default, Some(json!("abc")));

    assert_eq!(
        Obj::parse_from_json(Some(json!({}))).unwrap(),
        Obj {
            a: 100,
            b: "abc".to_string()
        }
    );

    assert_eq!(
        Obj::parse_from_json(Some(json!({
            "a": 1,
        })))
        .unwrap(),
        Obj {
            a: 1,
            b: "abc".to_string()
        }
    );

    assert_eq!(
        Obj::parse_from_json(Some(json!({
            "a": 300,
            "b": "def",
        })))
        .unwrap(),
        Obj {
            a: 300,
            b: "def".to_string()
        }
    );
}

#[test]
fn object_default_override_by_field() {
    #[derive(Object, Debug, Eq, PartialEq)]
    #[oai(default)]
    struct Obj {
        #[oai(default = "default_a")]
        a: i32,
        b: String,
    }

    fn default_a() -> i32 {
        300
    }

    impl Default for Obj {
        fn default() -> Self {
            Self {
                a: 100,
                b: "abc".to_string(),
            }
        }
    }

    let meta = get_meta::<Obj>();

    let field_meta = meta.properties[0].1.unwrap_inline();
    assert_eq!(field_meta.default, Some(json!(300)));

    let field_meta = meta.properties[1].1.unwrap_inline();
    assert_eq!(field_meta.default, Some(json!("abc")));

    assert_eq!(
        Obj::parse_from_json(Some(json!({}))).unwrap(),
        Obj {
            a: 300,
            b: "abc".to_string()
        }
    );

    assert_eq!(
        Obj::parse_from_json(Some(json!({
            "a": 1,
        })))
        .unwrap(),
        Obj {
            a: 1,
            b: "abc".to_string()
        }
    );

    assert_eq!(
        Obj::parse_from_json(Some(json!({
            "a": 500,
            "b": "def",
        })))
        .unwrap(),
        Obj {
            a: 500,
            b: "def".to_string()
        }
    );
}
