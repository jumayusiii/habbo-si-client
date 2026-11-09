use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=resources.rc");
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=../assets/habbo.ico");

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing")).join("resources.o");
    let status = Command::new("windres")
        .arg("resources.rc")
        .arg("-O")
        .arg("coff")
        .arg("-o")
        .arg(&out)
        .status()
        .expect("windres is required to embed the Windows icon and manifest");
    assert!(status.success(), "windres failed");
    println!("cargo:rustc-link-arg={}", out.display());
}
