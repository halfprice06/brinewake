//! Checking for, fetching and installing new versions of BRINEWAKE.
//!
//! A release is described by `latest.json` (a [`Manifest`]) and
//! `latest.json.sig`, an ed25519 signature of the manifest's exact bytes by
//! the release key. The launcher trusts a manifest only if the signature
//! checks against [`RELEASE_KEY`], and a download only if its size and
//! BLAKE3 hash match the signed manifest.
//!
//! The game runs from one fixed folder in the player's data folder,
//! `<store>/current/` (the same layout as the game bundled with the
//! launcher, with its `VERSION` file), so firewall rules made for it last
//! across updates. The version before is kept in `<store>/previous/`. On
//! first run the bundled game is copied there.

pub mod font;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The launcher's protocol. A manifest whose `launcher` is higher needs a
/// newer launcher, downloaded by hand from the page.
pub const LAUNCHER: u32 = 1;

/// Where the launcher looks for the newest release, in order: the game's
/// page, then the GitHub repository's latest release (the page serves the
/// same signed files, so either will do).
pub const MANIFEST_URLS: [&str; 2] = [
    "https://danprice.ai/brinewake/latest.json",
    "https://github.com/halfprice06/brinewake/releases/latest/download/latest.json",
];

/// The page players download the game from.
pub const PAGE: &str = "danprice.ai/brinewake";

/// The public half of the release signing key (ed25519, hex). The private
/// half never enters the repository.
pub const RELEASE_KEY: &str = include_str!("release_key.txt");

/// The largest manifest or signature the launcher will read.
const SMALL_LIMIT: u64 = 256 * 1024;

/// The largest game download the launcher will accept.
const DOWNLOAD_LIMIT: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub schema: u32,
    pub version: String,
    pub published: String,
    #[serde(default)]
    pub notes: Vec<String>,
    /// The launcher protocol this release needs.
    pub launcher: u32,
    pub platforms: BTreeMap<String, Platform>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Platform {
    pub label: String,
    /// The game alone, installed by the launcher.
    pub game: Asset,
    /// The whole download (launcher and bundled game) for the page.
    pub download: Asset,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Asset {
    pub url: String,
    pub size: u64,
    pub blake3: String,
}

/// This build's platform key in the manifest.
pub fn platform() -> &'static str {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "macos-arm64"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "windows-x64"
    } else {
        "unsupported"
    }
}

/// `major.minor.patch` as numbers, for comparing versions.
pub fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let mut parts = s.trim().split('.');
    let v = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(v)
}

pub fn decode_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

pub fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Check `signature` (hex) over `bytes` against the public key `key` (hex).
pub fn verify(bytes: &[u8], signature: &str, key: &str) -> Result<(), String> {
    let key: [u8; 32] = decode_hex(key)
        .and_then(|k| k.try_into().ok())
        .ok_or("the release key is malformed")?;
    let signature: [u8; 64] = decode_hex(signature)
        .and_then(|s| s.try_into().ok())
        .ok_or("the signature is malformed")?;
    let key =
        ed25519_dalek::VerifyingKey::from_bytes(&key).map_err(|_| "the release key is invalid")?;
    key.verify_strict(bytes, &ed25519_dalek::Signature::from_bytes(&signature))
        .map_err(|_| "the release signature does not match".to_string())
}

/// Parse a manifest whose signature has been checked.
pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest, String> {
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|e| format!("the release manifest is unreadable: {e}"))?;
    if manifest.schema != 1 {
        return Err(format!("unknown manifest schema {}", manifest.schema));
    }
    if parse_version(&manifest.version).is_none() {
        return Err(format!(
            "bad version {:?} in the manifest",
            manifest.version
        ));
    }
    Ok(manifest)
}

/// One installed (or bundled) game.
#[derive(Clone, Debug, PartialEq)]
pub struct Install {
    pub version: String,
    pub dir: PathBuf,
}

