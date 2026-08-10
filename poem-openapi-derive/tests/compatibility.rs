use std::{path::PathBuf, process::Command};

#[test]
fn current_derive_compiles_with_poem_openapi_5_1_16() {
    let derive_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = derive_dir.parent().unwrap();
    let manifest = derive_dir.join("tests/fixtures/runtime-5-1-16/Cargo.toml");
    let target_dir = workspace_dir.join("target/compatibility/runtime-5-1-16");

    let status = Command::new(env!("CARGO"))
        .args(["check", "--quiet", "--manifest-path"])
        .arg(manifest)
        .env("CARGO_TARGET_DIR", target_dir)
        .status()
        .expect("failed to run cargo check for compatibility fixture");

    assert!(status.success());
}
