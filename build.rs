use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR should be set by cargo");
    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION should be set by cargo");
    let version_path = Path::new(&out_dir).join("version.rs");
    fs::write(
        &version_path,
        format!("pub const VERSION: &str = \"{version}\";\n"),
    )
    .expect("failed to write version file");

    // Make the version available as a compile-time env var for the CLI and
    // upgrade command, and re-run the build script when the manifest changes so
    // the generated version file is updated and any source that includes it is
    // recompiled.
    println!("cargo:rustc-env=TODORK_VERSION={version}");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
