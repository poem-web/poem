use poem_openapi::{
    NewType,
    types::{Example, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ToJSON, Type},
};
use serde_json::json;

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

#[tokio::test]
async fn rename_new_type() {
    #[derive(NewType)]
    #[oai(rename = "TYPE_A")]
    struct TypeA(String);

    assert_eq!(TypeA::name(), "TYPE_A");
}

#[tokio::test]
async fn rename_new_type_using_const() {
    const NEW_NAME: &str = "NEW_NAME";

    #[derive(NewType)]
    #[oai(rename = NEW_NAME)]
    struct TypeA(String);

    assert_eq!(TypeA::name(), NEW_NAME);
}

#[tokio::test]
async fn new_type_validator_string_length() {
    /// A string with length constraints
    #[derive(Debug, NewType)]
    #[oai(validator(min_length = 2, max_length = 10))]
    struct BoundedString(String);

    // Check that the schema contains the validator constraints
    let schema = BoundedString::schema_ref();
    let schema = schema.unwrap_inline();
    assert_eq!(schema.min_length, Some(2));
    assert_eq!(schema.max_length, Some(10));

    // Test valid input
    let result = BoundedString::parse_from_json(Some(json!("hello")));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, "hello");

    // Test too short
    let result = BoundedString::parse_from_json(Some(json!("a")));
    assert!(result.is_err());

    // Test too long
    let result = BoundedString::parse_from_json(Some(json!("this is way too long")));
    assert!(result.is_err());
}

#[tokio::test]
async fn new_type_validator_numeric() {
    /// A number with range constraints
    #[derive(Debug, NewType)]
    #[oai(validator(minimum(value = 0.0), maximum(value = 100.0)))]
    struct Percentage(i32);

    // Check that the schema contains the validator constraints
    let schema = Percentage::schema_ref();
    let schema = schema.unwrap_inline();
    assert_eq!(schema.minimum, Some(0.0));
    assert_eq!(schema.maximum, Some(100.0));

    // Test valid input
    let result = Percentage::parse_from_json(Some(json!(50)));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, 50);

    // Test below minimum
    let result = Percentage::parse_from_json(Some(json!(-1)));
    assert!(result.is_err());

    // Test above maximum
    let result = Percentage::parse_from_json(Some(json!(101)));
    assert!(result.is_err());
}

#[tokio::test]
async fn new_type_validator_pattern() {
    /// An email-like string
    #[derive(Debug, NewType)]
    #[oai(validator(pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"))]
    struct Email(String);

    // Check that the schema contains the pattern
    let schema = Email::schema_ref();
    let schema = schema.unwrap_inline();
    assert!(schema.pattern.is_some());

    // Test valid input
    let result = Email::parse_from_json(Some(json!("test@example.com")));
    assert!(result.is_ok());

    // Test invalid input
    let result = Email::parse_from_json(Some(json!("not-an-email")));
    assert!(result.is_err());
}

#[tokio::test]
async fn new_type_validator_from_parameter() {
    #[derive(Debug, NewType)]
    #[oai(validator(min_length = 2, max_length = 10))]
    struct BoundedString(String);

    // Test valid input
    let result = BoundedString::parse_from_parameter("hello");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, "hello");

    // Test too short
    let result = BoundedString::parse_from_parameter("a");
    assert!(result.is_err());

    // Test too long
    let result = BoundedString::parse_from_parameter("this is way too long");
    assert!(result.is_err());
}

#[tokio::test]
async fn new_type_validator_multiple_of() {
    /// A number that must be a multiple of 5
    #[derive(Debug, NewType)]
    #[oai(validator(multiple_of = 5.0))]
    struct MultipleOfFive(i32);

    // Check that the schema contains the constraint
    let schema = MultipleOfFive::schema_ref();
    let schema = schema.unwrap_inline();
    assert_eq!(schema.multiple_of, Some(5.0));

    // Test valid input
    let result = MultipleOfFive::parse_from_json(Some(json!(15)));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, 15);

    // Test invalid input
    let result = MultipleOfFive::parse_from_json(Some(json!(7)));
    assert!(result.is_err());
}
