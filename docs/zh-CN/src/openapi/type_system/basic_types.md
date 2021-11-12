# Basic types

基础类型可以作为请求的参数，请求内容或者请求响应内容。`Poem`定义了一个`Type trait`，实现了该`trait`的类型都是基础类型，它们能在运行时提供一些关于该类型的信息用于生成接口定义文件。

`Poem`为大部分常用类型实现了`Type`trait，你可以直接使用它们，同样也可以自定义新的类型，但你需要对 [Json Schema](https://json-schema.org/) 有一定了解。

下表是 Open API 中的数据类型对应的Rust数据类型（只是一小部分）：

| Open API                                | Rust                              |
|-----------------------------------------|-----------------------------------|
| `{type: "integer", format: "int32" }`   | i32                               |
| `{type: "integer", format: "float32" }` | f32                               |
| `{type: "bool" }`                       | bool                              |
| `{type: "string" }`                     | String, &str                      |
| `{type: "string", format: "binary" }`   | Binary                            |
| `{type: "string", format: "bytes" }`    | Base64                            |
| `{type: "array" }`                      | Vec<T>                            |
