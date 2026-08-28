use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{
    backup::{Backup, Progress},
    Connection, OpenFlags, TransactionBehavior, MAIN_DB,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    auth::{audit, AuthError, AuthSession},
    clinical::LEGACY_RULESET,
    db::{
        apply_migrations, configure_connection, read_schema_version, validate_connection, Database,
        DatabaseError, LATEST_SCHEMA_VERSION, MIN_SUPPORTED_SCHEMA_VERSION, RECOVERY_DIRECTORY,
    },
    hardware::LABEL_RENDERER_VERSION,
    output::PREPARATION_LABEL_TEMPLATE_VERSION,
};

use super::{
    BackupManifest, BackupResult, Diagnostics, RestoreInput, RestorePreflight, RestoreResult,
    StartupState,
};

const BACKUP_FORMAT_VERSION: u32 = 1;
const RESTORE_TOKEN_VERSION: &str = "oncoflow-restore-confirmation-v1";
const APP_NAME: &str = "OncoFlow";
const APPLICATION_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Error)]
pub(crate) enum RecoveryError {
    #[error("the local database is not ready")]
    DatabaseNotReady,
    #[error("the selected backup destination is invalid")]
    InvalidDestination,
    #[error("the selected file is not a valid OncoFlow backup: {0}")]
    InvalidBackup(&'static str),
    #[error("backup schema {found} is newer than supported schema {supported}")]
    UnsupportedFutureSchema { found: i64, supported: i64 },
    #[error("backup schema {0} is too old to migrate safely")]
    UnsupportedOldSchema(i64),
    #[error("the restore confirmation is missing or no longer matches the selected file")]
    RestoreConfirmationMismatch,
    #[error("the active database recovery backup could not be created and validated")]
    RecoveryBackupRequired,
    #[error("the restore failed; the active database was recovered")]
    RestoreFailedRecovered,
    #[error("the restore failed and automatic recovery also failed")]
    RestoreRecoveryFailed,
    #[error("a filesystem operation failed: {0}")]
    Io(&'static str, #[source] std::io::Error),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("backup metadata could not be serialized")]
    Serialization,
}

#[derive(Debug, Clone)]
struct InspectedDatabase {
    path: PathBuf,
    file_name: String,
    schema_version: i64,
    size_bytes: u64,
    sha256: String,
    created_at: Option<String>,
    backup_application_version: Option<String>,
}

pub(crate) struct RecoveryService<'a> {
    database: &'a Database,
    session: &'a AuthSession,
    startup: &'a StartupState,
}

impl<'a> RecoveryService<'a> {
    pub(crate) fn new(
        database: &'a Database,
        session: &'a AuthSession,
        startup: &'a StartupState,
    ) -> Self {
        Self {
            database,
            session,
            startup,
        }
    }

