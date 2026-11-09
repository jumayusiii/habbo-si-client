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

    // The header pill shows the exact build, so the timestamp is stamped in here rather
    // than being a hand-maintained constant.
    let stamp = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-Date -Format 'yyyy-MM-dd HH:mm'",
        ])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| text.len() == 16)
        .unwrap_or_else(|| "2026-11-08 00:00".into());
    let (date, time) = stamp.split_once(' ').unwrap_or(("2026-11-08", "00:00"));
    println!("cargo:rustc-env=HSC_BUILD_DATE={date}");
    println!("cargo:rustc-env=HSC_BUILD_TIME={time}");

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
