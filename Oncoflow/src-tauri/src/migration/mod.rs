#[cfg(feature = "migration-cli")]
mod access;
mod mapping;
mod report;
mod validation;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::db;
use mapping::{
    create_drug_detail_placeholders, create_regimen_group_placeholders,
    create_regimen_placeholders, import_mapped_table, TABLE_MAPPINGS,
};
use validation::{require_valid, validate_database};

#[cfg(feature = "migration-cli")]
pub use access::{run_extracted_import, sha256_file};

pub trait SourceDatabase {
    fn table_names(&mut self) -> Result<Vec<String>, ImportError>;
    fn read_table(&mut self, table: &str) -> Result<Vec<SourceRow>, ImportError>;
    fn row_count(&mut self, table: &str) -> Result<u64, ImportError>;
    fn source_schema(&mut self) -> Result<Vec<SourceTableSchema>, ImportError>;
    fn text_encoding_summary(&mut self) -> Result<TextEncodingSummary, ImportError>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceColumnSchema {
    pub name: String,
    pub access_type: String,
    pub ordinal: u32,
    pub nullable: bool,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceTableSchema {
    pub table: String,
    pub primary_keys: Vec<String>,
    pub columns: Vec<SourceColumnSchema>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TextEncodingSummary {
    pub source_encoding: String,
    pub text_cell_count: u64,
    pub thai_text_cell_count: u64,
    pub replacement_character_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SourceValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f64),
    Text(String),
}

impl SourceValue {
    fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    fn to_text(&self) -> Option<String> {
        match self {
            Self::Null => None,
            Self::Boolean(value) => Some(if *value { "-1" } else { "0" }.to_owned()),
            Self::Integer(value) => Some(value.to_string()),
            Self::Real(value) => Some(value.to_string()),
            Self::Text(value) => Some(value.clone()),
        }
    }

    fn to_integer(&self) -> Option<i64> {
        match self {
            Self::Null => None,
            Self::Boolean(value) => Some((*value).into()),
            Self::Integer(value) => Some(*value),
            Self::Real(value) if value.fract() == 0.0 => Some(*value as i64),
            Self::Real(_) => None,
            Self::Text(value) => value.trim().parse().ok(),
        }
    }

    fn to_real(&self) -> Option<f64> {
        match self {
            Self::Null => None,
            Self::Boolean(value) => Some(if *value { 1.0 } else { 0.0 }),
            Self::Integer(value) => Some(*value as f64),
            Self::Real(value) => Some(*value),
            Self::Text(value) => value.trim().parse().ok(),
        }
    }

    fn to_boolean(&self) -> Option<bool> {
        match self {
            Self::Null => None,
            Self::Boolean(value) => Some(*value),
            Self::Integer(value) => Some(*value != 0),
            Self::Real(value) => Some(*value != 0.0),
            Self::Text(value) => mapping::normalize_boolean(value).ok(),
        }
    }

    fn to_lookup_value(&self) -> Option<String> {
        self.to_text().filter(|value| !value.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SourceRow {
    values: BTreeMap<String, SourceValue>,
}

impl SourceRow {
    pub fn new(values: impl IntoIterator<Item = (String, SourceValue)>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(key, value)| (key.to_lowercase(), value))
                .collect(),
        }
    }

    pub fn get(&self, column: &str) -> Option<&SourceValue> {
        self.values.get(&column.to_lowercase())
    }

    fn json_payload(&self) -> Result<String, ImportError> {
        Ok(serde_json::to_string(&self.values)?)
    }
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database initialization error: {0}")]
    Database(#[from] db::DatabaseError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Access read error: {0}")]
    Access(String),
    #[error("conversion error: {0}")]
    Conversion(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("source table '{table}' row {row_number}: {message}")]
    Row {
        table: String,
        row_number: u64,
        message: String,
    },
    #[error(
        "output database already exists; pass --replace to replace it after a successful import"
    )]
    OutputExists,
}

impl ImportError {
    fn row(table: &str, zero_based_row: usize, message: String) -> Self {
        Self::Row {
            table: table.to_owned(),
            row_number: (zero_based_row + 1) as u64,
            message,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TableReport {
    pub source_table: String,
    pub destination_table: Option<String>,
    pub source_row_count: u64,
    pub imported_row_count: u64,
    pub skipped_row_count: u64,
    pub error_count: u64,
    pub synthetic_row_count: u64,
    pub status: String,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MigrationIssue {
    pub severity: String,
    pub category: String,
    pub source_table: Option<String>,
    pub destination_table: Option<String>,
    pub row_number: Option<u64>,
    pub identifier: Option<String>,
    pub message: String,
}

impl MigrationIssue {
    fn resolved_orphan(
        source_table: &str,
        destination_table: &str,
        identifier: String,
        message: &str,
    ) -> Self {
        Self {
            severity: "warning".to_owned(),
            category: "resolved_legacy_orphan".to_owned(),
            source_table: Some(source_table.to_owned()),
            destination_table: Some(destination_table.to_owned()),
            row_number: None,
            identifier: Some(identifier),
            message: message.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ValidationSummary {
    pub integrity_check: String,
    pub foreign_key_violation_count: u64,
    pub duplicate_identifier_count: u64,
    pub destination_text_cell_count: u64,
    pub destination_thai_text_cell_count: u64,
    pub destination_replacement_character_count: u64,
    pub null_violation_count: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MigrationReport {
    pub report_version: u32,
    pub generated_at_unix_seconds: u64,
    pub source_filename: String,
    pub source_sha256: String,
    pub destination_filename: String,
    pub schema_version: i64,
    pub all_data_local: bool,
    pub external_tables_imported: u64,
    pub source_schema: Vec<SourceTableSchema>,
    pub text_encoding: TextEncodingSummary,
    pub tables: Vec<TableReport>,
    pub validation: ValidationSummary,
    pub issues: Vec<MigrationIssue>,
}

pub struct ImportOptions<'a> {
    pub destination: &'a Path,
    pub replace: bool,
    pub source_filename: &'a str,
    pub source_sha256: &'a str,
    pub json_report: &'a Path,
    pub markdown_report: &'a Path,
}

const SPECIALTY_TABLES: &[&str] = &["CA Breast", "CA Coloretal", "DTPs", "F/U schedule"];

const INTENTIONALLY_EXCLUDED: &[(&str, &str)] = &[
    (
        "ANLink",
        "empty admission-link staging table; no local destination",
    ),
    (
        "Appdate",
        "temporary appointment workflow state; canonical appointments are imported from Appoint",
    ),
    (
        "Change",
        "legacy inventory-change workflow has no compatible destination table",
    ),
    (
        "Change Details",
        "legacy inventory-change detail has no compatible destination table",
    ),
    (
        "PI",
        "legacy lookup has no compatible destination and its semantics are undocumented",
    ),
    (
        "PrescriptionDetails",
        "legacy medication-error structure has no compatible destination table",
    ),
    (
        "TblAlert",
        "legacy alert-name lookup has no compatible destination table",
    ),
    (
        "TblECOG",
        "legacy ECOG lookup has no compatible destination table",
    ),
    (
        "TblIntTo",
        "intervention codes are preserved on Intervention; lookup has no destination",
    ),
    (
        "TblIntType",
        "intervention codes are preserved on Intervention; lookup has no destination",
    ),
    (
        "TblME",
        "legacy medication-error lookup has no compatible destination table",
    ),
    (
        "TblOccupation",
        "patient occupation text is preserved directly; lookup is redundant",
    ),
    (
        "TblResponse",
        "response codes are preserved on Intervention; lookup has no destination",
    ),
    (
        "ราคายาเดิม",
        "historical drug-price archive; current drug master is imported from Tbldrug",
    ),
];

pub fn import_source(
    source: &mut dyn SourceDatabase,
    options: &ImportOptions<'_>,
) -> Result<MigrationReport, ImportError> {
    if options.destination.exists() && !options.replace {
        return Err(ImportError::OutputExists);
    }
    if let Some(parent) = options.destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary = temporary_database_path(options.destination);
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }

    let import_result = import_to_temporary(source, options, &temporary);
    let report = match import_result {
        Ok(report) => report,
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
    };

    replace_output(&temporary, options.destination, options.replace)?;
    report::write_json_report(&report, options.json_report)?;
    report::write_markdown_report(&report, options.markdown_report)?;
    Ok(report)
}

fn import_to_temporary(
    source: &mut dyn SourceDatabase,
    options: &ImportOptions<'_>,
    temporary: &Path,
) -> Result<MigrationReport, ImportError> {
    let mut connection = Connection::open(temporary)?;
    db::configure_connection(&connection)?;
    db::apply_migrations(&connection)?;
    let source_tables = source.table_names()?;
    let source_schema = source.source_schema()?;
    let text_encoding = source.text_encoding_summary()?;
    let external_tables = source_tables
        .iter()
        .filter(|name| name.to_ascii_lowercase().starts_with("dbo_"))
        .count();
    if external_tables > 0 {
        // Presence is allowed as legacy evidence; import plans never reference these tables.
    }

    let transaction = connection.transaction()?;
    let (mut tables, mut issues) = import_transaction(source, &transaction, &source_tables)?;
    let validation = validate_database(&transaction)?;
    require_valid(&validation)?;
    if text_encoding.thai_text_cell_count > 0 && validation.destination_thai_text_cell_count == 0 {
        return Err(ImportError::Validation(
            "source contains Thai text but no Thai text survived in the destination".to_owned(),
        ));
    }
    transaction.commit()?;

    tables.sort_by(|left, right| left.source_table.cmp(&right.source_table));
    issues.sort_by(|left, right| {
        left.source_table
            .cmp(&right.source_table)
            .then(left.identifier.cmp(&right.identifier))
    });
    let schema_version: i64 = connection
        .query_row(
            "SELECT value FROM app_meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )?
        .parse()
        .map_err(|_| ImportError::Validation("schema version is not an integer".into()))?;

    Ok(MigrationReport {
        report_version: 1,
        generated_at_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        source_filename: options.source_filename.to_owned(),
        source_sha256: options.source_sha256.to_owned(),
        destination_filename: options
            .destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("oncoflow.db")
            .to_owned(),
        schema_version,
        all_data_local: true,
        external_tables_imported: 0,
        source_schema,
        text_encoding,
        tables,
        validation,
        issues,
    })
}

fn import_transaction(
    source: &mut dyn SourceDatabase,
    transaction: &Transaction<'_>,
    source_tables: &[String],
) -> Result<(Vec<TableReport>, Vec<MigrationIssue>), ImportError> {
    let source_table_set = source_tables
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut reports = Vec::new();
    let mut issues = Vec::new();
    let mut placeholders_created = false;
    let mut synthetic_regimens = 0;
    let mut synthetic_groups = 0;
    let mut synthetic_drugs = 0;
    let mut synthetic_drug_detail_groups = 0;

    for mapping in TABLE_MAPPINGS {
        if !source_table_set.contains(&mapping.source.to_ascii_lowercase()) {
            return Err(ImportError::Access(format!(
                "required source table '{}' is missing",
                mapping.source
            )));
        }
        if mapping.source == "Tblregimen details1" && !placeholders_created {
            let groups = source.read_table("Tblregimen details1")?;
            synthetic_regimens = create_regimen_placeholders(transaction, &groups, &mut issues)?;
            placeholders_created = true;
        }
        if mapping.source == "Tblregimen details2" {
            let items = source.read_table("Tblregimen details2")?;
            let (synthetic_parent, groups) =
                create_regimen_group_placeholders(transaction, &items, &mut issues)?;
            synthetic_regimens += synthetic_parent;
            synthetic_groups = groups;
        }
        if mapping.source == "TblDrug Details2" {
            let details = source.read_table("TblDrug Details2")?;
            (synthetic_drugs, synthetic_drug_detail_groups) =
                create_drug_detail_placeholders(transaction, &details, &mut issues)?;
        }
        reports.push(import_mapped_table(
            source,
            transaction,
            mapping,
            &mut issues,
        )?);
    }

    if let Some(report) = reports
        .iter_mut()
        .find(|report| report.destination_table.as_deref() == Some("regimens"))
    {
        report.synthetic_row_count = synthetic_regimens;
    }
    if let Some(report) = reports
        .iter_mut()
        .find(|report| report.destination_table.as_deref() == Some("regimen_groups"))
    {
        report.synthetic_row_count = synthetic_groups;
    }
    if let Some(report) = reports
        .iter_mut()
        .find(|report| report.destination_table.as_deref() == Some("drugs"))
    {
        report.synthetic_row_count = synthetic_drugs;
    }
    if let Some(report) = reports
        .iter_mut()
        .find(|report| report.destination_table.as_deref() == Some("drug_detail_groups"))
    {
        report.synthetic_row_count = synthetic_drug_detail_groups;
    }

    let mut synthetic_patients = 0;
    for table in SPECIALTY_TABLES {
        let (report, created_patients) =
            import_specialty_table(source, transaction, table, &mut issues)?;
        synthetic_patients += created_patients;
        reports.push(report);
    }
    if let Some(report) = reports
        .iter_mut()
        .find(|report| report.destination_table.as_deref() == Some("patients"))
    {
        report.synthetic_row_count += synthetic_patients;
    }

    for (table, reason) in INTENTIONALLY_EXCLUDED {
        if source_table_set.contains(&table.to_ascii_lowercase()) {
            reports.push(TableReport {
                source_table: (*table).to_owned(),
                destination_table: None,
                source_row_count: source.row_count(table)?,
                imported_row_count: 0,
                skipped_row_count: source.row_count(table)?,
                error_count: 0,
                synthetic_row_count: 0,
                status: "intentionally_excluded".to_owned(),
                notes: vec![(*reason).to_owned()],
            });
        }
    }

    let planned = TABLE_MAPPINGS
        .iter()
        .map(|mapping| mapping.source.to_ascii_lowercase())
        .chain(
            SPECIALTY_TABLES
                .iter()
                .map(|table| table.to_ascii_lowercase()),
        )
        .chain(
            INTENTIONALLY_EXCLUDED
                .iter()
                .map(|(table, _)| table.to_ascii_lowercase()),
        )
        .collect::<BTreeSet<_>>();
    for unclassified in source_table_set.difference(&planned) {
        if !unclassified.starts_with("dbo_") && !unclassified.starts_with("msys") {
            return Err(ImportError::Validation(format!(
                "source table '{unclassified}' is neither mapped nor explicitly excluded"
            )));
        }
    }

    Ok((reports, issues))
}

fn import_specialty_table(
    source: &mut dyn SourceDatabase,
    transaction: &Transaction<'_>,
    table: &str,
    issues: &mut Vec<MigrationIssue>,
) -> Result<(TableReport, u64), ImportError> {
    let rows = source.read_table(table)?;
    let source_count = rows.len() as u64;
    let mut imported = 0;
    let mut skipped = 0;
    let mut synthetic_patients = 0;

    for (index, row) in rows.iter().enumerate() {
        let legacy_hn = row
            .get("hn")
            .and_then(SourceValue::to_lookup_value)
            .unwrap_or_default();
        let mut patient_id = transaction
            .query_row(
                "SELECT id FROM patients WHERE legacy_hn = ?1",
                params![legacy_hn],
                |result| result.get::<_, i64>(0),
            )
            .optional()?;
        if patient_id.is_none() && !legacy_hn.is_empty() {
            transaction.execute(
                "INSERT INTO patients (legacy_hn, title) VALUES (?1, ?2)",
                params![legacy_hn, "[Legacy specialty-only patient]"],
            )?;
            patient_id = Some(transaction.last_insert_rowid());
            synthetic_patients += 1;
            issues.push(MigrationIssue {
                severity: "warning".to_owned(),
                category: "resolved_specialty_patient_orphan".to_owned(),
                source_table: Some(table.to_owned()),
                destination_table: Some("patients".to_owned()),
                row_number: Some((index + 1) as u64),
                identifier: None,
                message: "created a synthetic patient parent for a specialty-only legacy record"
                    .to_owned(),
            });
        }
        let Some(patient_id) = patient_id else {
            skipped += 1;
            issues.push(MigrationIssue {
                severity: "warning".to_owned(),
                category: "specialty_record_without_patient".to_owned(),
                source_table: Some(table.to_owned()),
                destination_table: Some("legacy_specialty_records".to_owned()),
                row_number: Some((index + 1) as u64),
                identifier: None,
                message: "specialty record was skipped because it has no patient identifier"
                    .to_owned(),
            });
            continue;
        };

        transaction.execute(
            "INSERT INTO legacy_specialty_records (patient_id, source_table, legacy_payload_json) VALUES (?1, ?2, ?3)",
            params![patient_id, table, row.json_payload()?],
        )?;
        imported += 1;
    }

    Ok((
        TableReport {
            source_table: table.to_owned(),
            destination_table: Some("legacy_specialty_records".to_owned()),
            source_row_count: source_count,
            imported_row_count: imported,
            skipped_row_count: skipped,
            error_count: skipped,
            synthetic_row_count: 0,
            status: if skipped == 0 {
                "imported"
            } else {
                "imported_with_skips"
            }
            .to_owned(),
            notes: vec!["preserved as compatibility JSON without interpretation".to_owned()],
        },
        synthetic_patients,
    ))
}

fn temporary_database_path(destination: &Path) -> PathBuf {
    let filename = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("oncoflow.db");
    destination.with_file_name(format!(".{filename}.importing-{}", std::process::id()))
}

fn replace_output(temporary: &Path, destination: &Path, replace: bool) -> Result<(), ImportError> {
    if !destination.exists() {
        fs::rename(temporary, destination)?;
        return Ok(());
    }
    if !replace {
        return Err(ImportError::OutputExists);
    }

    let backup = destination.with_extension("db.pre-import-backup");
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    fs::rename(destination, &backup)?;
    if let Err(error) = fs::rename(temporary, destination) {
        let _ = fs::rename(&backup, destination);
        return Err(error.into());
    }
    fs::remove_file(backup)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SyntheticSource {
        tables: BTreeMap<String, Vec<SourceRow>>,
    }

    impl SyntheticSource {
        fn empty() -> Self {
            let mut tables = BTreeMap::new();
            for mapping in TABLE_MAPPINGS {
                tables.insert(mapping.source.to_owned(), Vec::new());
            }
            for table in SPECIALTY_TABLES {
                tables.insert((*table).to_owned(), Vec::new());
            }
            Self { tables }
        }
    }

    impl SourceDatabase for SyntheticSource {
        fn table_names(&mut self) -> Result<Vec<String>, ImportError> {
            Ok(self.tables.keys().cloned().collect())
        }

        fn read_table(&mut self, table: &str) -> Result<Vec<SourceRow>, ImportError> {
            Ok(self.tables.get(table).cloned().unwrap_or_default())
        }

        fn row_count(&mut self, table: &str) -> Result<u64, ImportError> {
            Ok(self.tables.get(table).map_or(0, |rows| rows.len()) as u64)
        }

        fn source_schema(&mut self) -> Result<Vec<SourceTableSchema>, ImportError> {
            Ok(Vec::new())
        }

        fn text_encoding_summary(&mut self) -> Result<TextEncodingSummary, ImportError> {
            Ok(TextEncodingSummary::default())
        }
    }

    fn row(values: &[(&str, SourceValue)]) -> SourceRow {
        SourceRow::new(
            values
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone())),
        )
    }

    #[test]
    fn preserves_null_and_numeric_conversions() {
        assert_eq!(SourceValue::Null.to_text(), None);
        assert_eq!(SourceValue::Text("42".into()).to_integer(), Some(42));
        assert_eq!(SourceValue::Text("12.5".into()).to_real(), Some(12.5));
        assert_eq!(SourceValue::Integer(7).to_real(), Some(7.0));
    }

    #[test]
    fn mapping_imports_synthetic_lookup_row() {
        let mut source = SyntheticSource::empty();
        source.tables.insert(
            "Tblunit".into(),
            vec![row(&[
                ("unitcode", SourceValue::Text("A".into())),
                ("unitname", SourceValue::Text("หน่วย".into())),
            ])],
        );
        let mut connection = Connection::open_in_memory().unwrap();
        db::configure_connection(&connection).unwrap();
        db::apply_migrations(&connection).unwrap();
        let transaction = connection.transaction().unwrap();
        let table_names = source.table_names().unwrap();

        import_transaction(&mut source, &transaction, &table_names).unwrap();

        let name: String = transaction
            .query_row(
                "SELECT unit_name FROM units WHERE legacy_unitcode='A'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "หน่วย");
    }

    #[test]
    fn duplicate_identifier_rolls_back_transaction() {
        let mut source = SyntheticSource::empty();
        source.tables.insert(
            "Tblunit".into(),
            vec![
                row(&[
                    ("unitcode", SourceValue::Text("A".into())),
                    ("unitname", SourceValue::Text("first".into())),
                ]),
                row(&[
                    ("unitcode", SourceValue::Text("A".into())),
                    ("unitname", SourceValue::Text("duplicate".into())),
                ]),
            ],
        );
        let mut connection = Connection::open_in_memory().unwrap();
        db::configure_connection(&connection).unwrap();
        db::apply_migrations(&connection).unwrap();

        {
            let transaction = connection.transaction().unwrap();
            let table_names = source.table_names().unwrap();
            assert!(import_transaction(&mut source, &transaction, &table_names).is_err());
        }

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM units", [], |result| result.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