    pub(crate) fn create_backup(
        &self,
        destination_directory: impl AsRef<Path>,
    ) -> Result<BackupResult, RecoveryError> {
        if !self.startup.is_ready() {
            return Err(RecoveryError::DatabaseNotReady);
        }
        let actor = self.session.require_user()?;
        let destination_directory = destination_directory.as_ref();
        if !destination_directory.is_dir() {
            return Err(RecoveryError::InvalidDestination);
        }

        let source = self.database.open()?;
        validate_connection(&source)?;
        let schema_version = read_schema_version(&source)?
            .ok_or(RecoveryError::InvalidBackup("schema metadata is missing"))?;
        let (created_at, file_stamp) = timestamp_pair(&source)?;
        let destination = unique_path(
            destination_directory,
            &format!("OncoFlow_Backup_{file_stamp}"),
        );
        let manifest_path = manifest_path(&destination);

        if let Err(error) = online_snapshot(&source, &destination) {
            cleanup_generated(&destination, &manifest_path);
            return Err(error);
        }
        let inspected = match inspect_database(&destination, false) {
            Ok(value) => value,
            Err(error) => {
                cleanup_generated(&destination, &manifest_path);
                return Err(error);
            }
        };
        let manifest = BackupManifest {
            format_version: BACKUP_FORMAT_VERSION,
            application_name: APP_NAME.into(),
            application_version: APPLICATION_VERSION.into(),
            schema_version,
            created_at: created_at.clone(),
            database_file: inspected.file_name.clone(),
            database_size_bytes: inspected.size_bytes,
            sha256: inspected.sha256.clone(),
            integrity_check: "ok".into(),
            foreign_key_violations: 0,
        };
        if let Err(error) = write_manifest(&manifest_path, &manifest) {
            cleanup_generated(&destination, &manifest_path);
            return Err(error);
        }

        let mut audit_connection = self.database.open()?;
        let transaction =
            audit_connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Err(error) = audit::append_event(
            &transaction,
            Some(actor.id),
            "database_backup_created",
            "database",
            "oncoflow",
            &json!({
                "backup_file": inspected.file_name,
                "schema_version": schema_version,
                "sha256": inspected.sha256,
                "integrity": "ok"
            }),
        )
        .and_then(|_| transaction.commit())
        {
            cleanup_generated(&destination, &manifest_path);
            return Err(RecoveryError::Sqlite(error));
        }

        Ok(BackupResult {
            location: destination.display().to_string(),
            manifest_location: manifest_path.display().to_string(),
            file_name: inspected.file_name,
            created_at,
            schema_version,
            application_version: APPLICATION_VERSION.into(),
            integrity_check: "ok".into(),
            foreign_key_violations: 0,
            sha256: inspected.sha256,
            size_bytes: inspected.size_bytes,
        })
    }

    pub(crate) fn preflight_restore(
        &self,
        backup_path: impl AsRef<Path>,
    ) -> Result<RestorePreflight, RecoveryError> {
        self.authorize_restore()?;
        let inspected = inspect_database(backup_path.as_ref(), false)?;
        if same_file(&inspected.path, self.database.path()) {
            return Err(RecoveryError::InvalidBackup(
                "the active database cannot be selected as its own backup",
            ));
        }
        Ok(preflight_from_inspection(inspected))
    }

    pub(crate) fn restore(&self, input: RestoreInput) -> Result<RestoreResult, RecoveryError> {
        let actor = self.authorize_restore()?;
        if !input.confirmed {
            return Err(RecoveryError::RestoreConfirmationMismatch);
        }
        let inspected = inspect_database(Path::new(&input.backup_path), false)?;
        let expected_token = confirmation_token(&inspected);
        if !input
            .expected_sha256
            .eq_ignore_ascii_case(&inspected.sha256)
            || input.confirmation_token != expected_token
            || same_file(&inspected.path, self.database.path())
        {
            return Err(RecoveryError::RestoreConfirmationMismatch);
        }

        let data_directory = self
            .database
            .path()
            .parent()
            .ok_or(RecoveryError::InvalidDestination)?;
        let staging_directory = data_directory.join(RECOVERY_DIRECTORY).join("staging");
        fs::create_dir_all(&staging_directory)
            .map_err(|error| RecoveryError::Io("create restore staging directory", error))?;
        let stamp = timestamp_pair(&Connection::open_in_memory()?)?.1;
        let staged = unique_path(&staging_directory, &format!("restore_stage_{stamp}"));
        let staged_manifest = manifest_path(&staged);
        let result = self.restore_inner(actor.as_ref().map(|user| user.id), &inspected, &staged);
        cleanup_generated(&staged, &staged_manifest);
        result
    }

    pub(crate) fn diagnostics(&self) -> Result<Diagnostics, RecoveryError> {
        if !self.startup.is_ready() {
            return Err(RecoveryError::DatabaseNotReady);
        }
        self.session.require_user()?;
        let connection = self.database.open()?;
        validate_connection(&connection)?;
        let schema_version = read_schema_version(&connection)?
            .ok_or(RecoveryError::InvalidBackup("schema metadata is missing"))?;
        let last_backup_at = connection.query_row(
            "SELECT MAX(occurred_at) FROM audit_events WHERE event_type='database_backup_created'",
            [],
            |row| row.get::<_, Option<String>>(0),
        )?;
        let database_size_bytes = self
            .database
            .path()
            .metadata()
            .map_err(|error| RecoveryError::Io("read database metadata", error))?
            .len();
        Ok(Diagnostics {
            application_name: APP_NAME.into(),
            application_version: APPLICATION_VERSION.into(),
            schema_version,
            clinical_ruleset_version: LEGACY_RULESET.into(),
            label_template_version: PREPARATION_LABEL_TEMPLATE_VERSION.into(),
            label_renderer_version: LABEL_RENDERER_VERSION.into(),
            database_location: self.database.path().display().to_string(),
            database_size_bytes,
            integrity_check: "ok".into(),
            foreign_key_violations: 0,
            last_backup_at,
            platform: std::env::consts::OS.into(),
            automatic_backup_policy:
                "Validated pre-migration recovery copies only; manual backups are operator controlled."
                    .into(),
        })
    }

