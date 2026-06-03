use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    tauri_build::build();

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest_dir.join("bin").join("glean-ocr.swift");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bin = out_dir.join("glean-ocr");

    println!("cargo:rerun-if-changed={}", src.display());

    let status = Command::new("swiftc")
        .args([
            "-O",
            "-o",
            bin.to_str().unwrap(),
            src.to_str().unwrap(),
            "-framework",
            "Vision",
            "-framework",
            "AppKit",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:rustc-env=OCR_BIN_PATH={}", bin.display());
        }
        _ => {
            println!("cargo:warning=failed to compile glean-ocr.swift; OCR will be unavailable");
        }
    }
}
