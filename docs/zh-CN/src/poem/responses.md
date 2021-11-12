# 响应

所有可以转换为 HTTP 响应 `Response` 的类型都应该实现 `IntoResponse`，它们可以用作处理函数的返回值。

在下面的例子中，`string_response` 和 `status_response` 函数返回 `String` 和 `StatusCode`类型，因为 `Poem` 已经为它们实现了 `IntoResponse` 功能。

`no_response` 函数不返回值。我们也可以认为它的返回类型是`()`，`Poem`也为 `()` 实现 `IntoResponse`，它总是转换为 `200 OK`。

```rust
use poem::handler;
use poem::http::StatusCode;

#[handler]
fn string_response() -> String {
    "hello".to_string()
}

#[handler]
fn status_response() -> StatusCode {}

#[handler]
fn no_response() {}

```

# 内置响应

- **()**

    将状态设置为`OK`，body 为空。

- **&'static str**

    将状态设置为`OK`，将`Content-Type`设置为`text/plain`。字符串用作 body。

- **String**

    将状态设置为`OK`，将`Content-Type`设置为`text/plain`。字符串用作 body。

- **&'static [u8]**

   将状态设置为 `OK`，将 `Content-Type` 设置为 `application/octet-stream`。切片用作响应的 body。

- **Html&lt;T>**

   将状态设置为 `OK`，将 `Content-Type` 设置为 `text/html`. `T` 用作响应的 body。

- **Json&lt;T>**

   将状态设置为 `OK` ，将 `Content-Type` 设置为 `application/json`. 使用 [`serde_json`](https://crates.io/crates/serde_json) 将 `T` 序列化为 json 字符串。

- **Bytes**

   将状态设置为 `OK` ，将 `Content-Type` 设置为 `application/octet-stream`。字节串用作响应的 body。

- **Vec&lt;u8>**

  将状态设置为 `OK` ，将 `Content-Type` 设置为
`application/octet-stream`. vector 的数据用作 body。

- **Body**

  将状态设置为 `OK` 并使用指定的 body。

- **StatusCode**

   将状态设置为指定的状态代码 `StatusCode` ，body 为空。

- **(StatusCode, T)**

   将 `T` 转换为响应并设置指定的状态代码 `StatusCode`。

- **(StatusCode, HeaderMap, T)**

   将 `T` 转换为响应并设置指定的状态代码 `StatusCode`，然后合并指定的`HeaderMap`。

- **Response**

   `Response` 的实现者总是返回自身。

- **Compress&lt;T>**

   调用 `T::into_response` 获取响应，然后使用指定的算法压缩响应 body ，并设置正确的 `Content-Encoding`标头。

- **SSE**

    将状态设置为 `OK` ，将 `Content-Type` 设置为 `text/event-stream`，并带有事件流 body。使用 `SSE::new` 函数来创建它。

## 自定义响应

在下面的示例中，我们包装了一个名为 `PDF` 的响应，它向响应添加了一个 `Content-Type: applicationn/pdf` 标头。

```rust
use poem::{IntoResponse, Response};

struct PDF(Vec<u8>);

impl IntoResponse for PDF {
    fn into_response(self) -> Response { 
        Response::builder()
            .header("Content-Type", "application/pdf")
            .body(self.0)
    }
}
```
