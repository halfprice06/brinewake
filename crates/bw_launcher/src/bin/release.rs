//! Release tooling for BRINEWAKE's launcher:
//!
//! ```text
//! brinewake-release keygen KEYFILE              new signing key; prints the public key
//! brinewake-release pack DIR OUT.zip            zip a folder, keeping executable bits
//! brinewake-release manifest OUT.json VERSION DATE BASE_URL NOTES_FILE \
//!     PLATFORM LABEL GAME.zip DOWNLOAD.zip [PLATFORM LABEL GAME.zip DOWNLOAD.zip ...]
//! brinewake-release sign KEYFILE FILE           writes FILE.sig
//! brinewake-release verify FILE                 checks FILE.sig against the launcher's key
//! ```
//!
//! BASE_URL is where the files will be served from, e.g.
//! `https://danprice.ai/brinewake/files/0.2.0`. NOTES_FILE has one note per
//! line.

use bw_launcher::{
    Asset, LAUNCHER, Manifest, Platform, RELEASE_KEY, decode_hex, encode_hex, hash_file, pack,
    parse_manifest, parse_version, verify,
};
use ed25519_dalek::Signer;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("keygen") if args.len() == 2 => keygen(&args[1]),
        Some("pack") if args.len() == 3 => pack(Path::new(&args[1]), Path::new(&args[2])),
        Some("manifest") if args.len() >= 10 && (args.len() - 6).is_multiple_of(4) => {
            manifest(&args[1..])
        }
        Some("sign") if args.len() == 3 => sign(&args[1], &args[2]),
        Some("verify") if args.len() == 2 => check(&args[1]),
        _ => Err("usage: see the top of crates/bw_launcher/src/bin/release.rs".into()),
    };
    if let Err(e) = result {
        eprintln!("brinewake-release: {e}");
        std::process::exit(1);
    }
}

fn read_key(path: &str) -> Result<ed25519_dalek::SigningKey, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("could not read the key {path}: {e}"))?;
    let bytes: [u8; 32] = decode_hex(&text)
        .and_then(|b| b.try_into().ok())
        .ok_or("the key file is malformed")?;
    Ok(ed25519_dalek::SigningKey::from_bytes(&bytes))
}

fn keygen(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        return Err(format!("{path} exists; not overwriting a signing key"));
    }
    let mut seed = [0u8; 32];
    fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut seed))
        .map_err(|e| format!("no randomness: {e}"))?;
    let key = ed25519_dalek::SigningKey::from_bytes(&seed);
    fs::write(path, encode_hex(&seed)).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    println!("{}", encode_hex(key.verifying_key().as_bytes()));
    Ok(())
}

fn asset(base: &str, path: &str) -> Result<Asset, String> {
    let p = Path::new(path);
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("bad file name")?;
    Ok(Asset {
        url: format!("{}/{name}", base.trim_end_matches('/')),
        size: fs::metadata(p).map_err(|e| format!("{path}: {e}"))?.len(),
        blake3: hash_file(p)?,
    })
}

fn manifest(args: &[String]) -> Result<(), String> {
    let (out, version, date, base, notes_file) = (&args[0], &args[1], &args[2], &args[3], &args[4]);
    parse_version(version).ok_or("VERSION must be major.minor.patch")?;
    let notes = fs::read_to_string(notes_file)
        .map_err(|e| format!("{notes_file}: {e}"))?
        .lines()
        .map(|l| l.trim().trim_start_matches("- ").to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let mut platforms = BTreeMap::new();
    for chunk in args[5..].chunks(4) {
        platforms.insert(
            chunk[0].clone(),
            Platform {
                label: chunk[1].clone(),
                game: asset(base, &chunk[2])?,
                download: asset(base, &chunk[3])?,
            },
        );
    }
    let manifest = Manifest {
        schema: 1,
        version: version.clone(),
        published: date.clone(),
        notes,
        launcher: LAUNCHER,
        platforms,
    };
    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n";
    fs::write(out, json).map_err(|e| e.to_string())?;
    println!("wrote {out}");
    Ok(())
}

fn sign(key: &str, file: &str) -> Result<(), String> {
    let key = read_key(key)?;
    if encode_hex(key.verifying_key().as_bytes()) != RELEASE_KEY.trim() {
        return Err("this key is not the one the launcher trusts".into());
    }
    let bytes = fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    fs::write(
        format!("{file}.sig"),
        encode_hex(&key.sign(&bytes).to_bytes()),
    )
    .map_err(|e| e.to_string())?;
    println!("wrote {file}.sig");
    Ok(())
}

fn check(file: &str) -> Result<(), String> {
    let bytes = fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    let sig = fs::read_to_string(format!("{file}.sig")).map_err(|e| format!("{file}.sig: {e}"))?;
    verify(&bytes, &sig, RELEASE_KEY)?;
    let manifest = parse_manifest(&bytes)?;
    println!(
        "{file}: signature good; version {} for {:?}",
        manifest.version,
        manifest.platforms.keys().collect::<Vec<_>>()
    );
    Ok(())
}
