// Shared by the build scripts of bw_desktop and bw_launcher (through
// include!): on Windows (MinGW) builds, compile windows/brinewake.rc with the
// icon, the product name and this package's version into one executable.
// Every other target, and a machine without windres, builds without them.

/// `bin` is the Cargo binary to link the resources into; `description`,
/// `internal` and `filename` fill FileDescription, InternalName and
/// OriginalFilename.
fn windows_resources(bin: &str, description: &str, internal: &str, filename: &str) {
    use std::path::PathBuf;
    use std::process::Command;
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let dir = manifest.join("../bw_desktop/windows");
    println!("cargo:rerun-if-changed={}", dir.join("brinewake.rc").display());
    println!("cargo:rerun-if-changed={}", dir.join("brinewake.ico").display());
    println!("cargo:rerun-if-changed={}", dir.join("resources.rs").display());
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "windows" || env != "gnu" {
        return;
    }
    let version = std::env::var("CARGO_PKG_VERSION").expect("version");
    let mut commas: Vec<&str> = version.split(['.', '-', '+']).take(3).collect();
    commas.resize(4, "0");
    let script = std::fs::read_to_string(dir.join("brinewake.rc"))
        .expect("windows/brinewake.rc")
        .replace("@VERSION_COMMAS@", &commas.join(","))
        .replace("@VERSION@", &version)
        .replace("@DESCRIPTION@", description)
        .replace("@INTERNAL@", internal)
        .replace("@FILENAME@", filename);
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    let rc = out.join(format!("{internal}.rc"));
    std::fs::write(&rc, script).expect("write the resource script");
    let object = out.join(format!("{internal}-resources.o"));
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let windres =
        std::env::var("WINDRES").unwrap_or_else(|_| format!("{arch}-w64-mingw32-windres"));
    // Run beside the icon, so the script finds it by its plain name.
    let status = Command::new(&windres)
        .current_dir(&dir)
        .arg(&rc)
        .arg("-O")
        .arg("coff")
        .arg("-o")
        .arg(&object)
        .status();
    match status {
        Ok(s) if s.success() => {
            println!("cargo:rustc-link-arg-bin={bin}={}", object.display());
        }
        _ => println!("cargo:warning=no {windres}: {filename} has no icon or version details"),
    }
}
