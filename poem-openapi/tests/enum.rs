use poem_openapi::{
    registry::{MetaSchemaRef, Registry},
    types::{ParseFromJSON, ToJSON, Type},
    Enum,
};
use serde_json::{json, Value};

#[test]
fn meta_enum_items() {
    #[derive(Enum, Debug, Eq, PartialEq)]
    enum MyEnum {
        CreateUser,
        DeleteUser,
    }

    let mut registry = Registry::new();
    MyEnum::register(&mut registry);
    let meta = registry.schemas.remove("MyEnum").unwrap();
    assert_eq!(
        meta.enum_items,
        vec![json!("CREATE_USER"), json!("DELETE_USER")]
    );
}

#[test]
fn rename() {
    #[derive(Enum, Debug, Eq, PartialEq)]
    #[oai(name = "AAA")]
    enum MyEnum {
        CreateUser,
        DeleteUser,
    }

    let mut registry = Registry::new();
    MyEnum::register(&mut registry);
    let meta = registry.schemas.remove("AAA").unwrap();
    assert_eq!(meta.ty, "AAA");
    assert_eq!(MyEnum::schema_ref(), MetaSchemaRef::Reference("AAA"));
}

#[test]
fn rename_items() {
    #[derive(Enum, Debug, Eq, PartialEq)]
    #[oai(rename_items = "camelCase")]
    enum MyEnum {
        CreateUser,
        DeleteUser,
    }

    assert_eq!(
        MyEnum::parse_from_json(Value::String("createUser".to_string())).unwrap(),
        MyEnum::CreateUser
    );

    assert_eq!(
        MyEnum::parse_from_json(Value::String("deleteUser".to_string())).unwrap(),
        MyEnum::DeleteUser
    );

    assert_eq!(
        MyEnum::CreateUser.to_json(),
        Value::String("createUser".to_string())
    );
    assert_eq!(
        MyEnum::DeleteUser.to_json(),
        Value::String("deleteUser".to_string())
    );
}

#[test]
fn rename_item() {
    #[derive(Enum, Debug, Eq, PartialEq)]
    enum MyEnum {
        CreateUser,
        #[oai(name = "delete_user")]
        DeleteUser,
    }

    assert_eq!(
        MyEnum::parse_from_json(Value::String("CREATE_USER".to_string())).unwrap(),
        MyEnum::CreateUser
    );

    assert_eq!(
        MyEnum::parse_from_json(Value::String("delete_user".to_string())).unwrap(),
        MyEnum::DeleteUser
    );

    assert_eq!(
        MyEnum::CreateUser.to_json(),
        Value::String("CREATE_USER".to_string())
    );
    assert_eq!(
        MyEnum::DeleteUser.to_json(),
        Value::String("delete_user".to_string())
    );
}

#[test]
fn default() {
    #[derive(Enum, Debug, Eq, PartialEq)]
    #[oai(default)]
    enum MyEnum {
        CreateUser,
        DeleteUser,
    }

    impl Default for MyEnum {
        fn default() -> Self {
            MyEnum::DeleteUser
        }
    }

    let mut registry = Registry::new();
    MyEnum::register(&mut registry);
    let meta = registry.schemas.remove("MyEnum").unwrap();
    assert_eq!(meta.ty, "MyEnum");
    assert_eq!(meta.default, Some(json!("DELETE_USER")));
    assert_eq!(
        MyEnum::parse_from_json(json!(null)).unwrap(),
        MyEnum::DeleteUser
    );
}

#[test]
fn default_func() {
    #[derive(Enum, Debug, Eq, PartialEq)]
    #[oai(default = "default_my_enum")]
    enum MyEnum {
        CreateUser,
        DeleteUser,
    }

    fn default_my_enum() -> MyEnum {
        MyEnum::DeleteUser
    }

    let mut registry = Registry::new();
    MyEnum::register(&mut registry);
    let meta = registry.schemas.remove("MyEnum").unwrap();
    assert_eq!(meta.ty, "MyEnum");
    assert_eq!(meta.default, Some(json!("DELETE_USER")));
    assert_eq!(
        MyEnum::parse_from_json(json!(null)).unwrap(),
        MyEnum::DeleteUser
    );
}
