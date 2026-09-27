//! Hash the simulation's source into the binary as a build fingerprint.
//!
//! `rules_digest` covers the content constants and only changes when someone
//! remembers to change them.  Behaviour lives in `bw_sim`, so two builds can
//! share a digest and still disagree about what a tick does.  The fingerprint
//! below is the source those two builds were made from, and the lockstep
//! handshake refuses a pair that disagrees on it before the match starts
//! rather than a second later as a desync.
//!
//! It is deliberately not part of `rules_digest`: saves and replays are keyed
//! by the digest, and mixing the fingerprint in would make every recording
//! unreadable on the next edit to any source file.

use std::path::{Path, PathBuf};

fn main() {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf);
    let mut hasher = blake3::Hasher::new();
    let mut ok = true;
    for name in ["bw_core", "bw_content", "bw_sim"] {
        let Some(dir) = crates.as_ref().map(|c| c.join(name).join("src")) else {
            ok = false;
            break;
        };
        println!("cargo:rerun-if-changed={}", dir.display());
        if !hash_dir(&dir, &dir, &mut hasher) {
            ok = false;
            break;
        }
    }
    // An unreadable tree gives every build the same word, so the handshake
    // check becomes a no-op instead of refusing every match.
    let fingerprint = if ok {
        hasher.finalize().to_hex()[..16].to_string()
    } else {
        "unknown".to_string()
    };
    println!("cargo:rustc-env=BW_BUILD_FINGERPRINT={fingerprint}");
}

/// Hash every file under `dir` by its path relative to `root` and by its
/// contents, in a stable order.  The path is relative so that two checkouts
/// in different directories agree.
fn hash_dir(root: &Path, dir: &Path, hasher: &mut blake3::Hasher) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            if !hash_dir(root, &path, hasher) {
                return false;
            }
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            return false;
        };
        let relative = path.strip_prefix(root).unwrap_or(&path);
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(&bytes);
    }
    true
}
