fn main() {
    // libgit2-sys (vendored) calls into advapi32 on Windows
    // (CryptAcquireContext, Reg*, token/SID APIs) but doesn't declare
    // the link dependency itself, so we do it here.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
    }
}
