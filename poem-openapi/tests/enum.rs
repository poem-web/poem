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
    #[oai(rename = "AAA")]
    enum MyEnum {
        CreateUser,
        DeleteUser,
    }

    let mut registry = Registry::new();
    MyEnum::register(&mut registry);
    let meta = registry.schemas.remove("AAA").unwrap();
    assert_eq!(meta.ty, "string");
    assert_eq!(MyEnum::schema_ref(), MetaSchemaRef::Reference("AAA"));
}

#[test]
fn rename_all() {
    #[derive(Enum, Debug, Eq, PartialEq)]
    #[oai(rename_all = "camelCase")]
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
        #[oai(rename = "delete_user")]
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
#[should_panic]
fn duplicate_name() {
    #[derive(Enum)]
    enum EnumA {
        A,
    }

    mod t {
        use super::*;

        #[derive(Enum)]
        pub enum EnumA {
            B,
        }
    }

    let mut registry = Registry::new();
    EnumA::register(&mut registry);
    t::EnumA::register(&mut registry);
}
