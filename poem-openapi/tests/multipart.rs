use std::io::Write;

use poem::{Request, RequestBody};
use poem_openapi::{
    payload::{ParsePayload, Payload},
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        multipart::{JsonField, Upload},
        Binary,
    },
    Enum, Multipart, Object,
};

fn create_multipart_payload(parts: &[(&str, Option<&str>, &[u8])]) -> Vec<u8> {
    let mut data = Vec::new();

    for part in parts {
        data.write_all(b"--X-BOUNDARY\r\n").unwrap();
        match part.1 {
            Some(filename) => data
                .write_all(
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n\r\n",
                        part.0, filename
                    )
                    .as_bytes(),
                )
                .unwrap(),
            None => data
                .write_all(
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                        part.0
                    )
                    .as_bytes(),
                )
                .unwrap(),
        }

        data.write_all(part.2).unwrap();
        data.write_all(b"\r\n").unwrap();
    }

    data.write_all(b"--X-BOUNDARY--\r\n").unwrap();
    data
}

#[tokio::test]
async fn rename_all() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    #[oai(rename_all = "UPPERCASE")]
    struct A {
        name: String,
        file: Binary<Vec<u8>>,
    }

    let data = create_multipart_payload(&[("NAME", None, b"abc"), ("FILE", None, &[1, 2, 3])]);
    let a = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap();
    assert_eq!(
        a,
        A {
            name: "abc".to_string(),
            file: Binary(vec![1, 2, 3])
        }
    )
}

#[tokio::test]
async fn required_fields() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    struct A {
        name: String,
        file: Binary<Vec<u8>>,
    }

    let schema_ref = A::schema_ref();
    let schema: &MetaSchema = schema_ref.unwrap_inline();
    assert_eq!(schema.ty, "object");
    assert_eq!(schema.properties.len(), 2);

    assert_eq!(schema.properties[0].0, "name");
    assert_eq!(schema.properties[0].1.unwrap_inline().ty, "string");

    assert_eq!(schema.properties[1].0, "file");
    assert_eq!(schema.properties[1].1.unwrap_inline().ty, "string");

    assert_eq!(schema.required, &["name", "file"]);

    let data = create_multipart_payload(&[("name", None, b"abc")]);
    let err = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "parse multipart error: field `file` is required"
    );
}

#[tokio::test]
async fn optional_fields() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    struct A {
        name: Option<String>,
        file: Binary<Vec<u8>>,
    }

    let schema_ref = A::schema_ref();
    let schema: &MetaSchema = schema_ref.unwrap_inline();
    assert_eq!(schema.ty, "object");
    assert_eq!(schema.properties.len(), 2);

    assert_eq!(schema.properties[0].0, "name");
    assert_eq!(schema.properties[0].1.unwrap_inline().ty, "string");

    assert_eq!(schema.properties[1].0, "file");
    assert_eq!(schema.properties[1].1.unwrap_inline().ty, "string");
    assert_eq!(
        schema.properties[1].1.unwrap_inline().format,
        Some("binary")
    );

    assert_eq!(schema.required, &["file"]);

    let data = create_multipart_payload(&[("file", None, &[1, 2, 3])]);
    let a = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap();
    assert_eq!(
        a,
        A {
            name: None,
            file: Binary(vec![1, 2, 3])
        }
    )
}

#[tokio::test]
async fn rename_field() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    struct A {
        #[oai(rename = "Name")]
        name: String,
        file: Binary<Vec<u8>>,
    }

    let data = create_multipart_payload(&[("Name", None, b"abc"), ("file", None, &[1, 2, 3])]);
    let a = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap();
    assert_eq!(
        a,
        A {
            name: "abc".to_string(),
            file: Binary(vec![1, 2, 3])
        }
    )
}

#[tokio::test]
async fn skip() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    struct A {
        name: String,
        file: Binary<Vec<u8>>,
        #[oai(skip)]
        value1: i32,
        #[oai(skip)]
        value2: i32,
    }

    let data = create_multipart_payload(&[("name", None, b"abc"), ("file", None, &[1, 2, 3])]);
    let a = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap();
    assert_eq!(
        a,
        A {
            name: "abc".to_string(),
            file: Binary(vec![1, 2, 3]),
            value1: 0,
            value2: 0,
        }
    );
}

#[tokio::test]
async fn upload() {
    #[derive(Multipart, Debug)]
    struct A {
        name: String,
        file: Upload,
    }

    let data =
        create_multipart_payload(&[("name", None, b"abc"), ("file", Some("1.txt"), &[1, 2, 3])]);
    let a = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap();
    assert_eq!(a.name, "abc".to_string());

    assert_eq!(a.file.file_name(), Some("1.txt"));
    assert_eq!(a.file.content_type(), None);
    assert_eq!(a.file.into_vec().await.unwrap(), vec![1, 2, 3]);
}