impl Install {
    /// The game's executable inside its folder.
    pub fn exe(&self) -> PathBuf {
        game_exe(&self.dir)
    }

    fn complete(&self) -> bool {
        self.exe().is_file()
    }
}

/// Where the game's executable sits in a game folder.
pub fn game_exe(dir: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        dir.join("BRINEWAKE.app/Contents/MacOS/brinewake")
    } else if cfg!(windows) {
        dir.join("brinewake.exe")
    } else {
        dir.join("brinewake")
    }
}

/// The game shipped with this launcher, if it has one: beside the launcher
/// on Windows (`game/`), in the app's resources on macOS.
pub fn bundled() -> Option<Install> {
    let exe = std::env::current_exe().ok()?;
    let dir = if cfg!(target_os = "macos") {
        exe.parent()?.parent()?.join("Resources/game")
    } else {
        exe.parent()?.join("game")
    };
    let version = fs::read_to_string(dir.join("VERSION"))
        .ok()?
        .trim()
        .to_string();
    parse_version(&version)?;
    let install = Install { version, dir };
    install.complete().then_some(install)
}

/// The installed game: `current/`, and `previous/` from the update before.
pub struct Store {
    pub root: PathBuf,
}

impl Store {
    /// The player's store: `BRINEWAKE_GAME_STORE`, else the platform's data
    /// folder.
    pub fn default_location() -> Option<Self> {
        if let Some(root) = std::env::var_os("BRINEWAKE_GAME_STORE") {
            return Some(Self {
                root: PathBuf::from(root),
            });
        }
        let root = if cfg!(target_os = "macos") {
            PathBuf::from(std::env::var_os("HOME")?)
                .join("Library/Application Support/Brinewake/game")
        } else if cfg!(windows) {
            PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
                .join("Brinewake")
                .join("game")
        } else {
            PathBuf::from(std::env::var_os("HOME")?).join(".local/share/brinewake/game")
        };
        Some(Self { root })
    }

    /// The current game, if it is complete.
    pub fn installed(&self) -> Option<Install> {
        read_install(&self.root.join("current"))
    }

    /// Unpack a verified game archive for `version` and make it current.
    pub fn install(&self, version: &str, archive: &Path) -> Result<Install, String> {
        let incoming = self.incoming()?;
        let result = unpack(archive, &incoming).and_then(|()| match read_install(&incoming) {
            Some(i) if i.version == version => Ok(()),
            Some(i) => Err(format!(
                "the download holds {} instead of {version}",
                i.version
            )),
            None => Err("the download holds no game".into()),
        });
        if let Err(e) = result {
            let _ = fs::remove_dir_all(&incoming);
            return Err(e);
        }
        self.swap_in(&incoming)
    }

    /// Copy the game bundled with the launcher in as current.
    pub fn adopt(&self, bundled: &Install) -> Result<Install, String> {
        let incoming = self.incoming()?;
        if let Err(e) = copy_dir(&bundled.dir, &incoming) {
            let _ = fs::remove_dir_all(&incoming);
            return Err(e);
        }
        // The copy is the launcher's own game, already approved when the
        // player opened the launcher: it should not ask again.
        clear_download_marks(&incoming);
        if read_install(&incoming).is_none() {
            let _ = fs::remove_dir_all(&incoming);
            return Err("the bundled game is incomplete".into());
        }
        self.swap_in(&incoming)
    }

