//! The release tool for SI Future HUD's updates.
//!
//! ```text
//! si-hud-update keygen [--dir DIR]
//!     Makes the release signing key: DIR/update-signing.key (private,
//!     mode 600) and DIR/update-signing.pub. Never overwrites a key.
//!     DIR defaults to ~/Documents/Claude/si-future-hud-release/keys.
//!
//! si-hud-update feed --dmg PATH --version X.Y.Z [--key FILE]
//!         [--notes-zh TEXT|@FILE] [--notes-en TEXT|@FILE]
//!         [--url URL]... [--pub-date RFC3339] [--out FILE] [--any-key]
//!     Checks the disk image (one app, bundle id com.agenthud.next.preview,
//!     version X.Y.Z), hashes it, signs `si-future-hud-update\nX.Y.Z\n<sha256>`
//!     and writes the feed (default site/update.json). Without --url the links
//!     are the site's SI-Future-HUD.dmg and the GitHub release asset
//!     SI-Future-HUD_X.Y.Z_aarch64.dmg. The key must be the one the app has
//!     compiled in unless --any-key.
//!
//! si-hud-update verify [--feed FILE] [--dmg PATH]
//!     Checks a feed's signature against the compiled-in key, and that the
//!     disk image is the one it describes.
//! ```
use agent_hud_updater::{feed, fetch, github_dmg, install, SITE_DMG};
use base64::Engine;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
#[path = "unix.rs"]
mod private;
#[cfg(not(unix))]
#[path = "elsewhere.rs"]
mod private;

const KEY_FILE: &str = "update-signing.key";
const PUBLIC_FILE: &str = "update-signing.pub";

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = run(&arguments) {
        eprintln!("si-hud-update: {error}");
        std::process::exit(1);
    }
}

fn run(arguments: &[String]) -> Result<(), String> {
    let (command, rest) = arguments
        .split_first()
        .ok_or("usage: si-hud-update keygen|feed|verify … (see the file's header)")?;
    let options = Options::parse(rest)?;
    match command.as_str() {
        "keygen" => keygen(&options),
        "feed" => make_feed(&options),
        "verify" => verify(&options),
        other => Err(format!("unknown command {other}")),
    }
}

/// `--name value` pairs; `--url` may repeat; `--any-key` takes no value.
struct Options {
    values: HashMap<String, String>,
    urls: Vec<String>,
    any_key: bool,
}

impl Options {
    fn parse(arguments: &[String]) -> Result<Self, String> {
        let mut options = Options {
            values: HashMap::new(),
            urls: Vec::new(),
            any_key: false,
        };
        let mut rest = arguments.iter();
        while let Some(name) = rest.next() {
            let name = name
                .strip_prefix("--")
                .ok_or_else(|| format!("unexpected argument {name}"))?;
            if name == "any-key" {
                options.any_key = true;
                continue;
            }
            let value = rest
                .next()
                .ok_or_else(|| format!("--{name} needs a value"))?
                .clone();
            if name == "url" {
                options.urls.push(value);
            } else if options.values.insert(name.to_owned(), value).is_some() {
                return Err(format!("--{name} is given twice"));
            }
        }
        Ok(options)
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    fn required(&self, name: &str) -> Result<&str, String> {
        self.get(name)
            .ok_or_else(|| format!("--{name} is required"))
    }
}

fn default_keys() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home).join("Documents/Claude/si-future-hud-release/keys"))
}

fn keygen(options: &Options) -> Result<(), String> {
    let directory = match options.get("dir") {
        Some(directory) => PathBuf::from(directory),
        None => default_keys()?,
    };
    let key = directory.join(KEY_FILE);
    if key.exists() {
        return Err(format!(
            "{} already exists; it is never overwritten",
            key.display()
        ));
    }
    let (private, public) = feed::generate_key()?;
    private::directory(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let encode = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
    private::write_new(&key, format!("{}\n", encode(&private)).as_bytes())
        .map_err(|error| format!("{}: {error}", key.display()))?;
    let public_file = directory.join(PUBLIC_FILE);
    fs::write(&public_file, format!("{}\n", encode(&public)))
        .map_err(|error| format!("{}: {error}", public_file.display()))?;
    println!(
        "private key: {} (mode 600; keep it out of the repository)",
        key.display()
    );
    println!("public key:  {}", public_file.display());
    println!(
        "PUBLIC_KEY for src/crates/updater/src/feed.rs: {}",
        encode(&public)
    );
    Ok(())
}

fn read_key(path: &Path) -> Result<Vec<u8>, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .map_err(|_| format!("{} is not a base64 key", path.display()))
}

