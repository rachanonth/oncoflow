use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{backup::Backup, Connection, OptionalExtension};
use thiserror::Error;

pub const DATABASE_FILENAME: &str = "oncoflow.db";
pub(crate) const LATEST_SCHEMA_VERSION: i64 = 19;
pub(crate) const MIN_SUPPORTED_SCHEMA_VERSION: i64 = 1;
pub(crate) const RECOVERY_DIRECTORY: &str = "backups";
const MIGRATION_BACKUP_RETENTION: usize = 7;
const INITIAL_MIGRATION: &str = include_str!("../../../migrations/001_initial.sql");
const IMPORT_COMPATIBILITY_MIGRATION: &str =
    include_str!("../../../migrations/002_import_compatibility.sql");
const ORDERS_WORKFLOW_MIGRATION: &str = include_str!("../../../migrations/003_orders_workflow.sql");
const PREPARATION_WORKSPACE_MIGRATION: &str =
    include_str!("../../../migrations/004_preparation_workspace.sql");
const LOCAL_IDENTITY_AUDIT_MIGRATION: &str =
    include_str!("../../../migrations/005_local_identity_audit.sql");
const INVENTORY_LEDGER_MIGRATION: &str =
    include_str!("../../../migrations/006_inventory_ledger.sql");
const PREPARATION_INVENTORY_CONSUMPTION_MIGRATION: &str =
    include_str!("../../../migrations/007_preparation_inventory_consumption.sql");
const PREPARATION_OUTPUT_MIGRATION: &str =
    include_str!("../../../migrations/008_preparation_output.sql");
const USER_MANAGEMENT_MIGRATION: &str = include_str!("../../../migrations/009_user_management.sql");
const PAGE_GUIDANCE_MIGRATION: &str = include_str!("../../../migrations/010_page_guidance.sql");
const ORDER_ITEM_DILUENT_VOLUME_MIGRATION: &str =
    include_str!("../../../migrations/011_order_item_diluent_volume.sql");
const ORDER_MEASUREMENT_SNAPSHOT_MIGRATION: &str =
    include_str!("../../../migrations/012_order_measurement_snapshot.sql");
const PREPARATION_ASSIGNMENT_MIGRATION: &str =
    include_str!("../../../migrations/013_preparation_assignment.sql");
const DAILY_PREPARATION_TASKS_MIGRATION: &str =
    include_str!("../../../migrations/014_daily_preparation_tasks.sql");
const APPLICATION_SETTINGS_MIGRATION: &str =
    include_str!("../../../migrations/015_application_settings.sql");
const ORDER_ATTENDANCE_STATUS_MIGRATION: &str =
    include_str!("../../../migrations/016_order_attendance_status.sql");
const PREPARATION_FINAL_CONTAINERS_MIGRATION: &str =
    include_str!("../../../migrations/017_preparation_final_containers.sql");
const PREPARATION_LABEL_CONTENT_MIGRATION: &str =
    include_str!("../../../migrations/018_preparation_label_content.sql");
const PREPARATION_WITHDRAWAL_VOLUME_MIGRATION: &str =
    include_str!("../../../migrations/019_preparation_withdrawal_volume.sql");

