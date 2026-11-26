//! Reading the feed and downloading the disk image it names, over HTTPS (and,
//! for a test feed, plain HTTP or a local file).
use crate::feed::{self, Feed};
use crate::text::{text, Text};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

/// A feed is a few kilobytes; anything past this is not one.
const FEED_LIMIT: u64 = 1 << 20;

pub struct Fetcher {
    client: reqwest::blocking::Client,
    /// Plain HTTP and local files are allowed (a test feed).
    insecure: bool,
}

impl Fetcher {
    pub fn new(version: &str, insecure: bool) -> Result<Self, Text> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(format!("SI-Future-HUD/{version}"))
            .connect_timeout(Duration::from_secs(15))
            // Each read of a download, and the wait for a response.
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| {
                text(
                    format!("无法建立网络连接：{error}"),
                    format!("The network client could not start: {error}"),
                )
            })?;
        Ok(Self { client, insecure })
    }

    /// The first feed among `sources` that reads, parses and verifies with
    /// `public_key`, and the source it came from. When none does, the reason
    /// each failed.
    pub fn feed(&self, sources: &[String], public_key: &[u8]) -> Result<(Feed, String), Text> {
        let mut failures = Vec::new();
        for source in sources {
            let read = self.read(source, FEED_LIMIT).and_then(|bytes| {
                let feed = feed::parse(&bytes)?;
                feed::verify(&feed, public_key)?;
                Ok(feed)
            });
            match read {
                Ok(feed) => return Ok((feed, source.clone())),
                Err(reason) => failures.push((source.as_str(), reason)),
            }
        }
        Err(joined(&failures))
    }

    /// The feed's disk image in `directory`, downloaded from the first of its
    /// links that serves exactly the size and SHA-256 the feed gives; a copy
    /// already there that matches is kept. `progress` hears bytes done and
    /// the total.
    pub fn download(
        &self,
        feed: &Feed,
        directory: &Path,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<PathBuf, Text> {
        fs::create_dir_all(directory).map_err(|error| {
            text(
                format!("无法创建下载目录：{error}"),
                format!("The download folder could not be created: {error}"),
            )
        })?;
        let target = directory.join(image_name(&feed.version));
        if matches(&target, feed) {
            progress(feed.size, feed.size);
            return Ok(target);
        }
        let partial = directory.join(format!("{}.part", image_name(&feed.version)));
        let mut failures = Vec::new();
        for url in &feed.urls {
            let result = self
                .save(url, &partial, feed.size, progress)
                .and_then(|digest| {
                    if digest == feed.sha256 {
                        Ok(())
                    } else {
                        Err(text(
                            "下载的文件 SHA-256 和 update.json 不一致",
                            "The downloaded file's SHA-256 does not match update.json",
                        ))
                    }
                });
            match result {
                Ok(()) => {
                    return fs::rename(&partial, &target)
                        .map(|_| target)
                        .map_err(|error| {
                            text(
                                format!("下载的文件无法保存：{error}"),
                                format!("The download could not be saved: {error}"),
                            )
                        });
                }
                Err(reason) => {
                    let _ = fs::remove_file(&partial);
                    failures.push((url.as_str(), reason));
                }
            }
        }
        Err(joined(&failures))
    }

    /// Streams `url` into `path`, refusing more than `size` bytes or fewer,
    /// and answers the SHA-256 of what it wrote.
    fn save(
        &self,
        url: &str,
        path: &Path,
        size: u64,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<String, Text> {
        let mut source = self.open(url)?;
        let mut file = fs::File::create(path).map_err(|error| {
            text(
                format!("无法写入下载文件：{error}"),
                format!("The download could not be written: {error}"),
            )
        })?;
        let mut digest = Sha256::new();
        let mut done = 0u64;
        let mut buffer = vec![0u8; 256 * 1024];
        loop {
            let read = source.read(&mut buffer).map_err(|error| {
                text(
                    format!("下载中断：{error}"),
                    format!("The download stopped: {error}"),
                )
            })?;
            if read == 0 {
                break;
            }
            done += read as u64;
            if done > size {
                return Err(text(
                    "下载的文件比 update.json 写的大",
                    "The download is larger than update.json says",
                ));
            }
            digest.update(&buffer[..read]);
            file.write_all(&buffer[..read]).map_err(|error| {
                text(
                    format!("无法写入下载文件：{error}"),
                    format!("The download could not be written: {error}"),
                )
            })?;
            progress(done, size);
        }
        file.sync_all().map_err(|error| {
            text(
                format!("无法写入下载文件：{error}"),
                format!("The download could not be written: {error}"),
            )
        })?;
        if done != size {
            return Err(text(
                format!("下载的文件大小 {done} 字节，和 update.json 写的 {size} 字节不一致"),
                format!("The download is {done} bytes, not the {size} update.json says"),
            ));
        }
        Ok(hex(&digest.finalize()))
    }

    fn read(&self, url: &str, limit: u64) -> Result<Vec<u8>, Text> {
        let mut bytes = Vec::new();
        self.open(url)?
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                text(
                    format!("读取中断：{error}"),
                    format!("Reading stopped: {error}"),
                )
            })?;
        if bytes.len() as u64 > limit {
            return Err(text(
                "内容太大，不是 update.json",
                "Too large to be update.json",
            ));
        }
        Ok(bytes)
    }

    fn open(&self, url: &str) -> Result<Box<dyn Read>, Text> {
        let parsed = reqwest::Url::parse(url).map_err(|_| {
            text(
                format!("地址无效：{url}"),
                format!("Not a valid link: {url}"),
            )
        })?;
        match parsed.scheme() {
            "https" => {}
            "http" | "file" if self.insecure => {}
            scheme => {
                return Err(text(
                    format!("不允许用 {scheme} 地址下载更新"),
                    format!("Updates are not downloaded over {scheme}"),
                ))
            }
        }
        if parsed.scheme() == "file" {
            let path = parsed.to_file_path().map_err(|_| {
                text(
                    format!("地址无效：{url}"),
                    format!("Not a valid link: {url}"),
                )
            })?;
            return fs::File::open(&path)
                .map(|file| Box::new(file) as Box<dyn Read>)
                .map_err(|error| {
                    text(
                        format!("无法打开 {}：{error}", path.display()),
                        format!("{} could not be opened: {error}", path.display()),
                    )
                });
        }
        let response = self
            .client
            .get(parsed)
            .header("Cache-Control", "no-cache")
            .send()
            .map_err(|error| {
                text(
                    format!("连不上：{}", without_url(&error)),
                    format!("Could not connect: {}", without_url(&error)),
                )
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(text(
                format!("服务器回应 {status}"),
                format!("The server answered {status}"),
            ));
        }
        Ok(Box::new(response))
    }
}

