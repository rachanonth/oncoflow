use serde::Serialize;
use tauri::State;

use crate::{
    auth::{AuthError, AuthSession},
    db::Database,
};

use super::{
    BackupResult, Diagnostics, RecoveryError, RecoveryService, RestoreInput, RestorePreflight,
    RestoreResult, StartupState, StartupStatus,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
}

impl From<RecoveryError> for CommandError {
    fn from(error: RecoveryError) -> Self {
        match error {
            RecoveryError::Auth(AuthError::AuthenticationRequired) => Self::plain(
                "authentication_required",
                "Sign in with a local OncoFlow account to continue.",
            ),
            RecoveryError::DatabaseNotReady => Self::plain(
                "database_not_ready",
                "The local database is not ready. Use the recovery screen or retry startup.",
            ),
            RecoveryError::InvalidDestination | RecoveryError::Io(_, _) => Self::plain(
                "backup_destination_unavailable",
                "OncoFlow could not write to the selected location. Choose another writable folder.",
            ),
            RecoveryError::InvalidBackup(reason) => Self::plain("invalid_backup", reason),
            RecoveryError::UnsupportedFutureSchema { found, supported } => Self::plain(
                "unsupported_future_schema",
                format!(
                    "This backup uses schema version {found}; this OncoFlow release supports up to version {supported}. Install a newer OncoFlow version to restore it."
                ),
            ),
            RecoveryError::UnsupportedOldSchema(_) => Self::plain(
                "unsupported_old_schema",
                "This backup predates the oldest migration path supported by OncoFlow.",
            ),
            RecoveryError::RestoreConfirmationMismatch => Self::plain(
                "restore_confirmation_mismatch",
                "The backup changed after preflight or restore was not explicitly confirmed. Run preflight again.",
            ),
            RecoveryError::RecoveryBackupRequired => Self::plain(
                "recovery_backup_required",
                "Restore was stopped because the current database could not first be backed up and validated.",
            ),
            RecoveryError::RestoreFailedRecovered => Self::plain(
                "restore_failed_recovered",
                "Restore did not complete. The validated pre-restore database was preserved or recovered.",
            ),
            RecoveryError::RestoreRecoveryFailed => Self::plain(
                "restore_recovery_failed",
                "Restore failed and OncoFlow could not automatically recover the active database. Exit and preserve the data folder before taking further action.",
            ),
            RecoveryError::Auth(_)
            | RecoveryError::Database(_)
            | RecoveryError::Sqlite(_)
            | RecoveryError::Serialization => Self::plain(
                "database_recovery_error",
                "The local backup or restore operation could not be completed safely.",
            ),
        }
    }
}

impl CommandError {
    fn plain(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[tauri::command]
pub(crate) fn get_startup_status(
    database: State<'_, Database>,
    startup: State<'_, StartupState>,
) -> StartupStatus {
    startup.status(&database)
}

#[tauri::command]
pub(crate) fn retry_database_initialization(
    database: State<'_, Database>,
    startup: State<'_, StartupState>,
) -> Result<StartupStatus, CommandError> {
    match Database::initialize(database.path()) {
        Ok(_) => {
            startup.mark_ready();
            Ok(startup.status(&database))
        }
        Err(error) => {
            startup.mark_failed(&error);
            Ok(startup.status(&database))
        }
    }
}

#[tauri::command]
pub(crate) fn create_database_backup(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    startup: State<'_, StartupState>,
    destination_directory: String,
) -> Result<BackupResult, CommandError> {
    RecoveryService::new(&database, &session, &startup)
        .create_backup(destination_directory)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn preflight_database_restore(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    startup: State<'_, StartupState>,
    backup_path: String,
) -> Result<RestorePreflight, CommandError> {
    RecoveryService::new(&database, &session, &startup)
        .preflight_restore(backup_path)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn restore_database(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    startup: State<'_, StartupState>,
    input: RestoreInput,
) -> Result<RestoreResult, CommandError> {
    RecoveryService::new(&database, &session, &startup)
        .restore(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_diagnostics(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    startup: State<'_, StartupState>,
) -> Result<Diagnostics, CommandError> {
    RecoveryService::new(&database, &session, &startup)
        .diagnostics()
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn open_data_folder(database: State<'_, Database>) -> Result<(), CommandError> {
    let directory = database.path().parent().ok_or_else(|| {
        CommandError::plain(
            "data_folder_unavailable",
            "The OncoFlow data folder is unavailable.",
        )
    })?;
    std::fs::create_dir_all(directory).map_err(|_| {
        CommandError::plain(
            "data_folder_unavailable",
            "The OncoFlow data folder could not be opened.",
        )
    })?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(directory)
            .spawn()
            .map_err(|_| {
                CommandError::plain(
                    "data_folder_unavailable",
                    "Windows Explorer could not open the OncoFlow data folder.",
                )
            })?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err(CommandError::plain(
            "unsupported_platform",
            "Open data folder is available in the Windows desktop build.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_command_errors_do_not_expose_sensitive_sources_or_paths() {
        let error = CommandError::from(RecoveryError::Io(
            "synthetic operation",
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "SecretPassword Patient Name C:\\Sensitive\\oncoflow.db",
            ),
        ));
        let serialized = serde_json::to_string(&error).unwrap();
        assert!(!serialized.contains("SecretPassword"));
        assert!(!serialized.contains("Patient Name"));
        assert!(!serialized.contains("Sensitive"));
        assert_eq!(error.code, "backup_destination_unavailable");
    }
}
