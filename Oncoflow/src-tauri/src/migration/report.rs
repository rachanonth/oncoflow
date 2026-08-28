use std::{fs, path::Path};

use serde::Serialize;

use super::{ImportError, MigrationReport};

pub(crate) fn write_json_report(report: &MigrationReport, path: &Path) -> Result<(), ImportError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(report)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

pub(crate) fn write_markdown_report(
    report: &MigrationReport,
    path: &Path,
) -> Result<(), ImportError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut markdown = String::from(
        "# OncoFlow MDB migration report\n\n\
         This report contains counts and schema diagnostics only. It intentionally omits patient names, addresses, clinical notes, and passwords.\n\n",
    );
    markdown.push_str(&format!(
        "- Source: `{}`\n- Source SHA-256: `{}`\n- Destination: `{}`\n- Schema version: {}\n- SQLite integrity check: `{}`\n- SQLite foreign-key violations: {}\n\n",
        report.source_filename,
        report.source_sha256,
        report.destination_filename,
        report.schema_version,
        report.validation.integrity_check,
        report.validation.foreign_key_violation_count,
    ));
    markdown.push_str(&format!(
        "- Source tables inspected: {}\n- Text cells decoded: {}\n- Thai text cells detected: {}\n- Unicode replacement characters: {}\n\n",
        report.source_schema.len(),
        report.text_encoding.text_cell_count,
        report.text_encoding.thai_text_cell_count,
        report.text_encoding.replacement_character_count,
    ));
    markdown.push_str(&format!(
        "- Destination text cells: {}\n- Destination Thai text cells: {}\n- Destination replacement characters: {}\n\n",
        report.validation.destination_text_cell_count,
        report.validation.destination_thai_text_cell_count,
        report.validation.destination_replacement_character_count,
    ));
    markdown.push_str(&format!(
        "- Destination NOT NULL violations: {}\n\n",
        report.validation.null_violation_count,
    ));
    markdown.push_str("## Table counts\n\n| Source table | Destination table | Source | Imported | Skipped | Errors | Synthetic | Status |\n|---|---:|---:|---:|---:|---:|---:|---|\n");
    for table in &report.tables {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            escape_cell(&table.source_table),
            escape_cell(table.destination_table.as_deref().unwrap_or("—")),
            table.source_row_count,
            table.imported_row_count,
            table.skipped_row_count,
            table.error_count,
            table.synthetic_row_count,
            escape_cell(&table.status),
        ));
    }

    markdown.push_str("\n## Migration issues\n\n");
    if report.issues.is_empty() {
        markdown.push_str("No migration issues were reported.\n");
    } else {
        for issue in &report.issues {
            let identifier = issue
                .identifier
                .as_deref()
                .map(|value| format!(" (legacy identifier `{}`)", escape_inline(value)))
                .unwrap_or_default();
            markdown.push_str(&format!(
                "- **{} / {}** — {}{}\n",
                escape_inline(&issue.severity),
                escape_inline(&issue.category),
                escape_inline(&issue.message),
                identifier,
            ));
        }
    }

    markdown.push_str("\n## Credential handling\n\nLegacy plaintext passwords were not copied. Imported legacy users are disabled and contain a non-credential placeholder until a later authentication migration is explicitly designed.\n");
    fs::write(path, markdown)?;
    Ok(())
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn escape_inline(value: &str) -> String {
    value.replace('`', "'").replace('\n', " ")
}

#[allow(dead_code)]
fn _assert_serializable<T: Serialize>() {}
