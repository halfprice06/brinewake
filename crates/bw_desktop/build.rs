//! On Windows (MinGW) builds, give brinewake.exe its icon. Every other
//! target, and a machine without windres, builds exactly as before.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=windows/brinewake.rc");
    println!("cargo:rerun-if-changed=windows/brinewake.ico");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "windows" || env != "gnu" {
        return;
    }
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    let object = out.join("brinewake-icon.o");
    let windres =
        std::env::var("WINDRES").unwrap_or_else(|_| format!("{arch}-w64-mingw32-windres"));
    let status = Command::new(&windres)
        .current_dir(manifest.join("windows"))
        .arg("brinewake.rc")
        .arg("-O")
        .arg("coff")
        .arg("-o")
        .arg(&object)
        .status();
    match status {
        Ok(s) if s.success() => {
            println!("cargo:rustc-link-arg-bins={}", object.display());
        }
        _ => println!("cargo:warning=no {windres}: brinewake.exe keeps the plain icon"),
    }
}
