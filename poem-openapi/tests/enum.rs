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
        vec![json!("CreateUser"), json!("DeleteUser")]
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
        MyEnum::parse_from_json(Value::String("CreateUser".to_string())).unwrap(),
        MyEnum::CreateUser
    );

    assert_eq!(
        MyEnum::parse_from_json(Value::String("delete_user".to_string())).unwrap(),
        MyEnum::DeleteUser
    );

    assert_eq!(
        MyEnum::CreateUser.to_json(),
        Value::String("CreateUser".to_string())
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

#[test]
fn remote() {
    #[derive(Debug, Eq, PartialEq)]
    enum EnumA {
        A,
        B,
        C,
    }

    #[derive(Debug, Enum, Eq, PartialEq)]
    #[oai(remote = "EnumA")]
    enum EnumB {
        A,
        B,
        C,
    }

    let mut registry = Registry::new();
    EnumB::register(&mut registry);
    let meta = registry.schemas.remove("EnumB").unwrap();
    assert_eq!(meta.enum_items, vec![json!("A"), json!("B"), json!("C")]);

    let b: EnumB = EnumA::A.into();
    assert_eq!(b, EnumB::A);
    let b: EnumB = EnumA::B.into();
    assert_eq!(b, EnumB::B);
    let b: EnumB = EnumA::C.into();
    assert_eq!(b, EnumB::C);

    let a: EnumA = EnumB::A.into();
    assert_eq!(a, EnumA::A);
    let a: EnumA = EnumB::B.into();
    assert_eq!(a, EnumA::B);
    let a: EnumA = EnumB::C.into();
    assert_eq!(a, EnumA::C);
}