    /// A fresh folder to unpack into, clearing any left by a launcher that
    /// stopped halfway.
    fn incoming(&self) -> Result<PathBuf, String> {
        fs::create_dir_all(&self.root)
            .map_err(|e| format!("could not create {}: {e}", self.root.display()))?;
        if let Ok(entries) = fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with("incoming-") {
                    let _ = fs::remove_dir_all(entry.path());
                }
            }
        }
        let dir = self.root.join(format!("incoming-{}", std::process::id()));
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir)
    }

    /// Make `incoming` current and keep the old current as previous. If the
    /// old game is still running (Windows will not move it), nothing
    /// changes.
    fn swap_in(&self, incoming: &Path) -> Result<Install, String> {
        let current = self.root.join("current");
        let previous = self.root.join("previous");
        if current.exists() {
            let _ = fs::remove_dir_all(&previous);
            if let Err(e) = fs::rename(&current, &previous) {
                let _ = fs::remove_dir_all(incoming);
                return Err(format!("the game is still running? ({e})"));
            }
        }
        if let Err(e) = fs::rename(incoming, &current) {
            let _ = fs::rename(&previous, &current);
            let _ = fs::remove_dir_all(incoming);
            return Err(format!("could not install: {e}"));
        }
        read_install(&current).ok_or_else(|| "the installed game is incomplete".into())
    }
}

/// A game folder with its `VERSION` and executable.
fn read_install(dir: &Path) -> Option<Install> {
    let version = fs::read_to_string(dir.join("VERSION"))
        .ok()?
        .trim()
        .to_string();
    parse_version(&version)?;
    let install = Install {
        version,
        dir: dir.to_path_buf(),
    };
    install.complete().then_some(install)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in
        fs::read_dir(from).map_err(|e| format!("could not read {}: {e}", from.display()))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let (src, dest) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_dir(&src, &dest)?;
        } else {
            fs::copy(&src, &dest).map_err(|e| format!("could not copy {}: {e}", src.display()))?;
        }
    }
    Ok(())
}

/// Remove the marks a browser leaves on downloaded files (macOS
/// quarantine, the Windows zone stream) from a folder the launcher copied.
fn clear_download_marks(dir: &Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("/usr/bin/xattr")
            .arg("-dr")
            .arg("com.apple.quarantine")
            .arg(dir)
            .output();
    }
    #[cfg(windows)]
    {
        fn walk(dir: &Path) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path);
                } else {
                    let mut stream = path.into_os_string();
                    stream.push(":Zone.Identifier");
                    let _ = fs::remove_file(stream);
                }
            }
        }
        walk(dir);
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    let _ = dir;
}

/// Unpack a zip archive into `dest`, refusing entries that would land
/// outside it and keeping Unix permissions.
pub fn unpack(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(archive).map_err(|e| format!("could not open the download: {e}"))?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| format!("the download is not a zip: {e}"))?;
    fs::create_dir_all(dest).map_err(|e| format!("could not create {}: {e}", dest.display()))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("bad zip entry: {e}"))?;
        let Some(name) = entry.enclosed_name() else {
            return Err(format!("unsafe path in the download: {}", entry.name()));
        };
        let path = dest.join(name);
        if entry.is_dir() {
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = fs::File::create(&path)
            .map_err(|e| format!("could not write {}: {e}", path.display()))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| format!("could not unpack {}: {e}", path.display()))?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(mode & 0o777))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Zip a folder's contents (for releases), keeping executable bits.