#[derive(Debug)]
pub struct Database {
    path: PathBuf,
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("could not create the database directory: {0}")]
    CreateDirectory(#[source] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("schema version {found} is newer than this application supports ({supported})")]
    UnsupportedSchemaVersion { found: i64, supported: i64 },
    #[error("schema version is missing after migrations completed")]
    MissingSchemaVersion,
    #[error("schema version is not a valid integer: {0}")]
    InvalidSchemaVersion(String),
    #[error("an existing database is not recognizable as OncoFlow")]
    UnrecognizedDatabase,
    #[error("database integrity validation failed")]
    IntegrityCheckFailed,
    #[error("database contains {0} foreign-key violation(s)")]
    ForeignKeyViolations(i64),
    #[error("a recovery backup could not be created before migration: {0}")]
    MigrationBackup(String),
    #[error("database migration failed")]
    MigrationFailed(#[source] Box<DatabaseError>),
    #[error("SQLite foreign-key enforcement could not be enabled")]
    ForeignKeysDisabled,
}

impl Database {
    pub fn initialize(path: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(DatabaseError::CreateDirectory)?;
        }

        let existing_nonempty = path
            .metadata()
            .map(|value| value.len() > 0)
            .unwrap_or(false);
        let database = Self { path };
        let connection = database.open()?;
        let current = read_schema_version(&connection)?;
        if existing_nonempty && current.is_none() {
            return Err(DatabaseError::UnrecognizedDatabase);
        }
        if let Some(version) = current {
            if version < LATEST_SCHEMA_VERSION {
                create_pre_migration_backup(&database, &connection, version)?;
            }
        }
        apply_migrations(&connection)
            .map_err(|error| DatabaseError::MigrationFailed(Box::new(error)))?;
        validate_connection(&connection)?;
        Ok(database)
    }

    pub(crate) fn at_path(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub fn schema_version(&self) -> Result<i64, DatabaseError> {
        let connection = self.open()?;
        read_schema_version(&connection)?.ok_or(DatabaseError::MissingSchemaVersion)
    }

    pub(crate) fn open(&self) -> Result<Connection, DatabaseError> {
        let connection = Connection::open(&self.path)?;
        configure_connection(&connection)?;
        Ok(connection)
    }
}

pub(crate) fn configure_connection(connection: &Connection) -> Result<(), DatabaseError> {
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    let foreign_keys_enabled: i64 =
        connection.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
    if foreign_keys_enabled != 1 {
        return Err(DatabaseError::ForeignKeysDisabled);
    }
    Ok(())
}

pub(crate) fn validate_connection(connection: &Connection) -> Result<(), DatabaseError> {
    let integrity = connection
        .prepare("PRAGMA integrity_check")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if integrity.len() != 1 || integrity[0] != "ok" {
        return Err(DatabaseError::IntegrityCheckFailed);
    }
    let foreign_key_violations: i64 =
        connection.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;
    if foreign_key_violations != 0 {
        return Err(DatabaseError::ForeignKeyViolations(foreign_key_violations));
    }
    Ok(())
}

pub(crate) fn apply_migrations(connection: &Connection) -> Result<(), DatabaseError> {
    let current = read_schema_version(connection)?.unwrap_or(0);
    if current > LATEST_SCHEMA_VERSION {
        return Err(DatabaseError::UnsupportedSchemaVersion {
            found: current,
            supported: LATEST_SCHEMA_VERSION,
        });
    }

    if current < 1 {
        connection.execute_batch(INITIAL_MIGRATION)?;
    }
    if current < 2 {
        connection.execute_batch(IMPORT_COMPATIBILITY_MIGRATION)?;
    }
    if current < 3 {
        connection.execute_batch(ORDERS_WORKFLOW_MIGRATION)?;
    }
    if current < 4 {
        connection.execute_batch(PREPARATION_WORKSPACE_MIGRATION)?;
    }
    if current < 5 {
        connection.execute_batch(LOCAL_IDENTITY_AUDIT_MIGRATION)?;
    }
    if current < 6 {
        connection.execute_batch(INVENTORY_LEDGER_MIGRATION)?;
    }
    if current < 7 {
        connection.execute_batch(PREPARATION_INVENTORY_CONSUMPTION_MIGRATION)?;
    }
    if current < 8 {
        connection.execute_batch(PREPARATION_OUTPUT_MIGRATION)?;
    }
    if current < 9 {
        connection.execute_batch(USER_MANAGEMENT_MIGRATION)?;
    }
    if current < 10 {
        connection.execute_batch(PAGE_GUIDANCE_MIGRATION)?;
    }
    if current < 11 {
        connection.execute_batch(ORDER_ITEM_DILUENT_VOLUME_MIGRATION)?;
    }
    if current < 12 {
        connection.execute_batch(ORDER_MEASUREMENT_SNAPSHOT_MIGRATION)?;
    }
    if current < 13 {
        connection.execute_batch(PREPARATION_ASSIGNMENT_MIGRATION)?;
    }
    if current < 14 {
        connection.execute_batch(DAILY_PREPARATION_TASKS_MIGRATION)?;
    }
    if current < 15 {
        connection.execute_batch(APPLICATION_SETTINGS_MIGRATION)?;
    }
    if current < 16 {
        connection.execute_batch(ORDER_ATTENDANCE_STATUS_MIGRATION)?;
    }
    if current < 17 {
        connection.execute_batch(PREPARATION_FINAL_CONTAINERS_MIGRATION)?;
    }
    if current < 18 {
        connection.execute_batch(PREPARATION_LABEL_CONTENT_MIGRATION)?;
    }
    if current < 19 {
        connection.execute_batch(PREPARATION_WITHDRAWAL_VOLUME_MIGRATION)?;
    }

    match read_schema_version(connection)? {
        Some(found) if found == LATEST_SCHEMA_VERSION => Ok(()),
        Some(found) => Err(DatabaseError::UnsupportedSchemaVersion {
            found,
            supported: LATEST_SCHEMA_VERSION,
        }),
        None => Err(DatabaseError::MissingSchemaVersion),
    }
}

pub(crate) fn read_schema_version(connection: &Connection) -> Result<Option<i64>, DatabaseError> {
    let app_meta_exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'app_meta')",
        [],
        |row| row.get(0),
    )?;

    if !app_meta_exists {
        return Ok(None);
    }

    let value = connection
        .query_row(
            "SELECT value FROM app_meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    value
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|_| DatabaseError::InvalidSchemaVersion(value))
        })
        .transpose()
}

fn create_pre_migration_backup(
    database: &Database,
    source: &Connection,
    version: i64,
) -> Result<(), DatabaseError> {
    let parent = database.path.parent().ok_or_else(|| {
        DatabaseError::MigrationBackup("the database directory is unavailable".into())
    })?;
    let directory = parent.join(RECOVERY_DIRECTORY).join("migration");
    fs::create_dir_all(&directory).map_err(|_| {
        DatabaseError::MigrationBackup("the recovery directory is unavailable".into())
    })?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| DatabaseError::MigrationBackup("the system clock is unavailable".into()))?
        .as_millis();
    let destination = directory.join(format!("pre_migration_schema_{version}_{timestamp}.db"));
    let mut output = Connection::open(&destination).map_err(|_| {
        DatabaseError::MigrationBackup("the recovery file could not be opened".into())
    })?;
    {
        let backup = Backup::new(source, &mut output).map_err(|_| {
            DatabaseError::MigrationBackup("the SQLite snapshot could not start".into())
        })?;
        backup
            .run_to_completion(128, Duration::from_millis(10), None)
            .map_err(|_| {
                DatabaseError::MigrationBackup("the SQLite snapshot did not complete".into())
            })?;
    }
    configure_connection(&output).map_err(|_| {
        DatabaseError::MigrationBackup("the recovery file could not be validated".into())
    })?;
    validate_connection(&output).map_err(|_| {
        DatabaseError::MigrationBackup("the recovery file failed validation".into())
    })?;
    drop(output);
    retain_migration_backups(&directory);
    Ok(())
}

