use std::sync::Mutex;

use rusqlite::ErrorCode;

use crate::db::{Database, DatabaseError};

use super::{DatabaseIssue, StartupStatus};

#[derive(Debug)]
pub(crate) struct StartupState {
    issue: Mutex<Option<DatabaseIssue>>,
}

impl StartupState {
    pub(crate) fn ready() -> Self {
        Self {
            issue: Mutex::new(None),
        }
    }

    pub(crate) fn failed(error: &DatabaseError) -> Self {
        Self {
            issue: Mutex::new(Some(issue_from_error(error))),
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.issue
            .lock()
            .map(|issue| issue.is_none())
            .unwrap_or(false)
    }

    pub(crate) fn mark_ready(&self) {
        if let Ok(mut issue) = self.issue.lock() {
            *issue = None;
        }
    }

    pub(crate) fn mark_failed(&self, error: &DatabaseError) {
        if let Ok(mut issue) = self.issue.lock() {
            *issue = Some(issue_from_error(error));
        }
    }

    pub(crate) fn status(&self, database: &Database) -> StartupStatus {
        let issue = self.issue.lock().ok().and_then(|value| value.clone());
        StartupStatus {
            database_ready: issue.is_none(),
            database_location: database.path().display().to_string(),
            issue,
        }
    }
}

fn issue_from_error(error: &DatabaseError) -> DatabaseIssue {
    let (code, title, message) = match error {
        DatabaseError::UnsupportedSchemaVersion { .. } => (
            "unsupported_schema",
            "Newer database version",
            "This database was created by a newer OncoFlow release. Install a compatible version before opening it.",
        ),
        DatabaseError::UnrecognizedDatabase => (
            "unrecognized_database",
            "Database not recognized",
            "The existing file is not a recognizable OncoFlow database. It was not replaced or modified.",
        ),
        DatabaseError::IntegrityCheckFailed
        | DatabaseError::ForeignKeyViolations(_)
        | DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase,
                ..
            },
            _,
        )) => (
            "database_corrupt",
            "Database integrity problem",
            "OncoFlow did not continue with the damaged database. Restore a validated backup or open the data folder for recovery.",
        ),
        DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked,
                ..
            },
            _,
        )) => (
            "database_locked",
            "Database is in use",
            "Close any other OncoFlow or SQLite process using the database, then retry.",
        ),
        DatabaseError::MigrationBackup(_) => (
            "migration_safety_failed",
            "Migration paused safely",
            "OncoFlow could not create and validate its pre-migration recovery copy, so no migration was attempted.",
        ),
        DatabaseError::MigrationFailed(_) => (
            "migration_failed",
            "Database migration failed",
            "The migration did not complete. The pre-migration recovery copy is preserved and the clinical workspace remains closed.",
        ),
        _ => (
            "database_unavailable",
            "Database unavailable",
            "OncoFlow could not open or initialize the local database. Retry or use the recovery tools.",
        ),
    };
    DatabaseIssue {
        code: code.into(),
        title: title.into(),
        message: message.into(),
    }
}
