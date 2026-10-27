//! One-time, account-exact quota history import. The original ledger is read-only;
//! usage events and its local contribution IDs are deliberately not copied.
use crate::legacy_database::ReadOnlyDatabase;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::Path;

const APPLE_REFERENCE_SECONDS: f64 = 978_307_200.0;
const RETENTION_MS: i64 = 30 * 86_400_000;

/// `pairs` contains only account-scoped window identities established by a
/// provider's successful identity read, never labels or approximate matches.
pub(crate) fn import(
    connection: &Connection,
    legacy_path: &Path,
    pairs: &[(String, String, String)],
) -> rusqlite::Result<usize> {
    if pairs.is_empty() || !legacy_path.is_file() {
        return Ok(0);
    }
    let canonical = legacy_path
        .canonicalize()
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let source_key = canonical.to_string_lossy();
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS legacy_quota_import (
            legacy_path TEXT NOT NULL,
            window_id TEXT NOT NULL,
            legacy_scope TEXT NOT NULL,
            legacy_window_id TEXT NOT NULL,
            imported_at_ms INTEGER NOT NULL,
            PRIMARY KEY (legacy_path, window_id, legacy_scope, legacy_window_id)
        ) WITHOUT ROWID;",
    )?;
    let mut pending = Vec::new();
    for (window, scope, legacy_window) in pairs {
        // The original ProviderAccount.windowID contract. A bare provider window
        // is insufficient evidence to assign historical observations to a login.
        let valid = legacy_window
            .strip_prefix("account:")
            .and_then(|rest| rest.split_once('/'))
            .is_some_and(|(identity, window)| {
                identity.len() == 64
                    && identity.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && !window.is_empty()
            });
        if !valid || window.is_empty() || scope.is_empty() {
            continue;
        }
        let done: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM legacy_quota_import
             WHERE legacy_path=?1 AND window_id=?2 AND legacy_scope=?3 AND legacy_window_id=?4)",
            params![source_key.as_ref(), window, scope, legacy_window],
            |row| row.get(0),
        )?;
        if !done {
            pending.push((window, scope, legacy_window));
        }
    }
    if pending.is_empty() {
        return Ok(0);
    }

    let source = ReadOnlyDatabase::open(&canonical)?;
    let now = Utc::now().timestamp_millis();
    let cutoff = now - RETENTION_MS;
    let since_reference = cutoff as f64 / 1000.0 - APPLE_REFERENCE_SECONDS;
    let now_reference = now as f64 / 1000.0 - APPLE_REFERENCE_SECONDS;
    let mut read = source.connection.prepare(
        "SELECT observed_at, remaining FROM quota_sample
         WHERE scope=?1 AND window_id=?2 AND observed_at>=?3 AND observed_at<=?4
         ORDER BY observed_at",
    )?;
    let transaction = connection.unchecked_transaction()?;
    let mut inserted = 0;
    for (window, scope, legacy_window) in pending {
        let samples = read.query_map(
            params![scope, legacy_window, since_reference, now_reference],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?)),
        )?;
        for sample in samples {
            let (reference, remaining) = sample?;
            if !reference.is_finite()
                || !remaining.is_finite()
                || !(0.0..=100.0).contains(&remaining)
            {
                continue;
            }
            let observed = ((reference + APPLE_REFERENCE_SECONDS) * 1000.0).round() as i64;
            if observed < cutoff || observed > now {
                continue;
            }
            inserted += transaction.execute(
                "INSERT OR IGNORE INTO quota_sample(window_id, observed_at_ms, remaining)
                 VALUES (?1, ?2, ?3)",
                params![window, observed, remaining],
            )?;
        }
        transaction.execute(
            "INSERT OR IGNORE INTO legacy_quota_import(legacy_path, window_id, legacy_scope, legacy_window_id, imported_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![source_key.as_ref(), window, scope, legacy_window, now],
        )?;
    }
    transaction.commit()?;
    Ok(inserted)
}
