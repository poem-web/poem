/// Include generated server and client types.
#[macro_export]
macro_rules! include_proto {
    ($package: tt) => {
        include!(concat!(env!("OUT_DIR"), concat!("/", $package, ".rs")));
    };
}

/// Include an encoded `prost_types::FileDescriptorSet` as a `&'static [u8]`.
#[macro_export]
macro_rules! include_file_descriptor_set {
    ($package: tt) => {
        include_bytes!(concat!(env!("OUT_DIR"), concat!("/", $package)))
    };
}
