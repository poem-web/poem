# OpenAPI

The OpenAPI Specification (OAS) defines a standard, language-agnostic interface to RESTful APIs which allows both humans 
and computers to discover and understand the capabilities of the service without access to source code, documentation, or 
through network traffic inspection. When properly defined, a consumer can understand and interact with the remote service 
with a minimal amount of implementation logic.

`Poem-openapi` is a [OpenAPI](https://swagger.io/specification/) server-side framework based on `Poem`.

Generally, if you want your API to support the OAS, you first need to create an [OpenAPI Definitions](https://swagger.io/specification/), 
and then write the corresponding code according to the definitions, or use `Swagger CodeGen` to generate the boilerplate 
server code. But `Poem-openapi` is different from these two, it allows you to only write Rust business code and use 
procedural macros to automatically generate lots of boilerplate code that conform to the OpenAPI specification.
