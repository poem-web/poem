use std::{fmt::Write, io::Cursor};

use http::{header, header::HeaderName, HeaderMap, HeaderValue};
use mime::Mime;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::Body;

const BOUNDARY_STRING: &str = "__poem_multipart_boundary__";

/// A field in a multipart form.
pub struct TestFormField {
    mime: Option<Mime>,
    name: Option<String>,
    filename: Option<String>,
    headers: HeaderMap,
    data: Body,
}

impl TestFormField {
    /// Create a text field.
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            mime: None,
            name: None,
            filename: None,
            headers: Default::default(),
            data: Body::from(s.into()),
        }
    }

    /// Create a binary field.
    pub fn bytes(s: impl Into<Vec<u8>>) -> Self {
        Self {
            mime: None,
            name: None,
            filename: None,
            headers: Default::default(),
            data: Body::from(s.into()),
        }
    }

    /// Create a field from async reader.
    pub fn async_reader(reader: impl AsyncRead + Send + 'static) -> Self {
        Self {
            mime: None,
            name: None,
            filename: None,
            headers: Default::default(),
            data: Body::from_async_read(reader),
        }
    }

    /// Sets the content type of this field.
    #[must_use]
    pub fn content_type(mut self, mime: impl AsRef<str>) -> Self {
        self.mime = Some(mime.as_ref().parse().expect("valid mime"));
        self
    }

    /// Sets the name of this field.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the field name of this field.
    #[must_use]
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Sets the header value for this field.
    #[must_use]
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into().map_err(|_| ()).expect("valid header name");
        let value = value
            .try_into()
            .map_err(|_| ())
            .expect("valid header value");
        self.headers.append(key, value);
        self
    }
}

/// A multipart/form-data body.
#[derive(Default)]
pub struct TestForm {
    fields: Vec<TestFormField>,
}

impl TestForm {
    /// Create a multipart/form-data body.
    pub fn new() -> TestForm {
        Default::default()
    }

    /// Adds a field.
    #[must_use]
    pub fn field(mut self, field: TestFormField) -> Self {
        self.fields.push(field);
        self
    }

    /// Adds a text field.
    #[must_use]
    pub fn text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push(TestFormField::text(value).name(name));
        self
    }

    /// Adds a binary field.
    #[must_use]
    pub fn bytes(mut self, name: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        self.fields.push(TestFormField::bytes(value).name(name));
        self
    }

    #[inline]
    pub(crate) fn boundary(&self) -> &str {
        BOUNDARY_STRING
    }

    pub(crate) fn into_async_read(self) -> impl AsyncRead + Unpin + Send + 'static {
        let crlf = b"\r\n";
        let sep = b"--";

        let mut stream: Box<dyn AsyncRead + Unpin + Send + 'static> = Box::new(tokio::io::empty());

        for TestFormField {
            mime,
            name,
            filename,
            mut headers,
            data,
        } in self.fields.into_iter()
        {
            let mut content_disposition = String::from("form-data");

            if let Some(name) = name {
                let _ = write!(content_disposition, "; name=\"{}\"", legal_str(name));
            }

            if let Some(filename) = filename {
                let _ = write!(
                    content_disposition,
                    "; filename=\"{}\"",
                    legal_str(filename)
                );
            }

            headers.insert(
                header::CONTENT_DISPOSITION,
                content_disposition.parse().unwrap(),
            );

            if let Some(mime) = mime {
                headers.insert(header::CONTENT_TYPE, mime.to_string().parse().unwrap());
            }

            let mut head = Vec::new();

            head.extend_from_slice(sep);
            head.extend_from_slice(BOUNDARY_STRING.as_bytes());
            head.extend_from_slice(crlf);
            head.extend_from_slice(&gen_headers(&headers));
            head.extend_from_slice(crlf);

            stream = Box::new(
                stream
                    .chain(Cursor::new(head))
                    .chain(data.into_async_read())
                    .chain(Cursor::new(crlf)),
            )
        }

        stream.chain({
            let mut end = Vec::new();
            end.extend_from_slice(sep);
            end.extend_from_slice(BOUNDARY_STRING.as_bytes());
            end.extend_from_slice(sep);
            Cursor::new(end)
        })
    }
}