    fn authorize_restore(&self) -> Result<Option<crate::auth::CurrentUser>, RecoveryError> {
        if self.startup.is_ready() {
            Ok(Some(self.session.require_user()?))
        } else {
            Ok(None)
        }
    }

    fn restore_inner(
        &self,
        actor_user_id: Option<i64>,
        candidate: &InspectedDatabase,
        staged_path: &Path,
    ) -> Result<RestoreResult, RecoveryError> {
        let candidate_connection = open_read_only(&candidate.path)?;
        online_snapshot(&candidate_connection, staged_path)?;
        drop(candidate_connection);

        let staged_database = Database::at_path(staged_path);
        let staged_connection = staged_database.open()?;
        apply_migrations(&staged_connection)?;
        validate_connection(&staged_connection)?;
        let staged_schema = read_schema_version(&staged_connection)?.ok_or(
            RecoveryError::InvalidBackup("staged schema metadata is missing"),
        )?;
        if staged_schema != LATEST_SCHEMA_VERSION {
            return Err(RecoveryError::InvalidBackup(
                "staged database did not reach the supported schema",
            ));
        }
        drop(staged_connection);

        if let Some(actor_user_id) = actor_user_id {
            let mut connection = self.database.open()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            audit::append_event(
                &transaction,
                Some(actor_user_id),
                "database_restore_started",
                "database",
                "oncoflow",
                &json!({
                    "backup_file": candidate.file_name,
                    "backup_schema_version": candidate.schema_version,
                    "sha256": candidate.sha256
                }),
            )?;
            transaction.commit()?;
        }

        let recovery = self
            .create_recovery_backup()
            .map_err(|_| RecoveryError::RecoveryBackupRequired)?;
        if restore_path_into_database(self.database, staged_path).is_err() {
            if restore_path_into_database(self.database, &recovery.location).is_err() {
                return Err(RecoveryError::RestoreRecoveryFailed);
            }
            return Err(RecoveryError::RestoreFailedRecovered);
        }

        let finalize = (|| -> Result<(), RecoveryError> {
            let mut connection = self.database.open()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            audit::append_event(
                &transaction,
                None,
                "database_restore_completed",
                "database",
                "oncoflow",
                &json!({
                    "backup_file": candidate.file_name,
                    "source_schema_version": candidate.schema_version,
                    "restored_schema_version": staged_schema,
                    "sha256": candidate.sha256,
                    "recovery_backup_file": recovery.file_name
                }),
            )?;
            transaction.commit()?;
            validate_connection(&connection)?;
            Ok(())
        })();

        if finalize.is_err() {
            if restore_path_into_database(self.database, &recovery.location).is_err() {
                return Err(RecoveryError::RestoreRecoveryFailed);
            }
            return Err(RecoveryError::RestoreFailedRecovered);
        }

        self.session.invalidate()?;
        self.startup.mark_ready();
        Ok(RestoreResult {
            restored_schema_version: staged_schema,
            migrated_from_schema_version: (candidate.schema_version < staged_schema)
                .then_some(candidate.schema_version),
            recovery_backup_location: recovery.location.display().to_string(),
            recovery_backup_sha256: recovery.sha256,
            restored_backup_sha256: candidate.sha256.clone(),
            session_cleared: true,
            restart_required: true,
        })
    }

