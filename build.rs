fn main() {
    // Ensure the binary is recompiled when the package version changes so
    // env!("CARGO_PKG_VERSION")-derived strings do not become stale across
    // cached builds.
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!(
        "cargo:rustc-env=TODORK_VERSION={}",
        env!("CARGO_PKG_VERSION")
    );
}
