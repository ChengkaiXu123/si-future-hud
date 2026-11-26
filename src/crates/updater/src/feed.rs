//! `update.json`: what a release says about itself, and the signature that
//! makes the app believe it.
//!
//! ```json
//! {"version":"0.1.8","pubDate":"2026-11-26T08:00:00Z",
//!  "notes":{"zh":"…","en":"…"},
//!  "sha256":"<64 lowercase hex digits of the disk image>","size":47185920,
//!  "urls":["<official site>","<GitHub release asset>"],
//!  "signature":"<base64 Ed25519 signature>"}
//! ```
//!
//! The signature covers exactly `si-future-hud-update\n<version>\n<sha256>`
//! (no trailing newline). The links and notes are not signed: a link that
//! serves anything else fails the SHA-256 check, and the size only bounds
//! the download.
use crate::text::{text, Text};
use base64::Engine;
use ring::signature::{self, Ed25519KeyPair, KeyPair, UnparsedPublicKey};
use serde::{Deserialize, Serialize};

/// The public half of the release signing key (base64, 32 bytes). Its
/// private half is kept outside the repository; `si-hud-update keygen`
/// made both.
pub const PUBLIC_KEY: &str = "2SQt01sDsutoknL01SZMT+/26RXZsdRcmDp2Jzsr/vU=";
/// The first line of every signed message, so a signature made for anything
/// else never verifies an update.
pub const CONTEXT: &str = "si-future-hud-update";
/// No release is this large; a feed that says so is refused.
const MAX_SIZE: u64 = 2 << 30;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notes {
    #[serde(default)]
    pub zh: String,
    #[serde(default)]
    pub en: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feed {
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pub_date: Option<String>,
    #[serde(default)]
    pub notes: Notes,
    pub sha256: String,
    pub size: u64,
    pub urls: Vec<String>,
    pub signature: String,
}

/// The exact bytes a release signature covers.
pub fn signed_message(version: &str, sha256: &str) -> String {
    format!("{CONTEXT}\n{version}\n{sha256}")
}

/// A feed read from its JSON, with every field the app relies on checked;
/// the signature is checked separately (`verify`).
pub fn parse(bytes: &[u8]) -> Result<Feed, Text> {
    let feed: Feed = serde_json::from_slice(bytes).map_err(|error| {
        text(
            format!("update.json 读不懂：{error}"),
            format!("update.json could not be read: {error}"),
        )
    })?;
    semver::Version::parse(&feed.version).map_err(|_| {
        text(
            format!("update.json 里的版本号「{}」不对", feed.version),
            format!("update.json names an invalid version, “{}”", feed.version),
        )
    })?;
    if !is_sha256(&feed.sha256) {
        return Err(text(
            "update.json 里的 sha256 不是 64 位小写十六进制",
            "update.json's sha256 is not 64 lowercase hexadecimal digits",
        ));
    }
    if feed.size == 0 || feed.size > MAX_SIZE {
        return Err(text(
            format!("update.json 里的大小 {} 字节不合理", feed.size),
            format!("update.json gives an implausible size, {} bytes", feed.size),
        ));
    }
    if feed.urls.is_empty() {
        return Err(text(
            "update.json 没有下载地址",
            "update.json has no download link",
        ));
    }
    Ok(feed)
}

/// Whether `public_key` signed this feed's version and SHA-256.
pub fn verify(feed: &Feed, public_key: &[u8]) -> Result<(), Text> {
    let invalid = || {
        text(
            "签名校验失败：update.json 不是用发布密钥签的，或被改过",
            "Signature check failed: update.json was not signed with the release key, or was altered",
        )
    };
    let signature = base64::engine::general_purpose::STANDARD
        .decode(feed.signature.trim())
        .map_err(|_| invalid())?;
    UnparsedPublicKey::new(&signature::ED25519, public_key)
        .verify(
            signed_message(&feed.version, &feed.sha256).as_bytes(),
            &signature,
        )
        .map_err(|_| invalid())
}

/// The compiled-in public key.
pub fn embedded_public_key() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(PUBLIC_KEY)
        .unwrap_or_default()
}

/// A new signing key: its PKCS#8 document (the private key file) and its
/// public half.
pub fn generate_key() -> Result<(Vec<u8>, Vec<u8>), String> {
    let random = ring::rand::SystemRandom::new();
    let document = Ed25519KeyPair::generate_pkcs8(&random)
        .map_err(|_| "The system random source failed".to_owned())?;
    let public = public_key_of(document.as_ref())?;
    Ok((document.as_ref().to_vec(), public))
}

