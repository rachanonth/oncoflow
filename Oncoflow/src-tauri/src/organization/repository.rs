use rusqlite::{params, Connection, Transaction};

use super::ApplicationSettings;

pub(super) fn load(connection: &Connection) -> rusqlite::Result<ApplicationSettings> {
    connection.query_row(
        "SELECT hospital_name FROM application_settings WHERE id=1",
        [],
        |row| {
            Ok(ApplicationSettings {
                hospital_name: row.get(0)?,
            })
        },
    )
}

pub(super) fn update(
    transaction: &Transaction<'_>,
    hospital_name: Option<&str>,
    actor_user_id: i64,
) -> rusqlite::Result<()> {
    transaction.execute(
        "UPDATE application_settings
         SET hospital_name=?1,updated_by_user_id=?2,updated_at=CURRENT_TIMESTAMP
         WHERE id=1",
        params![hospital_name, actor_user_id],
    )?;
    Ok(())
}
