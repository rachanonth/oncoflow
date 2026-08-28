use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DatabaseIssue {
    pub code: String,
    pub title: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartupStatus {
    pub database_ready: bool,
    pub database_location: String,
    pub issue: Option<DatabaseIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackupManifest {
    pub format_version: u32,
    pub application_name: String,
    pub application_version: String,
    pub schema_version: i64,
    pub created_at: String,
    pub database_file: String,
    pub database_size_bytes: u64,
    pub sha256: String,
    pub integrity_check: String,
    pub foreign_key_violations: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackupResult {
    pub location: String,
    pub manifest_location: String,
    pub file_name: String,
    pub created_at: String,
    pub schema_version: i64,
    pub application_version: String,
    pub integrity_check: String,
    pub foreign_key_violations: i64,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestorePreflight {
    pub location: String,
    pub file_name: String,
    pub schema_version: i64,
    pub supported_schema_version: i64,
    pub requires_migration: bool,
    pub created_at: Option<String>,
    pub backup_application_version: Option<String>,
    pub integrity_check: String,
    pub foreign_key_violations: i64,
    pub sha256: String,
    pub size_bytes: u64,
    pub confirmation_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestoreInput {
    pub backup_path: String,
    pub expected_sha256: String,
    pub confirmation_token: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestoreResult {
    pub restored_schema_version: i64,
    pub migrated_from_schema_version: Option<i64>,
    pub recovery_backup_location: String,
    pub recovery_backup_sha256: String,
    pub restored_backup_sha256: String,
    pub session_cleared: bool,
    pub restart_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Diagnostics {
    pub application_name: String,
    pub application_version: String,
    pub schema_version: i64,
    pub clinical_ruleset_version: String,
    pub label_template_version: String,
    pub label_renderer_version: String,
    pub database_location: String,
    pub database_size_bytes: u64,
    pub integrity_check: String,
    pub foreign_key_violations: i64,
    pub last_backup_at: Option<String>,
    pub platform: String,
    pub automatic_backup_policy: String,
}