/// A notes argument: the text itself, or `@FILE` for a file's text.
fn notes(options: &Options, name: &str) -> Result<String, String> {
    match options.get(name) {
        None => Ok(String::new()),
        Some(value) => match value.strip_prefix('@') {
            Some(path) => fs::read_to_string(path)
                .map(|text| text.trim_end().to_owned())
                .map_err(|error| format!("{path}: {error}")),
            None => Ok(value.to_owned()),
        },
    }
}

fn make_feed(options: &Options) -> Result<(), String> {
    let dmg = PathBuf::from(options.required("dmg")?);
    let version = options.required("version")?.to_owned();
    semver::Version::parse(&version).map_err(|_| format!("{version} is not a semantic version"))?;
    let key_path = match options.get("key") {
        Some(path) => PathBuf::from(path),
        None => default_keys()?.join(KEY_FILE),
    };
    let private = read_key(&key_path)?;
    let public = feed::public_key_of(&private)?;
    if public != feed::embedded_public_key() && !options.any_key {
        return Err(format!(
            "{} is not the key the app trusts (feed.rs PUBLIC_KEY); pass --any-key only for a test feed",
            key_path.display()
        ));
    }
    install::inspect(&dmg, &version).map_err(|reason| format!("{}: {reason}", dmg.display()))?;
    let size = fs::metadata(&dmg)
        .map_err(|error| format!("{}: {error}", dmg.display()))?
        .len();
    let sha256 = fetch::sha256_file(&dmg).map_err(|error| format!("{}: {error}", dmg.display()))?;
    let urls = if options.urls.is_empty() {
        vec![SITE_DMG.to_owned(), github_dmg(&version)]
    } else {
        options.urls.clone()
    };
    let signature = feed::sign(&version, &sha256, &private)?;
    let made = feed::Feed {
        version,
        pub_date: Some(
            options
                .get("pub-date")
                .map(str::to_owned)
                .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        ),
        notes: feed::Notes {
            zh: notes(options, "notes-zh")?,
            en: notes(options, "notes-en")?,
        },
        sha256,
        size,
        urls,
        signature,
    };
    let out = PathBuf::from(options.get("out").unwrap_or("site/update.json"));
    if let Some(folder) = out.parent().filter(|folder| !folder.as_os_str().is_empty()) {
        fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    let json = serde_json::to_string_pretty(&made).map_err(|error| error.to_string())?;
    fs::write(&out, format!("{json}\n")).map_err(|error| format!("{}: {error}", out.display()))?;
    // Read back as the app reads it.
    let written = feed::parse(&fs::read(&out).map_err(|error| error.to_string())?)
        .map_err(|reason| reason.to_string())?;
    feed::verify(&written, &public).map_err(|reason| reason.to_string())?;
    println!("wrote {}", out.display());
    println!(
        "  version {}  size {} bytes  sha256 {}",
        written.version, written.size, written.sha256
    );
    for url in &written.urls {
        println!("  {url}");
    }
    Ok(())
}

fn verify(options: &Options) -> Result<(), String> {
    let path = PathBuf::from(options.get("feed").unwrap_or("site/update.json"));
    let read =
        feed::parse(&fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?)
            .map_err(|reason| reason.to_string())?;
    feed::verify(&read, &feed::embedded_public_key()).map_err(|reason| reason.to_string())?;
    println!(
        "{}: {} is signed with the app's key",
        path.display(),
        read.version
    );
    if let Some(dmg) = options.get("dmg") {
        let dmg = Path::new(dmg);
        let size = fs::metadata(dmg)
            .map_err(|error| format!("{}: {error}", dmg.display()))?
            .len();
        let sha256 =
            fetch::sha256_file(dmg).map_err(|error| format!("{}: {error}", dmg.display()))?;
        if size != read.size || sha256 != read.sha256 {
            return Err(format!(
                "{} is not the image the feed describes (size {size}, sha256 {sha256})",
                dmg.display()
            ));
        }
        install::inspect(dmg, &read.version).map_err(|reason| reason.to_string())?;
        println!(
            "{}: matches, and holds SI Future HUD {}",
            dmg.display(),
            read.version
        );
    }
    Ok(())
}
