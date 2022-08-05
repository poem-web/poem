use std::{
    io::Result,
    path::{Path, PathBuf},
};

use crate::service_generator::PoemServiceGenerator;

#[derive(Debug)]
pub(crate) struct GrpcConfig {
    pub(crate) internal: bool,
    pub(crate) codec_list: Vec<String>,
    pub(crate) emit_package: bool,
    pub(crate) build_client: bool,
    pub(crate) build_server: bool,
    pub(crate) client_middlewares: Vec<String>,
    pub(crate) server_middlewares: Vec<String>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            internal: false,
            codec_list: Default::default(),
            emit_package: Default::default(),
            build_client: true,
            build_server: true,
            client_middlewares: Vec::new(),
            server_middlewares: Vec::new(),
        }
    }
}

/// Configuration options for GRPC code generation.
#[derive(Debug)]
pub struct Config {
    prost_config: prost_build::Config,
    grpc_config: GrpcConfig,
}

impl Default for Config {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Creates a new code generator configuration with default options.
    pub fn new() -> Self {
        Self {
            prost_config: prost_build::Config::default(),
            grpc_config: GrpcConfig::default(),
        }
    }

    #[doc(hidden)]
    pub fn internal(mut self) -> Self {
        self.grpc_config.internal = true;
        self
    }

