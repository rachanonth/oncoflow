use rusqlite::{params, Connection};
use serde_json::Value;

pub(crate) fn append_event(
    connection: &Connection,
    user_id: Option<i64>,
    event_type: &str,
    entity_type: &str,
    entity_id: impl ToString,
    metadata: &Value,
) -> rusqlite::Result<i64> {
    let metadata_json = serde_json::to_string(metadata)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(error.into()))?;
    connection.execute(
        "INSERT INTO audit_events(user_id,event_type,entity_type,entity_id,metadata_json)
         VALUES(?1,?2,?3,?4,?5)",
        params![
            user_id,
            event_type,
            entity_type,
            entity_id.to_string(),
            metadata_json
        ],
    )?;
    Ok(connection.last_insert_rowid())
}
