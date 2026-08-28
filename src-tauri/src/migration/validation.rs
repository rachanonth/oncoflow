use rusqlite::Connection;

use super::{ImportError, ValidationSummary};

pub(crate) fn validate_database(connection: &Connection) -> Result<ValidationSummary, ImportError> {
    let integrity_check: String =
        connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;

    let mut foreign_key_statement = connection.prepare("PRAGMA foreign_key_check")?;
    let foreign_key_rows = foreign_key_statement.query_map([], |_| Ok(()))?;
    let mut foreign_key_violation_count = 0_u64;
    for row in foreign_key_rows {
        row?;
        foreign_key_violation_count += 1;
    }

    let duplicate_checks = [
        ("patients", "legacy_hn"),
        ("drugs", "legacy_dcode"),
        ("orders", "legacy_orderid"),
        ("diagnoses", "legacy_diagcode"),
        ("regimens", "legacy_regcode"),
        ("users", "username"),
    ];
    let mut duplicate_identifier_count = 0_u64;
    for (table, column) in duplicate_checks {
        let sql = format!(
            "SELECT COUNT(*) FROM (SELECT \"{column}\" FROM \"{table}\" WHERE \"{column}\" IS NOT NULL GROUP BY \"{column}\" HAVING COUNT(*) > 1)"
        );
        duplicate_identifier_count += connection.query_row(&sql, [], |row| row.get::<_, u64>(0))?;
    }

    let (
        destination_text_cell_count,
        destination_thai_text_cell_count,
        destination_replacement_character_count,
    ) = inspect_destination_text(connection)?;
    let null_violation_count = count_null_violations(connection)?;

    Ok(ValidationSummary {
        integrity_check,
        foreign_key_violation_count,
        duplicate_identifier_count,
        destination_text_cell_count,
        destination_thai_text_cell_count,
        destination_replacement_character_count,
        null_violation_count,
    })
}

pub(crate) fn require_valid(summary: &ValidationSummary) -> Result<(), ImportError> {
    if summary.integrity_check != "ok" {
        return Err(ImportError::Validation(format!(
            "SQLite integrity_check returned '{}'",
            summary.integrity_check
        )));
    }
    if summary.foreign_key_violation_count != 0 {
        return Err(ImportError::Validation(format!(
            "SQLite foreign_key_check found {} violation(s)",
            summary.foreign_key_violation_count
        )));
    }
    if summary.duplicate_identifier_count != 0 {
        return Err(ImportError::Validation(format!(
            "duplicate identifier validation found {} duplicate(s)",
            summary.duplicate_identifier_count
        )));
    }
    if summary.destination_replacement_character_count != 0 {
        return Err(ImportError::Validation(format!(
            "destination contains {} Unicode replacement character(s)",
            summary.destination_replacement_character_count
        )));
    }
    if summary.null_violation_count != 0 {
        return Err(ImportError::Validation(format!(
            "destination contains {} NOT NULL violation(s)",
            summary.null_violation_count
        )));
    }
    Ok(())
}

fn count_null_violations(connection: &Connection) -> Result<u64, ImportError> {
    let mut table_statement = connection.prepare(
        "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let table_names = table_statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut violations = 0_u64;

    for table in table_names {
        let quoted_table = quote_identifier(&table);
        let mut column_statement =
            connection.prepare(&format!("PRAGMA table_info({quoted_table})"))?;
        let required_columns = column_statement
            .query_map([], |row| {
                let name = row.get::<_, String>(1)?;
                let not_null = row.get::<_, i64>(3)? != 0;
                Ok((name, not_null))
            })?
            .filter_map(|result| match result {
                Ok((name, true)) => Some(Ok(name)),
                Ok((_, false)) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;
        for column in required_columns {
            let sql = format!(
                "SELECT COUNT(*) FROM {quoted_table} WHERE {} IS NULL",
                quote_identifier(&column)
            );
            violations += connection.query_row(&sql, [], |row| row.get::<_, u64>(0))?;
        }
    }
    Ok(violations)
}

fn inspect_destination_text(connection: &Connection) -> Result<(u64, u64, u64), ImportError> {
    let mut table_statement = connection.prepare(
        "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let table_names = table_statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut text_cells = 0_u64;
    let mut thai_cells = 0_u64;
    let mut replacement_characters = 0_u64;

    for table in table_names {
        let quoted_table = quote_identifier(&table);
        let mut column_statement =
            connection.prepare(&format!("PRAGMA table_info({quoted_table})"))?;
        let text_columns = column_statement
            .query_map([], |row| {
                let name = row.get::<_, String>(1)?;
                let declared_type = row.get::<_, String>(2)?;
                Ok((name, declared_type))
            })?
            .filter_map(|result| match result {
                Ok((name, declared_type))
                    if declared_type.to_ascii_uppercase().contains("TEXT") =>
                {
                    Some(Ok(name))
                }
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;
        if text_columns.is_empty() {
            continue;
        }

        let select_columns = text_columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let mut value_statement =
            connection.prepare(&format!("SELECT {select_columns} FROM {quoted_table}"))?;
        let mut rows = value_statement.query([])?;
        while let Some(row) = rows.next()? {
            for index in 0..text_columns.len() {
                let Some(text) = row.get::<_, Option<String>>(index)? else {
                    continue;
                };
                text_cells += 1;
                if text
                    .chars()
                    .any(|character| ('\u{0e00}'..='\u{0e7f}').contains(&character))
                {
                    thai_cells += 1;
                }
                replacement_characters += text
                    .chars()
                    .filter(|character| *character == '\u{fffd}')
                    .count() as u64;
            }
        }
    }
    Ok((text_cells, thai_cells, replacement_characters))
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn validates_clean_database() {
        let connection = Connection::open_in_memory().unwrap();
        db::configure_connection(&connection).unwrap();
        db::apply_migrations(&connection).unwrap();

        let summary = validate_database(&connection).unwrap();

        assert_eq!(summary.integrity_check, "ok");
        assert_eq!(summary.foreign_key_violation_count, 0);
        assert_eq!(summary.duplicate_identifier_count, 0);
        assert_eq!(summary.destination_replacement_character_count, 0);
        assert_eq!(summary.null_violation_count, 0);
        require_valid(&summary).unwrap();
    }
}