pub fn pack(dir: &Path, out: &Path) -> Result<(), String> {
    let file =
        fs::File::create(out).map_err(|e| format!("could not create {}: {e}", out.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let mut paths = Vec::new();
    collect(dir, dir, &mut paths)?;
    paths.sort();
    for rel in paths {
        let path = dir.join(&rel);
        let name = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(&path)
                .map_err(|e| e.to_string())?
                .permissions()
                .mode()
                & 0o777
        };
        #[cfg(not(unix))]
        let mode = 0o644;
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(mode);
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        fs::File::open(&path)
            .and_then(|mut f| f.read_to_end(&mut bytes))
            .map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("could not read {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!("symlinks are not packed: {}", path.display()));
        } else if kind.is_dir() {
            collect(root, &path, out)?;
        } else {
            out.push(
                path.strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

/// BLAKE3 of a file, hex.
pub fn hash_file(path: &Path) -> Result<String, String> {
    let mut hasher = blake3::Hasher::new();
    let mut file =
        fs::File::open(path).map_err(|e| format!("could not open {}: {e}", path.display()))?;
    std::io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
    Ok(hasher.finalize().to_hex().to_string())
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(20))
        .user_agent(&format!(
            "brinewake-launcher/{} ({})",
            env!("CARGO_PKG_VERSION"),
            platform()
        ))
        .build()
}

/// Only https, or plain http to this machine for testing.
fn allowed_url(url: &str) -> bool {
    url.starts_with("https://")
        || url.starts_with("http://127.0.0.1:")
        || url.starts_with("http://localhost:")
}

/// Fetch a small file (the manifest or its signature).
pub fn fetch_small(url: &str) -> Result<Vec<u8>, String> {
    if !allowed_url(url) {
        return Err(format!("refusing {url}"));
    }
    let response = agent()
        .get(url)
        .timeout(Duration::from_secs(10))
        .call()
        .map_err(describe)?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(SMALL_LIMIT)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn describe(error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, _) => format!("the server answered {code}"),
        ureq::Error::Transport(t) => format!("no connection ({})", t.kind()),
    }
}

/// Fetch and check the newest release: its manifest, if the signature
/// holds.
pub fn check_any(urls: &[String]) -> Result<Manifest, String> {
    let mut last = String::from("no update address");
    for url in urls {
        match check(url) {
            Ok(manifest) => return Ok(manifest),
            Err(e) => last = e,
        }
    }
    Err(last)
}

pub fn check(manifest_url: &str) -> Result<Manifest, String> {
    let bytes = fetch_small(manifest_url)?;
    let signature = fetch_small(&format!("{manifest_url}.sig"))?;
    verify(&bytes, &String::from_utf8_lossy(&signature), RELEASE_KEY)?;
    parse_manifest(&bytes)
}

/// Download `asset` to `dest`, checking its size and hash, reporting
/// (bytes so far, total) as it goes.
pub fn download(
    asset: &Asset,
    dest: &Path,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    if !allowed_url(&asset.url) {
        return Err(format!("refusing {}", asset.url));
    }
    if asset.size > DOWNLOAD_LIMIT {
        return Err("the download is too large".into());
    }
    let response = agent().get(&asset.url).call().map_err(describe)?;
    let mut reader = response.into_reader().take(asset.size + 1);
    let mut out =
        fs::File::create(dest).map_err(|e| format!("could not save the download: {e}"))?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("the download stopped: {e}"))?;
        if n == 0 {
            break;
        }
        done += n as u64;
        if done > asset.size {
            return Err("the download is larger than the release says".into());
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n])
            .map_err(|e| format!("could not save the download: {e}"))?;
        progress(done, asset.size);
    }
    out.sync_all().map_err(|e| e.to_string())?;
    if done != asset.size {
        return Err(format!(
            "the download stopped at {done} of {} bytes",
            asset.size
        ));
    }
    if hasher.finalize().to_hex().as_str() != asset.blake3.to_ascii_lowercase() {
        return Err("the download does not match the release".into());
    }
    Ok(())
}

/// What the launcher is doing, for its window.
#[derive(Clone, Debug, PartialEq)]
pub enum Stage {
    Checking,
    Downloading {
        version: String,
        done: u64,
        total: u64,
    },
    Installing {
        version: String,
    },
    /// Ready to start `install`; `note` says why it is not the newest.
    Ready {
        install: Install,
        note: Option<String>,
    },
    /// Nothing to start.
    Failed(String),
}