    fn create_recovery_backup(&self) -> Result<InternalBackup, RecoveryError> {
        let source = self.database.open()?;
        validate_connection(&source)?;
        let schema_version = read_schema_version(&source)?.ok_or(RecoveryError::InvalidBackup(
            "active schema metadata is missing",
        ))?;
        let data_directory = self
            .database
            .path()
            .parent()
            .ok_or(RecoveryError::InvalidDestination)?;
        let recovery_directory = data_directory.join(RECOVERY_DIRECTORY).join("restore");
        fs::create_dir_all(&recovery_directory)
            .map_err(|error| RecoveryError::Io("create recovery directory", error))?;
        let (created_at, stamp) = timestamp_pair(&source)?;
        let destination = unique_path(&recovery_directory, &format!("pre_restore_{stamp}"));
        online_snapshot(&source, &destination)?;
        let inspected = inspect_database(&destination, true)?;
        let manifest = BackupManifest {
            format_version: BACKUP_FORMAT_VERSION,
            application_name: APP_NAME.into(),
            application_version: APPLICATION_VERSION.into(),
            schema_version,
            created_at,
            database_file: inspected.file_name.clone(),
            database_size_bytes: inspected.size_bytes,
            sha256: inspected.sha256.clone(),
            integrity_check: "ok".into(),
            foreign_key_violations: 0,
        };
        write_manifest(&manifest_path(&destination), &manifest)?;
        Ok(InternalBackup {
            location: destination,
            file_name: inspected.file_name,
            sha256: inspected.sha256,
        })
    }
}

#[derive(Debug)]
struct InternalBackup {
    location: PathBuf,
    file_name: String,
    sha256: String,
}

fn preflight_from_inspection(inspected: InspectedDatabase) -> RestorePreflight {
    let confirmation_token = confirmation_token(&inspected);
    RestorePreflight {
        location: inspected.path.display().to_string(),
        file_name: inspected.file_name,
        schema_version: inspected.schema_version,
        supported_schema_version: LATEST_SCHEMA_VERSION,
        requires_migration: inspected.schema_version < LATEST_SCHEMA_VERSION,
        created_at: inspected.created_at,
        backup_application_version: inspected.backup_application_version,
        integrity_check: "ok".into(),
        foreign_key_violations: 0,
        sha256: inspected.sha256.clone(),
        size_bytes: inspected.size_bytes,
        confirmation_token,
    }
}

fn inspect_database(path: &Path, allow_future: bool) -> Result<InspectedDatabase, RecoveryError> {
    if !path.is_file() {
        return Err(RecoveryError::InvalidBackup(
            "the selected file does not exist",
        ));
    }
    let connection = open_read_only(path)
        .map_err(|_| RecoveryError::InvalidBackup("SQLite could not open the file"))?;
    validate_connection(&connection)
        .map_err(|_| RecoveryError::InvalidBackup("database validation failed"))?;
    let schema_version = read_schema_version(&connection)
        .map_err(|_| RecoveryError::InvalidBackup("schema metadata is invalid"))?
        .ok_or(RecoveryError::InvalidBackup("schema metadata is missing"))?;
    if schema_version < MIN_SUPPORTED_SCHEMA_VERSION {
        return Err(RecoveryError::UnsupportedOldSchema(schema_version));
    }
    if !allow_future && schema_version > LATEST_SCHEMA_VERSION {
        return Err(RecoveryError::UnsupportedFutureSchema {
            found: schema_version,
            supported: LATEST_SCHEMA_VERSION,
        });
    }
    if !recognizable_oncoflow_schema(&connection)? {
        return Err(RecoveryError::InvalidBackup(
            "required OncoFlow tables are missing",
        ));
    }
    drop(connection);

    let size_bytes = path
        .metadata()
        .map_err(|error| RecoveryError::Io("read backup metadata", error))?
        .len();
    let sha256 = sha256_file(path)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(RecoveryError::InvalidBackup(
            "the backup filename is invalid",
        ))?
        .to_owned();
    let mut created_at = None;
    let mut backup_application_version = None;
    let sidecar = manifest_path(path);
    if sidecar.is_file() {
        let raw =
            fs::read(&sidecar).map_err(|error| RecoveryError::Io("read backup manifest", error))?;
        let manifest: BackupManifest = serde_json::from_slice(&raw)
            .map_err(|_| RecoveryError::InvalidBackup("the backup manifest is invalid"))?;
        if manifest.format_version != BACKUP_FORMAT_VERSION
            || manifest.application_name != APP_NAME
            || manifest.database_file != file_name
            || manifest.database_size_bytes != size_bytes
            || manifest.schema_version != schema_version
            || !manifest.sha256.eq_ignore_ascii_case(&sha256)
            || manifest.integrity_check != "ok"
            || manifest.foreign_key_violations != 0
        {
            return Err(RecoveryError::InvalidBackup(
                "the backup manifest does not match the database",
            ));
        }
        created_at = Some(manifest.created_at);
        backup_application_version = Some(manifest.application_version);
    }

    Ok(InspectedDatabase {
        path: path.to_path_buf(),
        file_name,
        schema_version,
        size_bytes,
        sha256,
        created_at,
        backup_application_version,
    })
}

fn recognizable_oncoflow_schema(connection: &Connection) -> Result<bool, RecoveryError> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type='table' AND name IN ('app_meta','users','patients','drugs','orders')",
        [],
        |row| row.get(0),
    )?;
    Ok(count == 5)
}

