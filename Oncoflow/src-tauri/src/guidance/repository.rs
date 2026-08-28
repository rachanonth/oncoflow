use rusqlite::{params, Connection, OptionalExtension, Transaction};

use super::PageGuidanceRecord;

pub(super) fn list(connection: &Connection) -> rusqlite::Result<Vec<PageGuidanceRecord>> {
    connection
        .prepare("SELECT page_key,guidance FROM page_guidance ORDER BY page_key")?
        .query_map([], |row| {
            Ok(PageGuidanceRecord {
                page_key: row.get(0)?,
                guidance: Some(row.get(1)?),
            })
        })?
        .collect()
}

pub(super) fn load(
    connection: &Connection,
    page_key: &str,
) -> rusqlite::Result<Option<PageGuidanceRecord>> {
    connection
        .query_row(
            "SELECT page_key,guidance FROM page_guidance WHERE page_key=?1",
            [page_key],
            |row| {
                Ok(PageGuidanceRecord {
                    page_key: row.get(0)?,
                    guidance: Some(row.get(1)?),
                })
            },
        )
        .optional()
}

pub(super) fn upsert(
    transaction: &Transaction<'_>,
    page_key: &str,
    guidance: &str,
    actor_user_id: i64,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO page_guidance(page_key,guidance,updated_by_user_id,updated_at)
         VALUES(?1,?2,?3,CURRENT_TIMESTAMP)
         ON CONFLICT(page_key) DO UPDATE SET
           guidance=excluded.guidance,
           updated_by_user_id=excluded.updated_by_user_id,
           updated_at=CURRENT_TIMESTAMP",
        params![page_key, guidance, actor_user_id],
    )?;
    Ok(())
}

pub(super) fn remove(transaction: &Transaction<'_>, page_key: &str) -> rusqlite::Result<()> {
    transaction.execute("DELETE FROM page_guidance WHERE page_key=?1", [page_key])?;
    Ok(())
}