/// The whole update: check, fetch and install when there is something
/// newer, and choose the game to start. Never fails for want of a network:
/// it falls back to the newest game already here.
pub fn update(
    manifest_urls: &[String],
    store: Option<&Store>,
    bundled: Option<Install>,
    mut report: impl FnMut(Stage),
) -> Stage {
    let mut installed = store.and_then(Store::installed);
    // The bundled game goes in the store when it is newer than what is
    // there (or nothing is): the game always runs from one place.
    if let (Some(store), Some(b)) = (store, &bundled)
        && installed
            .as_ref()
            .is_none_or(|i| parse_version(&i.version) < parse_version(&b.version))
        && let Ok(adopted) = store.adopt(b)
    {
        installed = Some(adopted);
    }
    // On a tie the installed copy wins (`max_by_key` keeps the last).
    let local = [bundled, installed]
        .into_iter()
        .flatten()
        .max_by_key(|i| parse_version(&i.version).unwrap_or_default());
    let fallback = |note: String| match &local {
        Some(install) => Stage::Ready {
            install: install.clone(),
            note: Some(note),
        },
        None => Stage::Failed(note),
    };
    report(Stage::Checking);
    let manifest = match check_any(manifest_urls) {
        Ok(m) => m,
        Err(e) => return fallback(format!("NO UPDATE CHECK: {e}")),
    };
    let newest = parse_version(&manifest.version).unwrap_or_default();
    if let Some(install) = &local
        && parse_version(&install.version).unwrap_or_default() >= newest
    {
        return Stage::Ready {
            install: install.clone(),
            note: None,
        };
    }
    if manifest.launcher > LAUNCHER {
        return fallback(format!("{} NEEDS A NEW LAUNCHER: {PAGE}", manifest.version));
    }
    let Some(entry) = manifest.platforms.get(platform()) else {
        return fallback(format!(
            "{} IS NOT OUT FOR THIS SYSTEM YET",
            manifest.version
        ));
    };
    let Some(store) = store else {
        return fallback("NOWHERE TO INSTALL UPDATES".into());
    };
    if let Err(e) = fs::create_dir_all(&store.root) {
        return fallback(format!("CANNOT INSTALL UPDATES: {e}"));
    }
    let archive = store
        .root
        .join(format!("download-{}.zip", std::process::id()));
    let version = manifest.version.clone();
    let result = download(&entry.game, &archive, |done, total| {
        report(Stage::Downloading {
            version: version.clone(),
            done,
            total,
        });
    })
    .and_then(|()| {
        report(Stage::Installing {
            version: version.clone(),
        });
        store.install(&version, &archive)
    });
    let _ = fs::remove_file(&archive);
    match result {
        Ok(install) => Stage::Ready {
            install,
            note: None,
        },
        Err(e) => fallback(format!("UPDATE FAILED: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bw_launcher_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A fake game folder for `version`.
    fn game(dir: &Path, version: &str) {
        let exe = game_exe(dir);
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, format!("game {version}")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(dir.join("VERSION"), version).unwrap();
    }

    #[test]
    fn versions_compare_as_numbers() {
        assert!(parse_version("0.10.0") > parse_version("0.9.9"));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version(" 1.2.3\n"), Some((1, 2, 3)));
    }

    #[test]
    fn a_signature_holds_only_for_its_bytes_and_key() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let public = encode_hex(key.verifying_key().as_bytes());
        let bytes = br#"{"schema":1}"#;
        let sig = encode_hex(&key.sign(bytes).to_bytes());
        assert!(verify(bytes, &sig, &public).is_ok());
        assert!(verify(br#"{"schema":2}"#, &sig, &public).is_err());
        let other = ed25519_dalek::SigningKey::from_bytes(&[8u8; 32]);
        assert!(verify(bytes, &sig, &encode_hex(other.verifying_key().as_bytes())).is_err());
        assert!(verify(bytes, "zz", &public).is_err());
    }

    #[test]
    fn the_embedded_release_key_is_a_valid_key() {
        let key: [u8; 32] = decode_hex(RELEASE_KEY).unwrap().try_into().unwrap();
        assert!(ed25519_dalek::VerifyingKey::from_bytes(&key).is_ok());
    }

    #[test]
    fn install_unpacks_switches_and_keeps_one_previous() {
        let dir = temp("install");
        let store = Store {
            root: dir.join("store"),
        };
        for v in ["0.1.0", "0.2.0", "0.3.0"] {
            let src = dir.join(format!("src-{v}"));
            game(&src, v);
            let zip = dir.join(format!("{v}.zip"));
            pack(&src, &zip).unwrap();
            let install = store.install(v, &zip).unwrap();
            assert_eq!(install.version, v);
            assert_eq!(
                fs::read_to_string(install.exe()).unwrap(),
                format!("game {v}")
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    fs::metadata(install.exe()).unwrap().permissions().mode() & 0o111,
                    0o111
                );
            }
        }
        assert_eq!(store.installed().unwrap().version, "0.3.0");
        assert_eq!(store.installed().unwrap().dir, store.root.join("current"));
        assert_eq!(
            read_install(&store.root.join("previous")).unwrap().version,
            "0.2.0"
        );
        let mut left: Vec<String> = fs::read_dir(&store.root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into())
            .collect();
        left.sort();
        assert_eq!(left, vec!["current", "previous"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_archive_without_a_game_is_refused() {
        let dir = temp("empty");
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("readme.txt"), "hi").unwrap();
        let zip = dir.join("x.zip");
        pack(&src, &zip).unwrap();
        let store = Store {
            root: dir.join("store"),
        };
        assert!(store.install("0.2.0", &zip).is_err());
        assert!(store.installed().is_none());
        // A game whose VERSION differs from the release is refused too.
        let other = dir.join("other");
        game(&other, "0.3.0");
        let zip = dir.join("y.zip");
        pack(&other, &zip).unwrap();
        assert!(store.install("0.2.0", &zip).is_err());
        assert!(store.installed().is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsafe_zip_paths_are_refused() {
        let dir = temp("slip");
        let zip_path = dir.join("evil.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        zip.start_file("../evil.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        assert!(unpack(&zip_path, &dir.join("out")).is_err());
        assert!(!dir.join("evil.txt").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_bundled_game_is_copied_in_when_it_is_newer() {
        let dir = temp("adopt");
        let store = Store {
            root: dir.join("store"),
        };
        let bundle_dir = dir.join("bundled");
        game(&bundle_dir, "0.2.0");
        let bundled = Install {
            version: "0.2.0".into(),
            dir: bundle_dir,
        };
        let stage = update(
            &["http://127.0.0.1:9/latest.json".to_string()],
            Some(&store),
            Some(bundled.clone()),
            |_| {},
        );
        let Stage::Ready { install, .. } = stage else {
            panic!("{stage:?}")
        };
        assert_eq!(install.dir, store.root.join("current"));
        assert_eq!(install.version, "0.2.0");
        // An older bundle does not replace a newer install.
        let old_dir = dir.join("old");
        game(&old_dir, "0.1.0");
        let stage = update(
            &["http://127.0.0.1:9/latest.json".to_string()],
            Some(&store),
            Some(Install {
                version: "0.1.0".into(),
                dir: old_dir,
            }),
            |_| {},
        );
        let Stage::Ready { install, .. } = stage else {
            panic!("{stage:?}")
        };
        assert_eq!(install.version, "0.2.0");
        assert_eq!(install.dir, store.root.join("current"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn with_no_network_the_newest_local_game_starts() {
        let dir = temp("offline");
        let store = Store {
            root: dir.join("store"),
        };
        let src = dir.join("src");
        game(&src, "0.2.0");
        let zip = dir.join("g.zip");
        pack(&src, &zip).unwrap();
        store.install("0.2.0", &zip).unwrap();
        let bundle_dir = dir.join("bundled");
        game(&bundle_dir, "0.1.0");
        let bundled = Install {
            version: "0.1.0".into(),
            dir: bundle_dir,
        };
        // Nothing listens on port 9.
        let stage = update(
            &["http://127.0.0.1:9/latest.json".to_string()],
            Some(&store),
            Some(bundled),
            |_| {},
        );
        match stage {
            Stage::Ready { install, note } => {
                assert_eq!(install.version, "0.2.0");
                assert!(note.unwrap().starts_with("NO UPDATE CHECK"));
            }
            other => panic!("{other:?}"),
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