#[tokio::test]
async fn validator() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    struct A {
        #[oai(validator(max_length = "10"))]
        name: String,
        #[oai(validator(maximum(value = "32")))]
        value: JsonField<i32>,
    }

    let data = create_multipart_payload(&[("name", None, b"abc"), ("value", None, b"20")]);
    let a = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap();
    assert_eq!(a.name, "abc".to_string());
    assert_eq!(a.value, JsonField(20));

    let data = create_multipart_payload(&[("name", None, b"abc"), ("value", None, b"40")]);
    let err = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "parse multipart error: field `value` verification failed. maximum(32, exclusive: false)"
    );
}

#[tokio::test]
async fn default() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    struct A {
        #[oai(default = "default_string")]
        value_string: String,
        #[oai(default = "default_values")]
        value_array: JsonField<Vec<i32>>,
    }

    fn default_string() -> String {
        "asd".to_string()
    }

    fn default_values() -> JsonField<Vec<i32>> {
        JsonField(vec![1, 2, 3])
    }

    let schema_ref = A::schema_ref();
    let schema: &MetaSchema = schema_ref.unwrap_inline();
    assert_eq!(schema.properties[0].0, "valueString");
    assert_eq!(schema.properties[0].1.unwrap_inline().ty, "string");
    assert_eq!(
        schema.properties[0].1.unwrap_inline().default,
        Some("asd".into())
    );

    assert_eq!(schema.properties[1].0, "valueArray");
    assert_eq!(schema.properties[1].1.unwrap_inline().ty, "array");
    assert_eq!(
        schema.properties[1]
            .1
            .unwrap_inline()
            .items
            .as_ref()
            .map(|schema| schema.unwrap_inline().ty),
        Some("integer")
    );
    assert_eq!(
        schema.properties[1].1.unwrap_inline().default,
        Some(vec![1, 2, 3].into())
    );

    let data = create_multipart_payload(&[
        ("valueString", None, b"abc"),
        ("valueArray", None, b"[10, 20, 30]"),
    ]);
    let a = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap();

    assert_eq!(
        a,
        A {
            value_string: "abc".to_string(),
            value_array: JsonField(vec![10, 20, 30]),
        }
    );

    let data = create_multipart_payload(&[]);
    let a = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap();

    assert_eq!(
        a,
        A {
            value_string: "asd".to_string(),
            value_array: JsonField(vec![1, 2, 3]),
        }
    );
}

#[tokio::test]
async fn array() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    struct A {
        value: Vec<String>,
        value2: Vec<String>,
    }

    let schema_ref = A::schema_ref();
    let schema: &MetaSchema = schema_ref.unwrap_inline();
    assert_eq!(schema.properties[0].0, "value");
    assert_eq!(schema.properties[0].1.unwrap_inline().ty, "array");
    assert_eq!(
        schema.properties[0]
            .1
            .unwrap_inline()
            .items
            .as_ref()
            .map(|schema| schema.unwrap_inline().ty),
        Some("string")
    );

    let data = create_multipart_payload(&[
        ("value", None, b"a1"),
        ("value", None, b"a2"),
        ("value", None, b"a3"),
    ]);
    let a = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap();
    assert_eq!(
        a,
        A {
            value: vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
            value2: vec![],
        }
    )
}

#[tokio::test]
async fn repeated_error() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    struct A {
        value: String,
    }

    let data = create_multipart_payload(&[("value", None, b"a1"), ("value", None, b"a2")]);
    let err = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "parse multipart error: failed to parse field `value`: failed to parse \"string\": repeated field"
    );
}

#[test]
fn inline_field() {
    #[derive(Multipart, Debug, PartialEq)]
    struct A {
        /// Inner Obj
        #[oai(default)]
        inner_obj: JsonField<InlineObj>,
        /// Inner Enum
        #[oai(default)]
        inner_enum: InlineEnum,
    }

    #[derive(Object, Debug, PartialEq)]
    struct InlineObj {
        v: i32,
    }

    impl Default for InlineObj {
        fn default() -> Self {
            Self { v: 100 }
        }
    }

    #[derive(Enum, Debug, PartialEq)]
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

    let schema_ref = A::schema_ref();
    let schema: &MetaSchema = schema_ref.unwrap_inline();

    let meta_inner_obj = schema.properties[0].1.unwrap_inline();
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

    let meta_inner_enum = schema.properties[1].1.unwrap_inline();
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

#[tokio::test]
async fn deny_unknown_fields() {
    #[derive(Multipart, Debug, Eq, PartialEq)]
    #[oai(deny_unknown_fields)]
    struct A {
        a: String,
        b: String,
    }

    let data = create_multipart_payload(&[
        ("a", None, b"abc"),
        ("b", None, b"def"),
        ("c", None, b"ghi"),
    ]);
    let err = A::from_request(
        &Request::builder()
            .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
            .finish(),
        &mut RequestBody::new(data.into()),
    )
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "parse multipart error: unknown field `c`");
}
