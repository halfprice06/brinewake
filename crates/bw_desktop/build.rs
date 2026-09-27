//! On Windows (MinGW) builds, give brinewake.exe its icon, product name and
//! version. Every other target, and a machine without windres, builds
//! exactly as before.

include!("windows/resources.rs");

fn main() {
    windows_resources("brinewake", "BRINEWAKE", "brinewake", "brinewake.exe");
}