fn open_read_only(path: &Path) -> Result<Connection, RecoveryError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    configure_connection(&connection)?;
    Ok(connection)
}

fn online_snapshot(source: &Connection, destination: &Path) -> Result<(), RecoveryError> {
    if destination.exists() {
        return Err(RecoveryError::InvalidDestination);
    }
    let mut output = Connection::open_with_flags(
        destination,
        OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE,
    )?;
    {
        let backup = Backup::new(source, &mut output)?;
        backup.run_to_completion(128, Duration::from_millis(10), None)?;
    }
    configure_connection(&output)?;
    validate_connection(&output)?;
    output.close().map_err(|(_, error)| error)?;
    Ok(())
}

fn restore_path_into_database(database: &Database, source: &Path) -> Result<(), RecoveryError> {
    let mut active = database.open()?;
    active.restore(MAIN_DB, source, None::<fn(Progress)>)?;
    configure_connection(&active)?;
    validate_connection(&active)?;
    Ok(())
}

fn confirmation_token(inspected: &InspectedDatabase) -> String {
    let mut hasher = Sha256::new();
    hasher.update(RESTORE_TOKEN_VERSION.as_bytes());
    hasher.update([0]);
    hasher.update(inspected.sha256.as_bytes());
    hasher.update([0]);
    hasher.update(inspected.schema_version.to_string().as_bytes());
    hasher.update([0]);
    hasher.update(inspected.size_bytes.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn sha256_file(path: &Path) -> Result<String, RecoveryError> {
    let mut file = File::open(path).map_err(|error| RecoveryError::Io("open backup", error))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| RecoveryError::Io("read backup", error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn write_manifest(path: &Path, manifest: &BackupManifest) -> Result<(), RecoveryError> {
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|_| RecoveryError::Serialization)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| RecoveryError::Io("create backup manifest", error))?;
    file.write_all(&bytes)
        .map_err(|error| RecoveryError::Io("write backup manifest", error))?;
    file.sync_all()
        .map_err(|error| RecoveryError::Io("flush backup manifest", error))?;
    Ok(())
}

fn timestamp_pair(connection: &Connection) -> Result<(String, String), RecoveryError> {
    connection
        .query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now'),
                    strftime('%Y-%m-%d_%H%M%S','now','localtime')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(Into::into)
}

fn unique_path(directory: &Path, stem: &str) -> PathBuf {
    for suffix in 0..10_000 {
        let name = if suffix == 0 {
            format!("{stem}.db")
        } else {
            format!("{stem}_{suffix:02}.db")
        };
        let candidate = directory.join(name);
        if !candidate.exists() && !manifest_path(&candidate).exists() {
            return candidate;
        }
    }
    directory.join(format!("{stem}_overflow.db"))
}

fn manifest_path(database_path: &Path) -> PathBuf {
    let mut value = database_path.as_os_str().to_os_string();
    value.push(".manifest.json");
    PathBuf::from(value)
}

fn cleanup_generated(database_path: &Path, manifest: &Path) {
    let _ = fs::remove_file(manifest);
    let _ = fs::remove_file(database_path);
}

fn same_file(first: &Path, second: &Path) -> bool {
    match (first.canonicalize(), second.canonicalize()) {
        (Ok(first), Ok(second)) => first == second,
        _ => first == second,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthService, BootstrapUserInput};

    const PASSWORD: &str = "Synthetic-Password-123!";

    struct Fixture {
        _directory: tempfile::TempDir,
        database: Database,
        session: AuthSession,
        startup: StartupState,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
            let session = AuthSession::default();
            AuthService::new(&database, &session)
                .bootstrap(BootstrapUserInput {
                    username: "active.admin".into(),
                    display_name: "Synthetic Active Admin".into(),
                    password: PASSWORD.into(),
                })
                .unwrap();
            Self {
                _directory: directory,
                database,
                session,
                startup: StartupState::ready(),
            }
        }

        fn service(&self) -> RecoveryService<'_> {
            RecoveryService::new(&self.database, &self.session, &self.startup)
        }

        fn candidate(&self, name: &str, username: &str) -> PathBuf {
            let donor_path = self._directory.path().join(format!("{name}_donor.db"));
            let donor = Database::initialize(&donor_path).unwrap();
            donor.open().unwrap().execute(
                "INSERT INTO users(username,display_name,password_hash,role,active,credential_kind,updated_at)
                 VALUES(?1,'Synthetic Restored User','$argon2id$synthetic','admin',1,'argon2id',CURRENT_TIMESTAMP)",
                [username],
            ).unwrap();
            donor
                .open()
                .unwrap()
                .execute(
                    "INSERT INTO patients(legacy_hn,first_name) VALUES('SYN-RESTORED','Synthetic')",
                    [],
                )
                .unwrap();
            let candidate = self._directory.path().join(format!("{name}.db"));
            online_snapshot(&donor.open().unwrap(), &candidate).unwrap();
            candidate
        }

        fn restore_input(&self, candidate: &Path) -> RestoreInput {
            let preflight = self.service().preflight_restore(candidate).unwrap();
            RestoreInput {
                backup_path: preflight.location,
                expected_sha256: preflight.sha256,
                confirmation_token: preflight.confirmation_token,
                confirmed: true,
            }
        }
    }

    #[test]
    fn valid_backup_is_complete_validated_checksummed_and_non_clinical() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        connection
            .execute(
                "INSERT INTO patients(legacy_hn,first_name) VALUES('SYN-BACKUP','Synthetic')",
                [],
            )
            .unwrap();
        let before: (i64, i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM patients),
                        (SELECT COUNT(*) FROM orders),
                        (SELECT COUNT(*) FROM inventory_movements)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let audit_before: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .unwrap();
        drop(connection);

        let destination = fixture._directory.path().join("manual");
        fs::create_dir(&destination).unwrap();
        let result = fixture.service().create_backup(&destination).unwrap();
        let inspected = inspect_database(Path::new(&result.location), false).unwrap();
        assert_eq!(inspected.schema_version, LATEST_SCHEMA_VERSION);
        assert_eq!(
            result.sha256,
            sha256_file(Path::new(&result.location)).unwrap()
        );
        assert!(Path::new(&result.manifest_location).is_file());
        let manifest = fs::read_to_string(&result.manifest_location).unwrap();
        assert!(!manifest.contains("SYN-BACKUP"));
        assert!(!manifest.contains(PASSWORD));
        assert!(!manifest.contains("$argon2"));

        let connection = fixture.database.open().unwrap();
        let after: (i64, i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM patients),
                        (SELECT COUNT(*) FROM orders),
                        (SELECT COUNT(*) FROM inventory_movements)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let audit_after: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(before, after);
        assert_eq!(audit_after, audit_before + 1);
    }

    #[test]
    fn invalid_destination_and_corrupt_or_unrelated_files_are_rejected() {
        let fixture = Fixture::new();
        assert!(matches!(
            fixture
                .service()
                .create_backup(fixture._directory.path().join("missing")),
            Err(RecoveryError::InvalidDestination)
        ));
        let corrupt = fixture._directory.path().join("corrupt.db");
        fs::write(&corrupt, b"not a sqlite database").unwrap();
        assert!(matches!(
            fixture.service().preflight_restore(&corrupt),
            Err(RecoveryError::InvalidBackup(_))
        ));
        let unrelated = fixture._directory.path().join("unrelated.db");
        Connection::open(&unrelated)
            .unwrap()
            .execute("CREATE TABLE unrelated(value TEXT)", [])
            .unwrap();
        assert!(matches!(
            fixture.service().preflight_restore(&unrelated),
            Err(RecoveryError::InvalidBackup(_))
        ));
    }

    #[test]
    fn future_schema_is_rejected_and_changed_file_invalidates_confirmation() {
        let fixture = Fixture::new();
        let future = fixture.candidate("future", "future.admin");
        Connection::open(&future)
            .unwrap()
            .execute(
                "UPDATE app_meta SET value='20' WHERE key='schema_version'",
                [],
            )
            .unwrap();
        assert!(matches!(
            fixture.service().preflight_restore(&future),
            Err(RecoveryError::UnsupportedFutureSchema { found: 20, .. })
        ));

        let candidate = fixture.candidate("changed", "changed.admin");
        let input = fixture.restore_input(&candidate);
        Connection::open(&candidate)
            .unwrap()
            .execute("INSERT INTO diagnoses(diagnosis) VALUES('Synthetic')", [])
            .unwrap();
        assert!(matches!(
            fixture.service().restore(input),
            Err(RecoveryError::RestoreConfirmationMismatch)
        ));
        assert!(fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM users WHERE username='active.admin')",
                [],
                |row| row.get::<_, bool>(0)
            )
            .unwrap());
    }

    #[test]
    fn valid_restore_creates_recovery_copy_and_restored_users_are_authoritative() {
        let fixture = Fixture::new();
        let candidate = fixture.candidate("valid_restore", "restored.admin");
        let input = fixture.restore_input(&candidate);
        let result = fixture.service().restore(input).unwrap();
        assert_eq!(result.restored_schema_version, LATEST_SCHEMA_VERSION);
        assert!(result.session_cleared);
        assert!(Path::new(&result.recovery_backup_location).is_file());
        assert_eq!(
            result.recovery_backup_sha256,
            sha256_file(Path::new(&result.recovery_backup_location)).unwrap()
        );
        assert!(fixture.session.current_user().unwrap().is_none());
        let connection = fixture.database.open().unwrap();
        assert_eq!(
            connection
                .query_row("SELECT GROUP_CONCAT(username, ',') FROM users", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
            "restored.admin"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM audit_events WHERE event_type='database_restore_completed' AND user_id IS NULL",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn supported_older_backup_migrates_in_staging_before_restore() {
        let fixture = Fixture::new();
        let candidate = fixture.candidate("schema_eight", "schema8.admin");
        let connection = Connection::open(&candidate).unwrap();
        connection
            .execute_batch(
                "DROP TABLE page_guidance;
                  ALTER TABLE preparation_tasks DROP COLUMN withdrawal_volume_ml;
                  ALTER TABLE preparation_output_snapshots DROP COLUMN withdrawal_volume_ml;
                  ALTER TABLE preparation_output_snapshots DROP COLUMN expiry_storage_text;
                 ALTER TABLE preparation_output_snapshots DROP COLUMN expiry_time_text;
                 ALTER TABLE preparation_output_snapshots DROP COLUMN warning_text;
                 ALTER TABLE preparation_output_snapshots DROP COLUMN hospital_name;
                 ALTER TABLE preparation_tasks DROP COLUMN final_container_count;
                 ALTER TABLE preparation_output_snapshots DROP COLUMN final_container_count;
                 DROP TRIGGER order_status_events_no_update;
                 DROP TRIGGER order_status_events_no_delete;
                 DROP INDEX uq_order_status_event_no_show;
                 DROP INDEX uq_order_status_event_reschedule_source;
                 DROP INDEX uq_order_status_event_reschedule_target;
                 DROP INDEX idx_order_status_events_effective;
                 DROP TABLE order_status_events;
                 DROP INDEX idx_orders_workflow_status;
                 ALTER TABLE orders DROP COLUMN workflow_status_changed_by_user_id;
                 ALTER TABLE orders DROP COLUMN workflow_status_changed_at;
                 ALTER TABLE orders DROP COLUMN workflow_status_reason;
                 ALTER TABLE orders DROP COLUMN workflow_status;
                 DROP INDEX idx_users_management;
                 ALTER TABLE users DROP COLUMN user_type;
                 ALTER TABLE order_items DROP COLUMN diluent_volume_ml;
                 ALTER TABLE orders DROP COLUMN weight_kg;
                 ALTER TABLE orders DROP COLUMN height_cm;
                 DROP INDEX idx_orders_assigned_preparer;
                 ALTER TABLE orders DROP COLUMN assigned_preparer_user_id;
                 UPDATE app_meta SET value='8' WHERE key='schema_version';",
            )
            .unwrap();
        drop(connection);
        let input = fixture.restore_input(&candidate);
        let result = fixture.service().restore(input).unwrap();
        assert_eq!(result.migrated_from_schema_version, Some(8));
        let connection = fixture.database.open().unwrap();
        assert_eq!(
            read_schema_version(&connection).unwrap(),
            Some(LATEST_SCHEMA_VERSION)
        );
        assert!(connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='preparation_output_snapshots')",
                [],
                |row| row.get::<_, bool>(0)
            )
            .unwrap());
    }

    #[test]
    fn failure_after_live_replace_recovers_the_previous_active_database() {
        let fixture = Fixture::new();
        let candidate = fixture.candidate("rollback", "rollback.admin");
        Connection::open(&candidate)
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_restore_audit_failure
                 BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='database_restore_completed'
                 BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        let input = fixture.restore_input(&candidate);
        assert!(matches!(
            fixture.service().restore(input),
            Err(RecoveryError::RestoreFailedRecovered)
        ));
        let connection = fixture.database.open().unwrap();
        assert!(connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM users WHERE username='active.admin')",
                [],
                |row| row.get::<_, bool>(0)
            )
            .unwrap());
        assert!(!connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM users WHERE username='rollback.admin')",
                [],
                |row| row.get::<_, bool>(0)
            )
            .unwrap());
        validate_connection(&connection).unwrap();
        assert!(fixture.session.current_user().unwrap().is_some());
    }

    #[test]
    fn restore_requires_confirmation_and_backup_requires_authentication() {
        let fixture = Fixture::new();
        let candidate = fixture.candidate("confirmation", "confirm.admin");
        let mut input = fixture.restore_input(&candidate);
        input.confirmed = false;
        assert!(matches!(
            fixture.service().restore(input),
            Err(RecoveryError::RestoreConfirmationMismatch)
        ));
        fixture.session.invalidate().unwrap();
        let destination = fixture._directory.path().join("anonymous");
        fs::create_dir(&destination).unwrap();
        assert!(matches!(
            fixture.service().create_backup(&destination),
            Err(RecoveryError::Auth(AuthError::AuthenticationRequired))
        ));
    }

    #[test]
    fn diagnostics_are_aggregate_only_and_report_separate_version_identities() {
        let fixture = Fixture::new();
        let diagnostics = fixture.service().diagnostics().unwrap();
        assert_eq!(diagnostics.schema_version, LATEST_SCHEMA_VERSION);
        assert_eq!(diagnostics.integrity_check, "ok");
        assert_eq!(diagnostics.foreign_key_violations, 0);
        assert_eq!(diagnostics.clinical_ruleset_version, LEGACY_RULESET);
        assert_eq!(
            diagnostics.label_template_version,
            PREPARATION_LABEL_TEMPLATE_VERSION
        );
        let serialized = serde_json::to_string(&diagnostics).unwrap();
        assert!(!serialized.contains(PASSWORD));
        assert!(!serialized.contains("password_hash"));
        assert!(!serialized.contains("patient_name"));
    }
}