fn retain_migration_backups(directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut backups = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            (name.starts_with("pre_migration_schema_") && name.ends_with(".db"))
                .then_some(entry.path())
        })
        .collect::<Vec<_>>();
    backups.sort();
    let remove_count = backups.len().saturating_sub(MIGRATION_BACKUP_RETENTION);
    for path in backups.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_database_and_applies_initial_migration() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILENAME);

        let database = Database::initialize(&path).expect("database should initialize");

        assert!(path.is_file());
        assert_eq!(database.schema_version().unwrap(), LATEST_SCHEMA_VERSION);

        let connection = database.open().unwrap();
        let patients_table_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'patients')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(patients_table_exists);
    }

    #[test]
    fn initialization_is_idempotent() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILENAME);

        Database::initialize(&path).expect("first initialization should succeed");
        let database = Database::initialize(&path).expect("second initialization should succeed");

        assert_eq!(database.schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    }

    #[test]
    fn upgrades_schema_version_one_to_latest() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        assert_eq!(read_schema_version(&connection).unwrap(), Some(1));

        apply_migrations(&connection).unwrap();

        assert_eq!(
            read_schema_version(&connection).unwrap(),
            Some(LATEST_SCHEMA_VERSION)
        );
        let compatibility_column_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('regimen_items') WHERE name='legacy_dose_text')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(compatibility_column_exists);
        let order_provenance_column_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('orders') WHERE name='oncoflow_created')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(order_provenance_column_exists);
        let preparation_table_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='preparation_tasks')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(preparation_table_exists);
    }

    #[test]
    fn order_migration_preserves_existing_historical_rows() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        connection
            .execute("INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-HN')", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO orders(id,legacy_orderid,patient_id) VALUES(7,'7',1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO drugs(id,legacy_dcode,drug_name) VALUES(1,'D1','Synthetic')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO order_items(id,order_id,drug_id,dose,ordering_no) VALUES(8,7,1,12.5,1)",
                [],
            )
            .unwrap();

        apply_migrations(&connection).unwrap();

        let row: (i64, f64, i64, Option<String>) = connection
            .query_row(
                "SELECT o.id, i.dose, o.oncoflow_created, i.legacy_dose_text
                 FROM orders o JOIN order_items i ON i.order_id=o.id",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(row, (7, 12.5, 0, None));
    }

    #[test]
    fn every_connection_enforces_foreign_keys() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let database = Database::initialize(directory.path().join(DATABASE_FILENAME))
            .expect("database should initialize");
        let connection = database.open().unwrap();

        let foreign_keys_enabled: i64 = connection
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys_enabled, 1);

        let result = connection.execute(
            "INSERT INTO patients (legacy_hn, diagnosis_id) VALUES ('TEST-HN', 999999)",
            [],
        );
        assert!(result.is_err(), "invalid foreign key should be rejected");
    }

    #[test]
    fn preparation_migration_preserves_existing_order_counts() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        connection
            .execute_batch(IMPORT_COMPATIBILITY_MIGRATION)
            .unwrap();
        connection.execute_batch(ORDERS_WORKFLOW_MIGRATION).unwrap();
        connection
            .execute("INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-HN')", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO orders(id,legacy_orderid,patient_id,oncoflow_created) VALUES(1,'OF-1',1,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO drugs(id,legacy_dcode,drug_name,marker) VALUES(1,'SYN-D','Synthetic',1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO order_items(id,order_id,drug_id) VALUES(1,1,1)",
                [],
            )
            .unwrap();

        apply_migrations(&connection).unwrap();

        let counts: (i64, i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM orders),
                        (SELECT COUNT(*) FROM order_items),
                        (SELECT COUNT(*) FROM preparation_tasks)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(counts, (1, 1, 0));
    }

    #[test]
    fn identity_migration_preserves_schema_four_tasks_without_fabricating_actors() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        connection
            .execute_batch(IMPORT_COMPATIBILITY_MIGRATION)
            .unwrap();
        connection.execute_batch(ORDERS_WORKFLOW_MIGRATION).unwrap();
        connection
            .execute_batch(PREPARATION_WORKSPACE_MIGRATION)
            .unwrap();
        connection
            .execute(
                "INSERT INTO users(id,legacy_user,username,display_name,password_hash,role,active)
             VALUES(1,'LEG','legacy.user','Legacy metadata','LEGACY_CREDENTIAL_DISABLED','user',0)",
                [],
            )
            .unwrap();
        connection
            .execute("INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-HN')", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO orders(id,legacy_orderid,patient_id,oncoflow_created) VALUES(1,'OF-1',1,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO drugs(id,legacy_dcode,drug_name,marker) VALUES(1,'SYN-D','Synthetic',1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO order_items(id,order_id,drug_id) VALUES(1,1,1)",
                [],
            )
            .unwrap();
        connection.execute(
            "INSERT INTO preparation_tasks(id,source_order_id,source_order_item_id,drug_id,state,prepared_at) VALUES(1,1,1,1,'prepared',CURRENT_TIMESTAMP)",
            [],
        ).unwrap();

        apply_migrations(&connection).unwrap();

        let row: (Option<i64>, Option<i64>, String) = connection
            .query_row(
                "SELECT prepared_by_user_id,verified_by_user_id,state FROM preparation_tasks WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(row, (None, None, "prepared".into()));
        let legacy_user: (String, String) = connection
            .query_row(
                "SELECT credential_kind,updated_at FROM users WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(legacy_user.0, "legacy_disabled");
        assert!(!legacy_user.1.is_empty());
        assert_eq!(
            read_schema_version(&connection).unwrap(),
            Some(LATEST_SCHEMA_VERSION)
        );
    }

    #[test]
    fn inventory_migration_preserves_schema_five_data_and_seeds_exact_opening_balances() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        connection
            .execute_batch(IMPORT_COMPATIBILITY_MIGRATION)
            .unwrap();
        connection.execute_batch(ORDERS_WORKFLOW_MIGRATION).unwrap();
        connection
            .execute_batch(PREPARATION_WORKSPACE_MIGRATION)
            .unwrap();
        connection
            .execute_batch(LOCAL_IDENTITY_AUDIT_MIGRATION)
            .unwrap();
        connection
            .execute(
                "INSERT INTO users(
                    id,username,display_name,password_hash,role,active,credential_kind,updated_at
                 ) VALUES(1,'synthetic','Synthetic user','$argon2id$synthetic','admin',1,'argon2id',CURRENT_TIMESTAMP)",
                [],
            )
            .unwrap();
        connection
            .execute("INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-HN')", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO orders(id,legacy_orderid,patient_id,oncoflow_created)
                 VALUES(1,'OF-SYN',1,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO drugs(id,legacy_dcode,drug_name,inventory_qty,inventory_enabled)
                 VALUES(1,'SYN-POS','Synthetic positive',12.5,1),
                       (2,'SYN-ZERO','Synthetic zero',0,1),
                       (3,'SYN-NULL','Synthetic null',NULL,0),
                       (4,'SYN-NEG','Synthetic negative',-2.25,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO order_items(id,order_id,drug_id) VALUES(1,1,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO inventory_events(id,legacy_incode,drug_id,quantity,inventory_ok,send_order)
                 VALUES(1,'1',1,5,1,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO preparation_tasks(
                    id,source_order_id,source_order_item_id,drug_id,state
                 ) VALUES(1,1,1,1,'pending')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO audit_events(id,user_id,event_type,entity_type,entity_id)
                 VALUES(1,1,'synthetic_existing','user','1')",
                [],
            )
            .unwrap();

        apply_migrations(&connection).unwrap();

        let openings = connection
            .prepare(
                "SELECT d.legacy_dcode,m.quantity_delta,m.actor_user_id,m.occurred_at
                 FROM inventory_movements m
                 JOIN drugs d ON d.id=m.drug_id
                 ORDER BY d.id",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            openings,
            vec![
                ("SYN-POS".into(), 12.5, None, None),
                ("SYN-ZERO".into(), 0.0, None, None),
                ("SYN-NEG".into(), -2.25, None, None),
            ]
        );
        let preserved: (i64, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM orders),
                    (SELECT COUNT(*) FROM order_items),
                    (SELECT COUNT(*) FROM inventory_events),
                    (SELECT COUNT(*) FROM preparation_tasks),
                    (SELECT COUNT(*) FROM audit_events)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(preserved, (1, 1, 1, 1, 1));
        assert_eq!(
            read_schema_version(&connection).unwrap(),
            Some(LATEST_SCHEMA_VERSION)
        );

        apply_migrations(&connection).unwrap();
        let opening_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM inventory_movements WHERE movement_type='opening_balance'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(opening_count, 3);
    }

    #[test]
    fn preparation_inventory_migration_preserves_schema_six_data_without_backfill() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        connection
            .execute_batch(IMPORT_COMPATIBILITY_MIGRATION)
            .unwrap();
        connection.execute_batch(ORDERS_WORKFLOW_MIGRATION).unwrap();
        connection
            .execute_batch(PREPARATION_WORKSPACE_MIGRATION)
            .unwrap();
        connection
            .execute_batch(LOCAL_IDENTITY_AUDIT_MIGRATION)
            .unwrap();
        connection
            .execute(
                "INSERT INTO users(
                    id,username,display_name,password_hash,role,active,credential_kind,updated_at
                 ) VALUES(1,'synthetic','Synthetic user','$argon2id$synthetic','admin',1,'argon2id',CURRENT_TIMESTAMP)",
                [],
            )
            .unwrap();
        connection
            .execute("INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-HN')", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO orders(id,legacy_orderid,patient_id,oncoflow_created)
                 VALUES(1,'OF-SYN',1,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO drugs(
                    id,legacy_dcode,drug_name,inventory_qty,inventory_enabled
                 ) VALUES(1,'SYN-D','Synthetic preparation drug',4.5,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO order_items(id,order_id,drug_id) VALUES(1,1,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO preparation_tasks(
                    id,source_order_id,source_order_item_id,drug_id,state,
                    prepared_at,verified_at,prepared_by_user_id,verified_by_user_id
                 ) VALUES(1,1,1,1,'verified',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,1,1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO audit_events(id,user_id,event_type,entity_type,entity_id)
                 VALUES(1,1,'synthetic_existing','preparation_task','1')",
                [],
            )
            .unwrap();
        connection
            .execute_batch(INVENTORY_LEDGER_MIGRATION)
            .unwrap();

        let before: (i64, f64, i64, i64, i64) = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM inventory_movements),
                    (SELECT SUM(quantity_delta) FROM inventory_movements),
                    (SELECT COUNT(*) FROM preparation_tasks),
                    (SELECT COUNT(*) FROM orders),
                    (SELECT COUNT(*) FROM audit_events)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();

        connection
            .execute_batch(PREPARATION_INVENTORY_CONSUMPTION_MIGRATION)
            .unwrap();

        let after: (i64, f64, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM inventory_movements),
                    (SELECT SUM(quantity_delta) FROM inventory_movements),
                    (SELECT COUNT(*) FROM preparation_tasks),
                    (SELECT COUNT(*) FROM orders),
                    (SELECT COUNT(*) FROM audit_events),
                    (SELECT COUNT(*) FROM preparation_inventory_postings)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(before, (after.0, after.1, after.2, after.3, after.4));
        assert_eq!(after.5, 0, "existing verified tasks must not be backfilled");
        let preserved_movement: (i64, f64, Option<i64>) = connection
            .query_row(
                "SELECT id,quantity_delta,preparation_task_id FROM inventory_movements",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(preserved_movement, (1, 4.5, None));
        assert_eq!(read_schema_version(&connection).unwrap(), Some(7));
        assert_eq!(
            connection
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        let foreign_key_violations: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(foreign_key_violations, 0);

        connection
            .execute(
                "INSERT INTO inventory_movements(
                    drug_id,movement_type,quantity_delta,occurred_at,actor_user_id,
                    reference_type,reference_id,preparation_task_id
                 ) VALUES(1,'preparation_issue',-1,CURRENT_TIMESTAMP,1,
                          'preparation_task','1',1)",
                [],
            )
            .unwrap();
        assert!(connection
            .execute(
                "INSERT INTO inventory_movements(
                    drug_id,movement_type,quantity_delta,occurred_at,actor_user_id,
                    reference_type,reference_id,preparation_task_id
                 ) VALUES(1,'preparation_issue',-1,CURRENT_TIMESTAMP,1,
                          'preparation_task','1-duplicate',1)",
                [],
            )
            .is_err());
        assert!(connection
            .execute(
                "INSERT INTO preparation_inventory_postings(
                    preparation_task_id,status,inventory_movement_id,containers_required,
                    balance_before,balance_after,resulting_stock_state,calculation_status,
                    calculation_ruleset_version,calculation_rule_id,workflow_rule_id,
                    reason_code,actor_user_id
                 ) VALUES(1,'posted',2,2,4.5,3.5,'normal','calculated',
                          'legacy-cytotoxic-v8','synthetic-calculation','synthetic-workflow',
                          'synthetic-invalid-quantity',1)",
                [],
            )
            .is_err());
        connection
            .execute(
                "INSERT INTO preparation_inventory_postings(
                    preparation_task_id,status,inventory_movement_id,containers_required,
                    balance_before,balance_after,resulting_stock_state,calculation_status,
                    calculation_ruleset_version,calculation_rule_id,workflow_rule_id,
                    reason_code,actor_user_id
                 ) VALUES(1,'posted',2,1,4.5,3.5,'normal','calculated',
                          'legacy-cytotoxic-v8','synthetic-calculation','synthetic-workflow',
                          'synthetic-valid-link',1)",
                [],
            )
            .unwrap();
        assert!(connection
            .execute(
                "UPDATE inventory_movements SET quantity_delta=-2 WHERE movement_type='preparation_issue'",
                [],
            )
            .is_err());
    }

    #[test]
    fn preparation_output_migration_preserves_schema_seven_without_backfill() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        connection
            .execute_batch(IMPORT_COMPATIBILITY_MIGRATION)
            .unwrap();
        connection.execute_batch(ORDERS_WORKFLOW_MIGRATION).unwrap();
        connection
            .execute_batch(PREPARATION_WORKSPACE_MIGRATION)
            .unwrap();
        connection
            .execute_batch(LOCAL_IDENTITY_AUDIT_MIGRATION)
            .unwrap();
        connection
            .execute_batch(INVENTORY_LEDGER_MIGRATION)
            .unwrap();
        connection
            .execute_batch(PREPARATION_INVENTORY_CONSUMPTION_MIGRATION)
            .unwrap();
        connection
            .execute_batch(
                "INSERT INTO users(
                   id,username,display_name,password_hash,role,active,
                   credential_kind,updated_at
                 ) VALUES(1,'synthetic','Synthetic pharmacist','$argon2id$synthetic',
                          'admin',1,'argon2id',CURRENT_TIMESTAMP);
                 INSERT INTO patients(id,legacy_hn,first_name)
                 VALUES(1,'SYN-HN','Synthetic');
                 INSERT INTO drugs(id,legacy_dcode,drug_name,marker)
                 VALUES(1,'SYN-D','Synthetic drug',1);
                 INSERT INTO orders(
                   id,legacy_orderid,patient_id,oncoflow_created,order_time
                 ) VALUES(1,'OF-SYN',1,1,'2026-08-23T09:00:00');
                 INSERT INTO order_items(id,order_id,drug_id,dose)
                 VALUES(1,1,1,100);
                 INSERT INTO preparation_tasks(
                   id,source_order_id,source_order_item_id,drug_id,state,
                   snapshot_ordered_dose_text,prepared_at,verified_at,
                   prepared_by_user_id,verified_by_user_id
                 ) VALUES(1,1,1,1,'verified','100',CURRENT_TIMESTAMP,
                          CURRENT_TIMESTAMP,1,1);
                 INSERT INTO audit_events(
                   id,user_id,event_type,entity_type,entity_id
                 ) VALUES(1,1,'synthetic_existing','preparation_task','1');",
            )
            .unwrap();

        let before: (i64, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM orders),
                        (SELECT COUNT(*) FROM order_items),
                        (SELECT COUNT(*) FROM preparation_tasks),
                        (SELECT COUNT(*) FROM inventory_movements),
                        (SELECT COUNT(*) FROM audit_events)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();

        connection
            .execute_batch(PREPARATION_OUTPUT_MIGRATION)
            .unwrap();

        let after: (i64, i64, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM orders),
                        (SELECT COUNT(*) FROM order_items),
                        (SELECT COUNT(*) FROM preparation_tasks),
                        (SELECT COUNT(*) FROM inventory_movements),
                        (SELECT COUNT(*) FROM audit_events),
                        (SELECT COUNT(*) FROM preparation_output_snapshots)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(before, (after.0, after.1, after.2, after.3, after.4));
        assert_eq!(after.5, 0, "existing verified tasks must not be backfilled");
        assert_eq!(read_schema_version(&connection).unwrap(), Some(8));
        assert_eq!(
            connection
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        let foreign_key_violations: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(foreign_key_violations, 0);
    }

    #[test]
    fn missing_database_initializes_but_existing_unrecognized_database_is_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.db");
        Database::initialize(&missing).unwrap();
        assert!(missing.is_file());

        let unrelated = directory.path().join("unrelated.db");
        Connection::open(&unrelated)
            .unwrap()
            .execute("CREATE TABLE unrelated(value TEXT)", [])
            .unwrap();
        assert!(matches!(
            Database::initialize(&unrelated),
            Err(DatabaseError::UnrecognizedDatabase)
        ));
        let connection = Connection::open(&unrelated).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM unrelated", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(!connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='app_meta')",
                [],
                |row| row.get::<_, bool>(0)
            )
            .unwrap());
    }

    #[test]
    fn user_management_migration_preserves_modern_and_legacy_identity_provenance() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            INITIAL_MIGRATION,
            IMPORT_COMPATIBILITY_MIGRATION,
            ORDERS_WORKFLOW_MIGRATION,
            PREPARATION_WORKSPACE_MIGRATION,
            LOCAL_IDENTITY_AUDIT_MIGRATION,
            INVENTORY_LEDGER_MIGRATION,
            PREPARATION_INVENTORY_CONSUMPTION_MIGRATION,
            PREPARATION_OUTPUT_MIGRATION,
        ] {
            connection.execute_batch(migration).unwrap();
        }
        connection.execute_batch(
            "INSERT INTO users(id,username,display_name,password_hash,role,active,credential_kind)
             VALUES(1,'modern.admin','Modern Admin','synthetic-hash','admin',1,'argon2id');
             INSERT INTO users(id,username,display_name,password_hash,role,active,credential_kind)
             VALUES(2,'legacy.disabled','Legacy Disabled','LEGACY_CREDENTIAL_DISABLED','user',0,'legacy_disabled');",
        ).unwrap();

        connection.execute_batch(USER_MANAGEMENT_MIGRATION).unwrap();

        let user_types: Vec<(i64, String)> = connection
            .prepare("SELECT id,user_type FROM users ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            user_types,
            vec![(1, "pharmacist".into()), (2, "non_pharmacist".into())]
        );
        assert_eq!(read_schema_version(&connection).unwrap(), Some(9));
        validate_connection(&connection).unwrap();
    }

    #[test]
    fn page_guidance_migration_preserves_schema_nine_data() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            INITIAL_MIGRATION,
            IMPORT_COMPATIBILITY_MIGRATION,
            ORDERS_WORKFLOW_MIGRATION,
            PREPARATION_WORKSPACE_MIGRATION,
            LOCAL_IDENTITY_AUDIT_MIGRATION,
            INVENTORY_LEDGER_MIGRATION,
            PREPARATION_INVENTORY_CONSUMPTION_MIGRATION,
            PREPARATION_OUTPUT_MIGRATION,
            USER_MANAGEMENT_MIGRATION,
        ] {
            connection.execute_batch(migration).unwrap();
        }
        connection
            .execute(
                "INSERT INTO patients(legacy_hn,first_name,last_name) VALUES('SYN-GUIDE','ก่อน','ย้าย')",
                [],
            )
            .unwrap();

        connection.execute_batch(PAGE_GUIDANCE_MIGRATION).unwrap();

        assert_eq!(read_schema_version(&connection).unwrap(), Some(10));
        assert_eq!(
            connection
                .query_row(
                    "SELECT first_name || last_name FROM patients WHERE legacy_hn='SYN-GUIDE'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "ก่อนย้าย"
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM page_guidance", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        validate_connection(&connection).unwrap();
    }

    #[test]
    fn order_item_diluent_volume_migration_preserves_schema_ten_data() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            INITIAL_MIGRATION,
            IMPORT_COMPATIBILITY_MIGRATION,
            ORDERS_WORKFLOW_MIGRATION,
            PREPARATION_WORKSPACE_MIGRATION,
            LOCAL_IDENTITY_AUDIT_MIGRATION,
            INVENTORY_LEDGER_MIGRATION,
            PREPARATION_INVENTORY_CONSUMPTION_MIGRATION,
            PREPARATION_OUTPUT_MIGRATION,
            USER_MANAGEMENT_MIGRATION,
            PAGE_GUIDANCE_MIGRATION,
        ] {
            connection.execute_batch(migration).unwrap();
        }
        connection
            .execute_batch(
                "INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-VOLUME');
             INSERT INTO drugs(id,legacy_dcode,drug_name) VALUES(1,'SYN-D','Synthetic');
             INSERT INTO orders(id,legacy_orderid,patient_id) VALUES(1,'SYN-O',1);
             INSERT INTO order_items(id,order_id,drug_id) VALUES(1,1,1);",
            )
            .unwrap();

        connection
            .execute_batch(ORDER_ITEM_DILUENT_VOLUME_MIGRATION)
            .unwrap();

        assert_eq!(read_schema_version(&connection).unwrap(), Some(11));
        assert_eq!(
            connection
                .query_row(
                    "SELECT diluent_volume_ml FROM order_items WHERE id=1",
                    [],
                    |row| row.get::<_, Option<f64>>(0),
                )
                .unwrap(),
            None,
        );
        assert!(connection
            .execute("UPDATE order_items SET diluent_volume_ml=-1 WHERE id=1", [],)
            .is_err());
        validate_connection(&connection).unwrap();
    }

    #[test]
    fn order_measurement_migration_creates_a_stable_snapshot() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            INITIAL_MIGRATION,
            IMPORT_COMPATIBILITY_MIGRATION,
            ORDERS_WORKFLOW_MIGRATION,
            PREPARATION_WORKSPACE_MIGRATION,
            LOCAL_IDENTITY_AUDIT_MIGRATION,
            INVENTORY_LEDGER_MIGRATION,
            PREPARATION_INVENTORY_CONSUMPTION_MIGRATION,
            PREPARATION_OUTPUT_MIGRATION,
            USER_MANAGEMENT_MIGRATION,
            PAGE_GUIDANCE_MIGRATION,
            ORDER_ITEM_DILUENT_VOLUME_MIGRATION,
        ] {
            connection.execute_batch(migration).unwrap();
        }
        connection
            .execute_batch(
                "INSERT INTO patients(id,legacy_hn,weight_kg,height_cm) VALUES(1,'SYN-MEASURE',70,175);
                 INSERT INTO orders(id,legacy_orderid,patient_id) VALUES(1,'SYN-ORDER',1);",
            )
            .unwrap();

        connection
            .execute_batch(ORDER_MEASUREMENT_SNAPSHOT_MIGRATION)
            .unwrap();

        assert_eq!(read_schema_version(&connection).unwrap(), Some(12));
        assert_eq!(
            connection
                .query_row(
                    "SELECT weight_kg,height_cm FROM orders WHERE id=1",
                    [],
                    |row| Ok((row.get::<_, Option<f64>>(0)?, row.get::<_, Option<f64>>(1)?)),
                )
                .unwrap(),
            (Some(70.0), Some(175.0)),
        );
        connection
            .execute(
                "UPDATE patients SET weight_kg=80,height_cm=180 WHERE id=1",
                [],
            )
            .unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT weight_kg,height_cm FROM orders WHERE id=1",
                    [],
                    |row| { Ok((row.get::<_, Option<f64>>(0)?, row.get::<_, Option<f64>>(1)?)) }
                )
                .unwrap(),
            (Some(70.0), Some(175.0)),
        );
        validate_connection(&connection).unwrap();
    }

    #[test]
    fn preparation_assignment_migration_preserves_orders_without_fabricating_a_pharmacist() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            INITIAL_MIGRATION,
            IMPORT_COMPATIBILITY_MIGRATION,
            ORDERS_WORKFLOW_MIGRATION,
            PREPARATION_WORKSPACE_MIGRATION,
            LOCAL_IDENTITY_AUDIT_MIGRATION,
            INVENTORY_LEDGER_MIGRATION,
            PREPARATION_INVENTORY_CONSUMPTION_MIGRATION,
            PREPARATION_OUTPUT_MIGRATION,
            USER_MANAGEMENT_MIGRATION,
            PAGE_GUIDANCE_MIGRATION,
            ORDER_ITEM_DILUENT_VOLUME_MIGRATION,
            ORDER_MEASUREMENT_SNAPSHOT_MIGRATION,
        ] {
            connection.execute_batch(migration).unwrap();
        }
        connection
            .execute_batch(
                "INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-ASSIGN');
                 INSERT INTO users(id,username,display_name,password_hash,role,active,
                    credential_kind,updated_at,user_type)
                 VALUES(1,'synthetic.preparer','เภสัชกรสังเคราะห์','$argon2id$synthetic',
                    'user',1,'argon2id',CURRENT_TIMESTAMP,'pharmacist');
                 INSERT INTO orders(id,legacy_orderid,patient_id,oncoflow_created)
                 VALUES(1,'SYN-ORDER',1,1);",
            )
            .unwrap();

        connection
            .execute_batch(PREPARATION_ASSIGNMENT_MIGRATION)
            .unwrap();

        assert_eq!(read_schema_version(&connection).unwrap(), Some(13));
        assert_eq!(
            connection
                .query_row(
                    "SELECT assigned_preparer_user_id FROM orders WHERE id=1",
                    [],
                    |row| row.get::<_, Option<i64>>(0),
                )
                .unwrap(),
            None
        );
        connection
            .execute(
                "UPDATE orders SET assigned_preparer_user_id=1 WHERE id=1",
                [],
            )
            .unwrap();
        validate_connection(&connection).unwrap();
    }

    #[test]
    fn daily_preparation_migration_preserves_existing_task_and_allows_one_per_date() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            INITIAL_MIGRATION,
            IMPORT_COMPATIBILITY_MIGRATION,
            ORDERS_WORKFLOW_MIGRATION,
            PREPARATION_WORKSPACE_MIGRATION,
            LOCAL_IDENTITY_AUDIT_MIGRATION,
            INVENTORY_LEDGER_MIGRATION,
            PREPARATION_INVENTORY_CONSUMPTION_MIGRATION,
            PREPARATION_OUTPUT_MIGRATION,
            USER_MANAGEMENT_MIGRATION,
            PAGE_GUIDANCE_MIGRATION,
            ORDER_ITEM_DILUENT_VOLUME_MIGRATION,
            ORDER_MEASUREMENT_SNAPSHOT_MIGRATION,
            PREPARATION_ASSIGNMENT_MIGRATION,
        ] {
            connection.execute_batch(migration).unwrap();
        }
        connection
            .execute_batch(
                "INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-DAILY');
                 INSERT INTO drugs(id,legacy_dcode,drug_name,marker)
                 VALUES(1,'SYN-DAILY','Synthetic daily agent',1);
                 INSERT INTO orders(id,legacy_orderid,patient_id,order_time,oncoflow_created)
                 VALUES(1,'SYN-ORDER',1,'2026-08-23T09:00',1);
                 INSERT INTO order_items(id,order_id,drug_id,start_date,stop_date)
                 VALUES(1,1,1,'2026-08-23','2026-08-27');
                 INSERT INTO users(
                    id,username,display_name,password_hash,role,user_type,active,
                    credential_kind,updated_at
                 ) VALUES(
                    1,'daily.pharmacist','Daily pharmacist','$argon2id$synthetic',
                    'user','pharmacist',1,'argon2id',CURRENT_TIMESTAMP
                 );
                 INSERT INTO preparation_tasks(
                    id,source_order_id,source_order_item_id,drug_id,state
                 ) VALUES(1,1,1,1,'pending');
                 INSERT INTO safety_acknowledgements(
                    order_id,preparation_task_id,order_item_id,finding_id,
                    finding_fingerprint,rule_id,ruleset_version,user_id
                 ) VALUES(
                    1,1,1,'daily-fixture',
                    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
                    'daily-rule','daily-ruleset',1
                 );",
            )
            .unwrap();

        connection
            .execute_batch(DAILY_PREPARATION_TASKS_MIGRATION)
            .unwrap();

        assert_eq!(read_schema_version(&connection).unwrap(), Some(14));
        assert_eq!(
            connection
                .query_row(
                    "SELECT preparation_date,snapshot_start_date,snapshot_stop_date
                     FROM preparation_tasks WHERE id=1",
                    [],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    },
                )
                .unwrap(),
            (
                "2026-08-23".into(),
                Some("2026-08-23".into()),
                Some("2026-08-27".into())
            )
        );
        connection
            .execute(
                "INSERT INTO preparation_tasks(
                    source_order_id,source_order_item_id,preparation_date,drug_id,state
                 ) VALUES(1,1,'2026-08-24',1,'pending')",
                [],
            )
            .unwrap();
        assert!(connection
            .execute(
                "INSERT INTO preparation_tasks(
                    source_order_id,source_order_item_id,preparation_date,drug_id,state
                 ) VALUES(1,1,'2026-08-24',1,'pending')",
                [],
            )
            .is_err());
        assert_eq!(
            connection
                .query_row(
                    "SELECT preparation_task_id FROM safety_acknowledgements",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
        validate_connection(&connection).unwrap();
    }

    #[test]
    fn order_attendance_migration_preserves_dates_and_does_not_fabricate_events() {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            INITIAL_MIGRATION,
            IMPORT_COMPATIBILITY_MIGRATION,
            ORDERS_WORKFLOW_MIGRATION,
            PREPARATION_WORKSPACE_MIGRATION,
            LOCAL_IDENTITY_AUDIT_MIGRATION,
            INVENTORY_LEDGER_MIGRATION,
            PREPARATION_INVENTORY_CONSUMPTION_MIGRATION,
            PREPARATION_OUTPUT_MIGRATION,
            USER_MANAGEMENT_MIGRATION,
            PAGE_GUIDANCE_MIGRATION,
            ORDER_ITEM_DILUENT_VOLUME_MIGRATION,
            ORDER_MEASUREMENT_SNAPSHOT_MIGRATION,
            PREPARATION_ASSIGNMENT_MIGRATION,
            DAILY_PREPARATION_TASKS_MIGRATION,
            APPLICATION_SETTINGS_MIGRATION,
        ] {
            connection.execute_batch(migration).unwrap();
        }
        connection
            .execute_batch(
                "INSERT INTO patients(id,legacy_hn) VALUES(1,'SYN-STATUS');
                 INSERT INTO drugs(id,legacy_dcode,drug_name) VALUES(1,'SYN-D','Synthetic');
                 INSERT INTO orders(id,legacy_orderid,patient_id,order_time,oncoflow_created)
                 VALUES(1,'SYN-ACTIVE',1,'2026-08-25T09:00',1),
                       (2,'SYN-LEGACY',1,'2020-01-01T09:00',0);
                 INSERT INTO order_items(id,order_id,drug_id,start_date,stop_date)
                 VALUES(1,1,1,'2026-08-25','2026-08-27');",
            )
            .unwrap();

        connection
            .execute_batch(ORDER_ATTENDANCE_STATUS_MIGRATION)
            .unwrap();

        assert_eq!(read_schema_version(&connection).unwrap(), Some(16));
        let rows = connection
            .prepare("SELECT legacy_orderid,workflow_status,order_time FROM orders ORDER BY id")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                (
                    "SYN-ACTIVE".into(),
                    "active".into(),
                    "2026-08-25T09:00".into()
                ),
                (
                    "SYN-LEGACY".into(),
                    "legacy".into(),
                    "2020-01-01T09:00".into()
                ),
            ]
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM order_status_events", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT start_date,stop_date FROM order_items WHERE id=1",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .unwrap(),
            ("2026-08-25".into(), "2026-08-27".into())
        );
        validate_connection(&connection).unwrap();
    }

    #[test]
    fn corrupt_database_is_rejected_without_replacing_its_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("corrupt.db");
        let original = b"synthetic corrupt database bytes";
        fs::write(&path, original).unwrap();
        assert!(Database::initialize(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn migration_failure_leaves_source_version_and_validated_recovery_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("migration-failure.db");
        let database = Database::initialize(&path).unwrap();
        database
            .open()
            .unwrap()
            .execute_batch(
                "DROP TRIGGER preparation_output_snapshots_reject_update;
                 DROP TRIGGER preparation_output_snapshots_reject_delete;
                 DROP TABLE preparation_output_snapshots;
                 CREATE TABLE preparation_output_snapshots(dummy INTEGER);
                 UPDATE app_meta SET value='7' WHERE key='schema_version';",
            )
            .unwrap();

        assert!(Database::initialize(&path).is_err());
        let connection = Connection::open(&path).unwrap();
        assert_eq!(read_schema_version(&connection).unwrap(), Some(7));
        assert_eq!(
            connection
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );

        let migration_directory = directory.path().join("backups").join("migration");
        let backups = fs::read_dir(&migration_directory)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|value| value == "db"))
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        let backup = Connection::open(&backups[0]).unwrap();
        assert_eq!(read_schema_version(&backup).unwrap(), Some(7));
        validate_connection(&backup).unwrap();
    }
}