/// Where a version's disk image is kept in the updates folder.
pub fn image_name(version: &str) -> String {
    format!("SI-Future-HUD-{version}.dmg")
}

/// Whether `path` holds exactly the feed's disk image.
pub fn matches(path: &Path, feed: &Feed) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.len() == feed.size)
        && sha256_file(path).is_ok_and(|digest| digest == feed.sha256)
}

/// The lowercase hexadecimal SHA-256 of a file.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    std::io::copy(&mut file, &mut digest)?;
    Ok(hex(&digest.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn without_url(error: &reqwest::Error) -> String {
    let mut error = error.to_string();
    if let Some(start) = error.find(" for url (") {
        error.truncate(start);
    }
    error
}

/// Each source's reason, with the host it came from.
fn joined(failures: &[(&str, Text)]) -> Text {
    let place = |source: &str| {
        reqwest::Url::parse(source)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .unwrap_or_else(|| "file".into())
    };
    match failures {
        [] => text("没有可用的地址", "No source to read"),
        [(_, only)] => only.clone(),
        many => text(
            many.iter()
                .map(|(source, reason)| format!("{}：{}", place(source), reason.zh))
                .collect::<Vec<_>>()
                .join("；"),
            many.iter()
                .map(|(source, reason)| format!("{}: {}", place(source), reason.en))
                .collect::<Vec<_>>()
                .join("; "),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feed::{generate_key, sign, Notes};

    fn url(path: &Path) -> String {
        reqwest::Url::from_file_path(path).unwrap().to_string()
    }

    fn image(directory: &Path, name: &str, bytes: &[u8]) -> (PathBuf, String) {
        let path = directory.join(name);
        fs::write(&path, bytes).unwrap();
        let digest = sha256_file(&path).unwrap();
        (path, digest)
    }

    fn feed_for(version: &str, sha256: &str, size: u64, urls: Vec<String>, pkcs8: &[u8]) -> Feed {
        Feed {
            version: version.into(),
            pub_date: None,
            notes: Notes::default(),
            sha256: sha256.into(),
            size,
            urls,
            signature: sign(version, sha256, pkcs8).unwrap(),
        }
    }

    #[test]
    fn the_first_verified_feed_wins_and_bad_ones_are_explained() {
        let folder = tempfile::tempdir().unwrap();
        let (private, public) = generate_key().unwrap();
        let (_, image_sha) = image(folder.path(), "a.dmg", b"disk image");
        let good = feed_for(
            "0.1.8",
            &image_sha,
            10,
            vec!["https://a.invalid/x.dmg".into()],
            &private,
        );
        let mut forged = good.clone();
        forged.version = "9.9.9".into();
        let good_path = folder.path().join("good.json");
        let forged_path = folder.path().join("forged.json");
        fs::write(&good_path, serde_json::to_vec(&good).unwrap()).unwrap();
        fs::write(&forged_path, serde_json::to_vec(&forged).unwrap()).unwrap();
        let fetcher = Fetcher::new("0.1.7", true).unwrap();
        let missing = url(&folder.path().join("missing.json"));
        let (feed, source) = fetcher
            .feed(&[missing, url(&forged_path), url(&good_path)], &public)
            .unwrap();
        assert_eq!(feed, good);
        assert!(source.ends_with("good.json"));
        let failure = fetcher.feed(&[url(&forged_path)], &public).unwrap_err();
        assert!(failure.en.contains("Signature check failed"), "{failure}");
    }

    #[test]
    fn plain_http_and_files_are_only_for_a_test_feed() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("update.json");
        fs::write(&path, b"{}").unwrap();
        let production = Fetcher::new("0.1.7", false).unwrap();
        let refused = production.feed(&[url(&path)], &[]).unwrap_err();
        assert!(refused.en.contains("not downloaded over file"), "{refused}");
        let refused = production
            .feed(&["http://127.0.0.1:9/update.json".into()], &[])
            .unwrap_err();
        assert!(refused.en.contains("over http"), "{refused}");
    }

    #[test]
    fn a_download_must_match_the_feeds_size_and_hash() {
        let folder = tempfile::tempdir().unwrap();
        let (private, _) = generate_key().unwrap();
        let (good, sha) = image(folder.path(), "good.dmg", b"0123456789");
        let (other, _) = image(folder.path(), "other.dmg", b"9876543210");
        let (long, _) = image(folder.path(), "long.dmg", b"0123456789-and-more");
        let (short, _) = image(folder.path(), "short.dmg", b"01234");
        let fetcher = Fetcher::new("0.1.7", true).unwrap();
        let updates = folder.path().join("updates");
        let mut seen = Vec::new();

        // A link that serves another file of the same size fails its hash;
        // the next link is tried.
        let feed = feed_for("0.1.8", &sha, 10, vec![url(&other), url(&good)], &private);
        let saved = fetcher
            .download(&feed, &updates, &mut |done, total| seen.push((done, total)))
            .unwrap();
        assert_eq!(saved, updates.join("SI-Future-HUD-0.1.8.dmg"));
        assert_eq!(fs::read(&saved).unwrap(), b"0123456789");
        assert_eq!(seen.last(), Some(&(10, 10)));
        assert!(!updates.join("SI-Future-HUD-0.1.8.dmg.part").exists());

        // Already there: kept, not downloaded again (its link is gone).
        let gone = feed_for(
            "0.1.8",
            &sha,
            10,
            vec![url(&folder.path().join("gone.dmg"))],
            &private,
        );
        assert_eq!(
            fetcher.download(&gone, &updates, &mut |_, _| {}).unwrap(),
            saved
        );

        let only = |source: &Path| {
            let feed = feed_for("0.1.9", &sha, 10, vec![url(source)], &private);
            fetcher
                .download(&feed, &updates, &mut |_, _| {})
                .unwrap_err()
        };
        assert!(only(&other).en.contains("SHA-256 does not match"));
        assert!(only(&long).en.contains("larger than update.json"));
        assert!(only(&short).en.contains("not the 10 update.json says"));
        assert!(!updates.join("SI-Future-HUD-0.1.9.dmg").exists());
        assert!(!updates.join("SI-Future-HUD-0.1.9.dmg.part").exists());
    }

    #[test]
    fn every_failed_source_is_named() {
        let failures = [
            (
                "https://pages.example.invalid/update.json",
                text("甲", "first"),
            ),
            (
                "https://raw.example.invalid/update.json",
                text("乙", "second"),
            ),
        ];
        let joined = joined(&failures);
        assert_eq!(
            joined.en,
            "pages.example.invalid: first; raw.example.invalid: second"
        );
        assert_eq!(
            joined.zh,
            "pages.example.invalid：甲；raw.example.invalid：乙"
        );
    }
}
