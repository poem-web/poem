# 处理错误

在 `Poem` 中，我们根据响应状态代码处理错误。当状态码在`400-599`时，我们可以认为处理此请求时出错。

我们可以使用 `EndpointExt::after` 创建一个新的 Endpoint 类型来自定义错误响应。

在下面的例子中，`after`函数用于转换`index`函数的输出，并在发生服务器错误时输出错误响应。

**注意`handler`宏生成的 Endpoint 类型总是`Endpoint<Output=Response>`，即使它返回一个 `Result<T>`.**

```rust
use poem::{handler, Result, Error};
use poem::http::StatusCode;

#[handler]
async fn index() -> Result<()> {
    Err(Error::new(StatusCode::BAD_REQUEST))
}

let ep = index.after(|resp| {
    if resp.status().is_server_error() {
        Response::builder()
            .status(resp.status())
            .body("custom error")
    } else {
        resp
    }
});
```

`EndpointExt::map_to_result` 函数可以帮助我们将任何类型的 Endpoint 转换为 `Endpoint<Output = Response>`，所以我们只需要检查状态码就知道是否发生了错误。

```rust
use poem::endpoint::make;
use poem::{Error, EndpointExt};
use poem::http::StatusCode;

let ep = make(|_| Ok::<(), Error>(Error::new(StatusCode::new(Status::BAD_REQUEST))))
    .map_to_response();
    
let ep = ep.after(|resp| {
    if resp.status().is_server_error() {
        Response::builder()
            .status(resp.status())
            .body("custom error")
    } else {
        resp
    }
});
```

## poem::Error

`poem::Error` 是一个通用的错误类型，它实现了 `From<T: Display>`，所以你可以很容易地使用 `?` 运算符来将任何错误类型转换为它。默认状态代码是`503 Internal Server Error`。 

```rust
use poem::Result;

#[handler]
fn index(data: Vec<u8>) -> Result<i32> {
    let value: i32 = serde_json::from_slice(&data)?;
    Ok(value)
}
```

但是有时候我们不想总是使用 `503` 状态码，`Poem` 提供了一些辅助函数来转换错误类型。

```rust
use poem::{Result, web::Json, error::BadRequest};

#[handler]
fn index(data: Vec<u8>) -> Result<Json<i32>> {
    let value: i32 = serde_json::from_slice(&data).map_err(BadRequest)?;
    Ok(Json(value))
}
```

## 自定义错误类型

有时我们可以使用自定义错误类型来减少重复的代码。

注意：`Poem` 的错误类型通常只需要实现 `IntoResponse`。

```rust
use poem::{
    Response,
    error::ReadBodyError,
    http::StatusCode,
};

enum MyError {
    InvalidValue,
    ReadBodyError(ReadBodyError),
}

impl IntoResponse for MyError {
    fn into_response(self) -> Response {
        match self {
            MyError::InvalidValue => Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body("invalid value"),
            MyError::ReadBodyError(err) => err.into(), // ReadBodyError 已经实现了 `IntoResponse`.
        }
    }
}

#[handler]
fn index(data: Result<String, ReadBodyError>) -> Result<(), MyError> {
    let data = data?;
    if data.len() > 10 {
        return Err(MyError::InvalidValue);
    }
}
```