fn legal_str(s: impl AsRef<str>) -> String {
    s.as_ref()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('r', "\\\r")
        .replace('\n', "\\\n")
}

fn gen_headers(headers: &HeaderMap) -> Vec<u8> {
    let mut data = Vec::new();
    for (k, v) in headers {
        data.extend_from_slice(format!("{}: {}\r\n", k.as_str(), v.to_str().unwrap()).as_bytes());
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler, test::TestClient, web::Multipart};

    #[tokio::test]
    async fn multipart() {
        let mut a = Vec::new();
        TestForm::new()
            .field(TestFormField::text("1"))
            .field(TestFormField::text("2").name("a"))
            .field(TestFormField::text("3").name("b").filename("3.txt"))
            .field(TestFormField::text("4").filename("3.txt"))
            .field(TestFormField::text("5").content_type("text/plain"))
            .field(TestFormField::bytes(vec![1, 2, 3]))
            .field(TestFormField::async_reader(Cursor::new(vec![4, 5, 6])))
            .into_async_read()
            .read_to_end(&mut a)
            .await
            .unwrap();

        #[handler(internal)]
        async fn index(mut multipart: Multipart) {
            let field = multipart.next_field().await.unwrap().unwrap();
            assert!(field.name().is_none());
            assert!(field.file_name().is_none());
            assert!(field.content_type().is_none());
            assert_eq!(field.text().await.unwrap(), "1");

            let field = multipart.next_field().await.unwrap().unwrap();
            assert_eq!(field.name(), Some("a"));
            assert!(field.file_name().is_none());
            assert!(field.content_type().is_none());
            assert_eq!(field.text().await.unwrap(), "2");

            let field = multipart.next_field().await.unwrap().unwrap();
            assert_eq!(field.name(), Some("b"));
            assert_eq!(field.file_name(), Some("3.txt"));
            assert!(field.content_type().is_none());
            assert_eq!(field.text().await.unwrap(), "3");

            let field = multipart.next_field().await.unwrap().unwrap();
            assert!(field.name().is_none());
            assert_eq!(field.file_name(), Some("4.txt"));
            assert!(field.content_type().is_none());
            assert_eq!(field.text().await.unwrap(), "4");

            let field = multipart.next_field().await.unwrap().unwrap();
            assert!(field.name().is_none());
            assert!(field.file_name().is_none());
            assert_eq!(field.content_type(), Some("text/plain"));
            assert_eq!(field.text().await.unwrap(), "5");

            let field = multipart.next_field().await.unwrap().unwrap();
            assert!(field.name().is_none());
            assert!(field.file_name().is_none());
            assert!(field.content_type().is_none());
            assert_eq!(field.bytes().await.unwrap(), vec![1, 2, 3]);

            let field = multipart.next_field().await.unwrap().unwrap();
            assert!(field.name().is_none());
            assert!(field.file_name().is_none());
            assert!(field.content_type().is_none());
            assert_eq!(field.bytes().await.unwrap(), vec![4, 5, 6]);
        }

        let cli = TestClient::new(index);
        let resp = cli
            .post("/")
            .multipart(
                TestForm::new()
                    .field(TestFormField::text("1"))
                    .field(TestFormField::text("2").name("a"))
                    .field(TestFormField::text("3").name("b").filename("3.txt"))
                    .field(TestFormField::text("4").filename("4.txt"))
                    .field(TestFormField::text("5").content_type("text/plain"))
                    .field(TestFormField::bytes(vec![1, 2, 3]))
                    .field(TestFormField::async_reader(Cursor::new(vec![4, 5, 6]))),
            )
            .send()
            .await;
        resp.assert_status_is_ok();
    }
}