    /// Configures the output directory where generated Rust files will be
    /// written.
    ///
    /// If unset, defaults to the OUT_DIR environment variable. OUT_DIR is set
    /// by Cargo when executing build scripts, so out_dir typically does not
    /// need to be configured.
    pub fn out_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.prost_config.out_dir(path);
        self
    }

    /// Add a codec.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = poem_grpc_build::Config::new();
    /// config.codec("::poem_grpc::codec::ProstCodec");
    /// config.codec("::poem_grpc::codec::JsonCodec");
    /// ```
    pub fn codec(mut self, path: impl Into<String>) -> Self {
        self.grpc_config.codec_list.push(path.into());
        self
    }

    /// Configure the code generator to generate Rust [`BTreeMap`][1] fields for
    /// Protobuf [`map`][2] type fields.
    ///
    /// # Arguments
    ///
    /// **`paths`** - paths to specific fields, messages, or packages which
    /// should use a Rust `BTreeMap` for Protobuf `map` fields. Paths are
    /// specified in terms of the Protobuf type name (not the generated Rust
    /// type name). Paths with a leading `.` are treated as fully
    /// qualified names. Paths without a leading `.` are treated as relative,
    /// and are suffix matched on the fully qualified field name. If a
    /// Protobuf map field matches any of the paths, a Rust `BTreeMap` field
    /// is generated instead of the default [`HashMap`][3].
    ///
    /// The matching is done on the Protobuf names, before converting to
    /// Rust-friendly casing standards.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = poem_grpc_build::Config::new();
    /// // Match a specific field in a message type.
    /// config.btree_map(&[".my_messages.MyMessageType.my_map_field"]);
    ///
    /// // Match all map fields in a message type.
    /// config.btree_map(&[".my_messages.MyMessageType"]);
    ///
    /// // Match all map fields in a package.
    /// config.btree_map(&[".my_messages"]);
    ///
    /// // Match all map fields. Specially useful in `no_std` contexts.
    /// config.btree_map(&["."]);
    ///
    /// // Match all map fields in a nested message.
    /// config.btree_map(&[".my_messages.MyMessageType.MyNestedMessageType"]);
    ///
    /// // Match all fields named 'my_map_field'.
    /// config.btree_map(&["my_map_field"]);
    ///
    /// // Match all fields named 'my_map_field' in messages named 'MyMessageType', regardless of
    /// // package or nesting.
    /// config.btree_map(&["MyMessageType.my_map_field"]);
    ///
    /// // Match all fields named 'my_map_field', and all fields in the 'foo.bar' package.
    /// config.btree_map(&["my_map_field", ".foo.bar"]);
    /// ```
    ///
    /// [1]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
    /// [2]: https://developers.google.com/protocol-buffers/docs/proto3#maps
    /// [3]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
    pub fn btree_map<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.prost_config.btree_map(paths);
        self
    }

    /// Configure the code generator to generate Rust [`bytes::Bytes`][1] fields
    /// for Protobuf [`bytes`][2] type fields.
    ///
    /// # Arguments
    ///
    /// **`paths`** - paths to specific fields, messages, or packages which
    /// should use a Rust `Bytes` for Protobuf `bytes` fields. Paths are
    /// specified in terms of the Protobuf type name (not the generated Rust
    /// type name). Paths with a leading `.` are treated as fully
    /// qualified names. Paths without a leading `.` are treated as relative,
    /// and are suffix matched on the fully qualified field name. If a
    /// Protobuf map field matches any of the paths, a Rust `Bytes` field is
    /// generated instead of the default [`Vec<u8>`][3].
    ///
    /// The matching is done on the Protobuf names, before converting to
    /// Rust-friendly casing standards.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = poem_grpc_build::Config::new();
    /// // Match a specific field in a message type.
    /// config.bytes(&[".my_messages.MyMessageType.my_bytes_field"]);
    ///
    /// // Match all bytes fields in a message type.
    /// config.bytes(&[".my_messages.MyMessageType"]);
    ///
    /// // Match all bytes fields in a package.
    /// config.bytes(&[".my_messages"]);
    ///
    /// // Match all bytes fields. Specially useful in `no_std` contexts.
    /// config.bytes(&["."]);
    ///
    /// // Match all bytes fields in a nested message.
    /// config.bytes(&[".my_messages.MyMessageType.MyNestedMessageType"]);
    ///
    /// // Match all fields named 'my_bytes_field'.
    /// config.bytes(&["my_bytes_field"]);
    ///
    /// // Match all fields named 'my_bytes_field' in messages named 'MyMessageType', regardless of
    /// // package or nesting.
    /// config.bytes(&["MyMessageType.my_bytes_field"]);
    ///
    /// // Match all fields named 'my_bytes_field', and all fields in the 'foo.bar' package.
    /// config.bytes(&["my_bytes_field", ".foo.bar"]);
    /// ```
    ///
    /// [1]: https://docs.rs/bytes/latest/bytes/struct.Bytes.html
    /// [2]: https://developers.google.com/protocol-buffers/docs/proto3#scalar
    /// [3]: https://doc.rust-lang.org/std/vec/struct.Vec.html
    pub fn bytes<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.prost_config.bytes(paths);
        self
    }

    /// Add additional attribute to matched messages, enums and one-ofs.
    ///
    /// # Arguments
    ///
    /// **`paths`** - a path matching any number of types. It works the same way
    /// as in [`btree_map`](#method.btree_map), just with the field name
    /// omitted.
    ///
    /// **`attribute`** - an arbitrary string to be placed before each matched
    /// type. The expected usage are additional attributes, but anything is
    /// allowed.
    ///
    /// The calls to this method are cumulative. They don't overwrite previous
    /// calls and if a type is matched by multiple calls of the method, all
    /// relevant attributes are added to it.
    ///
    /// For things like serde it might be needed to combine with [field
    /// attributes](#method.field_attribute).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = poem_grpc_build::Config::new();
    /// // Nothing around uses floats, so we can derive real `Eq` in addition to `PartialEq`.
    /// config.type_attribute(".", "#[derive(Eq)]");
    /// // Some messages want to be serializable with serde as well.
    /// config.type_attribute(
    ///     "my_messages.MyMessageType",
    ///     "#[derive(Serialize)] #[serde(rename-all = \"snake_case\")]",
    /// );
    /// config.type_attribute(
    ///     "my_messages.MyMessageType.MyNestedMessageType",
    ///     "#[derive(Serialize)] #[serde(rename-all = \"snake_case\")]",
    /// );
    /// ```
    ///
    /// # Oneof fields
    ///
    /// The `oneof` fields don't have a type name of their own inside Protobuf.
    /// Therefore, the field name can be used both with `type_attribute` and
    /// `field_attribute` ‒ the first is placed before the `enum` type
    /// definition, the other before the field inside corresponding
    /// message `struct`.
    ///
    /// In other words, to place an attribute on the `enum` implementing the
    /// `oneof`, the match would look like
    /// `my_messages.MyMessageType.oneofname`.
    pub fn type_attribute<P, A>(mut self, path: P, attribute: A) -> Self
    where
        P: AsRef<str>,
        A: AsRef<str>,
    {
        self.prost_config.type_attribute(path, attribute);
        self
    }

    /// Add additional attribute to matched fields.
    ///
    /// # Arguments
    ///
    /// **`path`** - a path matching any number of fields. These fields get the
    /// attribute. For details about matching fields see
    /// [`btree_map`](#method.btree_map).
    ///
    /// **`attribute`** - an arbitrary string that'll be placed before each
    /// matched field. The expected usage are additional attributes, usually
    /// in concert with whole-type attributes set with
    /// [`type_attribute`](method.type_attribute), but it is not checked and
    /// anything can be put there.
    ///
    /// Note that the calls to this method are cumulative ‒ if multiple paths
    /// from multiple calls match the same field, the field gets all the
    /// corresponding attributes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = poem_grpc_build::Config::new();
    /// // Prost renames fields named `in` to `in_`. But if serialized through serde,
    /// // they should as `in`.
    /// config.field_attribute("in", "#[serde(rename = \"in\")]");
    /// ```
    pub fn field_attribute(mut self, path: impl AsRef<str>, attribute: impl AsRef<str>) -> Self {
        self.prost_config.field_attribute(path, attribute);
        self
    }

    /// Emits GRPC endpoints with no attached package.
    pub fn disable_package_emission(mut self) -> Self {
        self.grpc_config.emit_package = true;
        self
    }

    /// When set, the `FileDescriptorSet` generated by `protoc` is written to
    /// the provided filesystem path.
    pub fn file_descriptor_set_path(mut self, path: impl AsRef<Path>) -> Self {
        self.prost_config
            .file_descriptor_set_path(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join(path));
        self
    }

    /// Enable or disable gRPC client code generation.
    pub fn build_client(mut self, enable: bool) -> Self {
        self.grpc_config.build_client = enable;
        self
    }

    /// Enable or disable gRPC server code generation.
    pub fn build_server(mut self, enable: bool) -> Self {
        self.grpc_config.build_server = enable;
        self
    }

    /// Apply a middleware to GRPC client
    pub fn client_middleware(mut self, expr: impl Into<String>) -> Self {
        self.grpc_config.client_middlewares.push(expr.into());
        self
    }

    /// Apply a middleware to GRPC server
    pub fn server_middleware(mut self, expr: impl Into<String>) -> Self {
        self.grpc_config.server_middlewares.push(expr.into());
        self
    }

    /// Compile .proto files into Rust files during a Cargo build with
    /// additional code generator configuration options.
    pub fn compile(
        mut self,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> Result<()> {
        self.prost_config
            .service_generator(Box::new(PoemServiceGenerator {
                config: self.grpc_config,
            }))
            .compile_protos(protos, includes)
    }
}
