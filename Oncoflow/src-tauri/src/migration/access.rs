use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{
    import_source, ImportError, ImportOptions, MigrationReport, SourceDatabase, SourceRow,
    SourceValue,
};

pub fn run_extracted_import(
    source_path: &Path,
    extracted_json_lines: &str,
    options: &ImportOptions<'_>,
) -> Result<MigrationReport, ImportError> {
    let before_hash = sha256_file(source_path)?;
    if !options.source_sha256.eq_ignore_ascii_case(&before_hash) {
        return Err(ImportError::Validation(
            "source checksum changed between CLI validation and import".to_owned(),
        ));
    }

    let mut source = ExtractedAccessSource::parse(extracted_json_lines)?;
    let report = import_source(&mut source, options)?;

    let after_hash = sha256_file(source_path)?;
    if before_hash != after_hash {
        return Err(ImportError::Validation(
            "read-only source checksum changed during import".to_owned(),
        ));
    }
    Ok(report)
}

pub fn sha256_file(path: &Path) -> Result<String, ImportError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:X}", hasher.finalize()))
}

struct ExtractedAccessSource {
    tables: BTreeMap<String, ExtractedTable>,
    text_encoding: super::TextEncodingSummary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractedTable {
    name: String,
    row_count: u64,
    primary_keys: Vec<String>,
    columns: Vec<super::SourceColumnSchema>,
    rows: Vec<BTreeMap<String, serde_json::Value>>,
}

impl ExtractedAccessSource {
    fn parse(extracted_json_lines: &str) -> Result<Self, ImportError> {
        if extracted_json_lines.contains('\u{fffd}') {
            return Err(ImportError::Access(
                "ACE extraction output contains a Unicode replacement character".to_owned(),
            ));
        }

        let mut tables = BTreeMap::new();
        let mut text_cells = 0_u64;
        let mut thai_text_cells = 0_u64;
        for (line_index, line) in extracted_json_lines.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let extracted: ExtractedTable = serde_json::from_str(line).map_err(|error| {
                ImportError::Access(format!(
                    "ACE extraction protocol line {} is invalid JSON: {error}",
                    line_index + 1
                ))
            })?;
            if extracted.name.to_ascii_lowercase().starts_with("dbo_") {
                return Err(ImportError::Access(
                    "extractor returned a forbidden dbo_* table".to_owned(),
                ));
            }
            if extracted.rows.len() as u64 != extracted.row_count {
                return Err(ImportError::Access(format!(
                    "ACE extraction count mismatch for table '{}'",
                    extracted.name
                )));
            }
            for row in &extracted.rows {
                for value in row.values() {
                    if let serde_json::Value::String(text) = value {
                        text_cells += 1;
                        if text
                            .chars()
                            .any(|character| ('\u{0e00}'..='\u{0e7f}').contains(&character))
                        {
                            thai_text_cells += 1;
                        }
                    }
                }
            }
            tables.insert(extracted.name.to_lowercase(), extracted);
        }
        Ok(Self {
            tables,
            text_encoding: super::TextEncodingSummary {
                source_encoding: "ACE Unicode text converted strictly to UTF-8".to_owned(),
                text_cell_count: text_cells,
                thai_text_cell_count: thai_text_cells,
                replacement_character_count: 0,
            },
        })
    }
}

impl SourceDatabase for ExtractedAccessSource {
    fn table_names(&mut self) -> Result<Vec<String>, ImportError> {
        Ok(self
            .tables
            .values()
            .map(|table| table.name.clone())
            .collect())
    }

    fn read_table(&mut self, table: &str) -> Result<Vec<SourceRow>, ImportError> {
        let table = self.tables.get(&table.to_lowercase()).ok_or_else(|| {
            ImportError::Access(format!("source table '{table}' was not extracted"))
        })?;
        table
            .rows
            .iter()
            .map(|row| {
                let values = row
                    .iter()
                    .map(|(name, value)| Ok((name.clone(), json_to_source(value)?)))
                    .collect::<Result<Vec<_>, ImportError>>()?;
                Ok(SourceRow::new(values))
            })
            .collect()
    }

    fn row_count(&mut self, table: &str) -> Result<u64, ImportError> {
        self.tables
            .get(&table.to_lowercase())
            .map(|table| table.row_count)
            .ok_or_else(|| ImportError::Access(format!("source table '{table}' was not extracted")))
    }

    fn source_schema(&mut self) -> Result<Vec<super::SourceTableSchema>, ImportError> {
        Ok(self
            .tables
            .values()
            .map(|table| super::SourceTableSchema {
                table: table.name.clone(),
                primary_keys: table.primary_keys.clone(),
                columns: table.columns.clone(),
            })
            .collect())
    }

    fn text_encoding_summary(&mut self) -> Result<super::TextEncodingSummary, ImportError> {
        Ok(self.text_encoding.clone())
    }
}

fn json_to_source(value: &serde_json::Value) -> Result<SourceValue, ImportError> {
    match value {
        serde_json::Value::Null => Ok(SourceValue::Null),
        serde_json::Value::Bool(value) => Ok(SourceValue::Boolean(*value)),
        serde_json::Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                Ok(SourceValue::Integer(integer))
            } else if let Some(real) = value.as_f64() {
                Ok(SourceValue::Real(real))
            } else {
                Err(ImportError::Conversion(
                    "ACE emitted a numeric value outside supported SQLite range".to_owned(),
                ))
            }
        }
        serde_json::Value::String(value) => Ok(SourceValue::Text(value.clone())),
        _ => Err(ImportError::Access(
            "ACE emitted an unsupported nested value".to_owned(),
        )),
    }
}
