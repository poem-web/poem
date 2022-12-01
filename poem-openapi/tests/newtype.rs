use poem_openapi::{
    types::{Example, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ToJSON, Type},
    NewType,
};

#[tokio::test]
async fn new_type() {
    #[derive(NewType)]
    struct MyString(String);

    assert_eq!(MyString::schema_ref(), String::schema_ref());
}

#[tokio::test]
async fn new_type_summary_and_description() {
    /// MyString
    ///
    /// A
    /// B
    /// C
    #[derive(NewType)]
    struct MyString(String);

    let schema = MyString::schema_ref();
    let schema = schema.unwrap_inline();
    assert_eq!(schema.title.as_deref(), Some("MyString"));
    assert_eq!(schema.description, Some("A\nB\nC"));
}

#[tokio::test]
async fn new_type_example() {
    #[derive(NewType)]
    #[oai(example)]
    struct MyString(String);

    impl Example for MyString {
        fn example() -> Self {
            Self("abc".to_string())
        }
    }

    let schema = MyString::schema_ref();
    let schema = schema.unwrap_inline();
    assert_eq!(schema.example, Some("abc".into()));
}

#[tokio::test]
async fn generic() {
    #[derive(NewType)]
    #[oai(to_header = false)]
    struct MyVec<T: ParseFromJSON + ToJSON + ParseFromParameter + ParseFromMultipartField>(Vec<T>);

    let schema = MyVec::<String>::schema_ref();
    let schema = schema.unwrap_inline();
    assert_eq!(schema.ty, "array");
    assert_eq!(
        schema
            .items
            .as_ref()
            .map(|schema| schema.unwrap_inline().ty),
        Some("string")
    );
}
