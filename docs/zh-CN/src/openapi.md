# OpenAPI

[OpenAPI]((https://swagger.io/specification/))规范为`RESTful API`定义了一个标准的并且与语言无关的接口，它允许人类和计算机在不访问源代码、文档或通过网络流量检查的情况下发现和理解服务的功能。若经良好定义，使调用者可以很容易的理解远程服务并与之交互, 并只需要很少的代码即可实现期望逻辑.

`Poem-openapi`是基于`Poem`的 [OpenAPI](https://swagger.io/specification/) 服务端框架。

通常，如果你希望让你的API支持该规范，首先需要创建一个 [接口定义文件](https://swagger.io/specification/) ，然后再按照接口定义编写对应的代码。或者创建接口定义文件后，用 `Swagger CodeGen` 来生成服务端代码框架。但`Poem-openapi`区别于这两种方法，它让你只需要编写Rust的业务代码，利用过程宏来自动生成符合OpenAPI规范的接口和接口定义文件（这相当于接口的文档）。