/// The public half of a PKCS#8 signing key.
pub fn public_key_of(pkcs8: &[u8]) -> Result<Vec<u8>, String> {
    Ed25519KeyPair::from_pkcs8(pkcs8)
        .map(|pair| pair.public_key().as_ref().to_vec())
        .map_err(|error| format!("Not an Ed25519 PKCS#8 key: {error}"))
}

/// The base64 signature of a release, made with a PKCS#8 signing key.
pub fn sign(version: &str, sha256: &str, pkcs8: &[u8]) -> Result<String, String> {
    let pair = Ed25519KeyPair::from_pkcs8(pkcs8)
        .map_err(|error| format!("Not an Ed25519 PKCS#8 key: {error}"))?;
    let signature = pair.sign(signed_message(version, sha256).as_bytes());
    Ok(base64::engine::general_purpose::STANDARD.encode(signature.as_ref()))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const SHA: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    pub(crate) fn signed(version: &str, sha256: &str, size: u64, pkcs8: &[u8]) -> Feed {
        Feed {
            version: version.into(),
            pub_date: Some("2026-11-26T08:00:00Z".into()),
            notes: Notes {
                zh: "修复".into(),
                en: "Fixes".into(),
            },
            sha256: sha256.into(),
            size,
            urls: vec!["https://example.invalid/SI-Future-HUD.dmg".into()],
            signature: sign(version, sha256, pkcs8).unwrap(),
        }
    }

    #[test]
    fn the_signed_message_is_fixed() {
        assert_eq!(
            signed_message("0.1.8", SHA),
            format!("si-future-hud-update\n0.1.8\n{SHA}")
        );
    }

    #[test]
    fn a_feed_signed_with_the_key_verifies_and_round_trips() {
        let (private, public) = generate_key().unwrap();
        let feed = signed("0.1.8", SHA, 47_185_920, &private);
        let json = serde_json::to_vec(&feed).unwrap();
        let read = parse(&json).unwrap();
        assert_eq!(read, feed);
        verify(&read, &public).unwrap();
        // The documented shape: camelCase pubDate, notes in both languages.
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(value["pubDate"], "2026-11-26T08:00:00Z");
        assert_eq!(value["notes"]["zh"], "修复");
    }

    #[test]
    fn a_signature_fails_for_another_key_version_or_hash() {
        let (private, public) = generate_key().unwrap();
        let (_, stranger) = generate_key().unwrap();
        let feed = signed("0.1.8", SHA, 10, &private);
        assert!(verify(&feed, &stranger).is_err(), "another key");
        let mut raised = feed.clone();
        raised.version = "0.1.9".into();
        assert!(verify(&raised, &public).is_err(), "a changed version");
        let mut swapped = feed.clone();
        swapped.sha256 = "0".repeat(64);
        assert!(verify(&swapped, &public).is_err(), "a changed hash");
        let mut garbled = feed.clone();
        garbled.signature = "not base64!".into();
        assert!(
            verify(&garbled, &public).is_err(),
            "an unreadable signature"
        );
        assert!(verify(&feed, &[]).is_err(), "no key at all");
        // Unsigned fields may change: links and notes.
        let mut moved = feed;
        moved.urls = vec!["https://mirror.invalid/a.dmg".into()];
        moved.notes.en = "Other words".into();
        verify(&moved, &public).unwrap();
    }

    #[test]
    fn malformed_feeds_are_refused_with_a_reason() {
        let (private, _) = generate_key().unwrap();
        let good = signed("0.1.8", SHA, 10, &private);
        let reject = |change: &dyn Fn(&mut serde_json::Value)| {
            let mut value = serde_json::to_value(&good).unwrap();
            change(&mut value);
            parse(&serde_json::to_vec(&value).unwrap()).unwrap_err()
        };
        assert!(reject(&|v| v["version"] = "latest".into())
            .en
            .contains("version"));
        assert!(reject(&|v| v["sha256"] = SHA.to_uppercase().into())
            .en
            .contains("sha256"));
        assert!(reject(&|v| v["sha256"] = "abc".into())
            .en
            .contains("sha256"));
        assert!(reject(&|v| v["size"] = 0.into()).en.contains("size"));
        assert!(reject(&|v| v["urls"] = serde_json::json!([]))
            .en
            .contains("link"));
        assert!(reject(&|v| {
            v.as_object_mut().unwrap().remove("signature");
        })
        .en
        .contains("could not be read"));
        assert!(parse(b"<html>404</html>").is_err());
    }

    #[test]
    fn the_embedded_key_is_a_whole_ed25519_key() {
        assert_eq!(embedded_public_key().len(), 32, "PUBLIC_KEY is set");
    }
}
