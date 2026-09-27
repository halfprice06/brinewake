//! On Windows (MinGW) builds, give the launcher (shipped as BRINEWAKE.exe)
//! the game's icon, product name and version.

include!("../bw_desktop/windows/resources.rs");

fn main() {
    windows_resources(
        "brinewake-launcher",
        "BRINEWAKE launcher",
        "brinewake-launcher",
        "BRINEWAKE.exe",
    );
}
