//! Read external or legacy SQLite databases in standard read-only mode.
use rusqlite::{types::ValueRef, Connection, OpenFlags};
use serde_json::Value;
use std::{path::Path, time::Duration};

pub struct ReadOnlyDatabase {
    pub connection: Connection,
}

impl ReadOnlyDatabase {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        // A writer holding the database delays a read by at most 250 ms, as
        // Open's ReadOnlySQLite waits, so a busy client never holds the
        // collection pass.
        connection.busy_timeout(Duration::from_millis(250))?;
        Ok(Self { connection })
    }
}

/// Why a read-only query has no rows to give.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseFailure {
    /// No regular file is there.
    Missing,
    /// The file could not be opened as a database.
    Unopenable,
    /// The connection could not be set up to wait for a writer.
    Unreadable,
    /// The query does not fit the database, or a writer held it too long.
    Unrecognized,
}

/// Rows of a read-only query on the database at `path`, each column as JSON:
/// text as a string, numbers as numbers, null, blobs and text that is not
/// UTF-8 as null. A writer holding the database delays the query at most
/// `wait`.
pub fn rows(
    path: &Path,
    sql: &str,
    params: &[&str],
    wait: Duration,
) -> Result<Vec<Vec<Value>>, DatabaseFailure> {
    if !path.is_file() {
        return Err(DatabaseFailure::Missing);
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| DatabaseFailure::Unopenable)?;
    connection
        .busy_timeout(wait)
        .map_err(|_| DatabaseFailure::Unreadable)?;
    let mut statement = connection
        .prepare(sql)
        .map_err(|_| DatabaseFailure::Unrecognized)?;
    let columns = statement.column_count();
    let mut rows = statement
        .query(rusqlite::params_from_iter(params))
        .map_err(|_| DatabaseFailure::Unrecognized)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|_| DatabaseFailure::Unrecognized)? {
        let mut values = Vec::with_capacity(columns);
        for index in 0..columns {
            values.push(match row.get_ref(index) {
                Ok(ValueRef::Integer(n)) => Value::from(n),
                Ok(ValueRef::Real(n)) => {
                    serde_json::Number::from_f64(n).map_or(Value::Null, Value::Number)
                }
                Ok(ValueRef::Text(bytes)) => std::str::from_utf8(bytes)
                    .map_or(Value::Null, |text| Value::String(text.to_owned())),
                _ => Value::Null,
            });
        }
        out.push(values);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Open reopens a WAL database whose `-wal` and `-shm` are gone with
    /// `immutable=1` (ReadOnlySQLite); the bundled SQLite reads such a
    /// database read-only as it is, even in a folder it cannot write.
    #[test]
    fn a_wal_database_at_rest_reads_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("client.db");
        {
            let writer = Connection::open(&path).unwrap();
            writer.pragma_update(None, "journal_mode", "WAL").unwrap();
            writer.execute("CREATE TABLE t(a)", []).unwrap();
            writer.execute("INSERT INTO t VALUES (1)", []).unwrap();
        }
        assert!(
            !dir.path().join("client.db-wal").exists()
                && !dir.path().join("client.db-shm").exists()
        );
        let count = |path: &Path| {
            ReadOnlyDatabase::open(path).and_then(|db| {
                db.connection
                    .query_row("SELECT count(*) FROM t", [], |row| row.get::<_, i64>(0))
            })
        };
        assert_eq!(count(&path), Ok(1));
        let mut permissions = std::fs::metadata(dir.path()).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(dir.path(), permissions.clone()).unwrap();
        let read_only_folder = count(&path);
        // Puts the temporary folder back so it can be removed.
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        std::fs::set_permissions(dir.path(), permissions).unwrap();
        assert_eq!(read_only_folder, Ok(1));
    }
}
