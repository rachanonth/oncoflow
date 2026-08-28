use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use super::{
    NormalizedRegimenItemInput, RegimenDetail, RegimenGroupDetail, RegimenGroupInput, RegimenInput,
    RegimenItemDetail, RegimenListRequest, RegimenListResponse, RegimenLookupOption,
    RegimenLookups, RegimenSortField, RegimenSummary, SortDirection,
};

pub(super) fn list_regimens(
    connection: &Connection,
    request: &RegimenListRequest,
) -> rusqlite::Result<RegimenListResponse> {
    let search = request.search.as_deref().unwrap_or("").trim();
    let pattern = format!("%{}%", escape_like(search));
    let limit = request.limit.unwrap_or(100).clamp(1, 200) as i64;
    let offset = request.offset.unwrap_or(0) as i64;
    let direction = match request.sort_direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    let order = match request.sort_by {
        RegimenSortField::Code => format!("r.legacy_regcode {direction}, r.id ASC"),
        RegimenSortField::Name => {
            format!("r.regimen_name {direction}, r.legacy_regcode ASC, r.id ASC")
        }
        RegimenSortField::Items => format!("item_count {direction}, r.regimen_name ASC"),
    };
    let where_clause = "(?1 = '%%' OR r.legacy_regcode LIKE ?1 ESCAPE '\\' COLLATE NOCASE OR r.regimen_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE)";
    let total = connection.query_row(
        &format!("SELECT COUNT(*) FROM regimens r WHERE {where_clause}"),
        [pattern.as_str()],
        |row| row.get::<_, u64>(0),
    )?;
    let sql = format!(
        "SELECT r.id, COALESCE(r.legacy_regcode, ''), r.regimen_name, r.marker,
                COUNT(DISTINCT g.id) AS group_count, COUNT(i.id) AS item_count
         FROM regimens r
         LEFT JOIN regimen_groups g ON g.regimen_id = r.id
         LEFT JOIN regimen_items i ON i.regimen_group_id = g.id
         WHERE {where_clause}
         GROUP BY r.id
         ORDER BY {order}
         LIMIT ?2 OFFSET ?3"
    );
    let items = connection
        .prepare(&sql)?
        .query_map(params![pattern, limit, offset], map_regimen_summary)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RegimenListResponse { items, total })
}

pub(super) fn get_regimen(
    connection: &Connection,
    regimen_id: i64,
) -> rusqlite::Result<Option<RegimenDetail>> {
    let detail = connection
        .query_row(
            "SELECT id, COALESCE(legacy_regcode, ''), regimen_name, marker, flag,
                    cycle_check, auto_mode, drug_alert, appointment_alert, counsel_alert
             FROM regimens WHERE id = ?1",
            [regimen_id],
            |row| {
                Ok(RegimenDetail {
                    id: row.get(0)?,
                    code: row.get(1)?,
                    name: row.get(2)?,
                    marker: row.get::<_, i64>(3)? != 0,
                    flag: row.get::<_, i64>(4)? != 0,
                    cycle_check: row.get::<_, i64>(5)? != 0,
                    auto_mode: row.get::<_, i64>(6)? != 0,
                    drug_alert: row.get::<_, i64>(7)? != 0,
                    appointment_alert: row.get::<_, i64>(8)? != 0,
                    counsel_alert: row.get::<_, i64>(9)? != 0,
                    groups: Vec::new(),
                })
            },
        )
        .optional()?;
    let Some(mut detail) = detail else {
        return Ok(None);
    };

    let mut group_statement = connection.prepare(
        "SELECT id, legacy_code, note, cycle_day, cycle_count
         FROM regimen_groups WHERE regimen_id = ?1 ORDER BY id ASC",
    )?;
    let groups = group_statement
        .query_map([regimen_id], |row| {
            Ok(RegimenGroupDetail {
                id: row.get(0)?,
                legacy_code: row.get(1)?,
                note: row.get(2)?,
                cycle_day: row.get(3)?,
                cycle_count: row.get(4)?,
                items: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut item_statement = connection.prepare(
        "SELECT i.id, i.regimen_group_id, i.drug_id, d.legacy_dcode, d.drug_name,
                i.dose, i.legacy_dose_text, i.unit_text, i.route_text, i.details,
                i.item_group, i.duration, i.start_day, i.ordering_no,
                i.default_diluent_id, dl.diluent_name, i.default_route_id,
                r.route_name, i.default_rate
         FROM regimen_items i
         JOIN drugs d ON d.id = i.drug_id
         LEFT JOIN diluents dl ON dl.id = i.default_diluent_id
         LEFT JOIN routes r ON r.id = i.default_route_id
         WHERE i.regimen_group_id = ?1
         ORDER BY CASE WHEN i.item_group IS NULL THEN 0 ELSE 1 END,
                  i.item_group COLLATE NOCASE,
                  CASE WHEN i.ordering_no IS NULL THEN 1 ELSE 0 END,
                  i.ordering_no,
                  i.id",
    )?;
    detail.groups = groups;
    for group in &mut detail.groups {
        group.items = item_statement
            .query_map([group.id], map_regimen_item)?
            .collect::<Result<Vec<_>, _>>()?;
    }
    Ok(Some(detail))
}

#[cfg(test)]
pub(super) fn get_regimen_by_code(
    connection: &Connection,
    code: &str,
) -> rusqlite::Result<Option<RegimenDetail>> {
    let id = connection
        .query_row(
            "SELECT id FROM regimens WHERE legacy_regcode = ?1 COLLATE NOCASE",
            [code],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    id.map(|id| get_regimen(connection, id))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn code_exists(
    transaction: &Transaction<'_>,
    code: &str,
    excluding_id: Option<i64>,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM regimens
         WHERE legacy_regcode = ?1 COLLATE NOCASE AND (?2 IS NULL OR id <> ?2))",
        params![code, excluding_id],
        |row| row.get(0),
    )
}

pub(super) fn regimen_exists(
    transaction: &Transaction<'_>,
    regimen_id: i64,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM regimens WHERE id = ?1)",
        [regimen_id],
        |row| row.get(0),
    )
}

pub(super) fn group_belongs_to_regimen(
    transaction: &Transaction<'_>,
    regimen_id: i64,
    group_id: i64,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM regimen_groups WHERE id = ?1 AND regimen_id = ?2)",
        params![group_id, regimen_id],
        |row| row.get(0),
    )
}

pub(super) fn item_belongs_to_regimen(
    transaction: &Transaction<'_>,
    regimen_id: i64,
    item_id: i64,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM regimen_items i
            JOIN regimen_groups g ON g.id = i.regimen_group_id
            WHERE i.id = ?1 AND g.regimen_id = ?2
         )",
        params![item_id, regimen_id],
        |row| row.get(0),
    )
}

pub(super) fn lookup_exists(
    transaction: &Transaction<'_>,
    table: &str,
    id: i64,
) -> rusqlite::Result<bool> {
    let sql = match table {
        "drugs" => "SELECT EXISTS(SELECT 1 FROM drugs WHERE id = ?1)",
        "routes" => "SELECT EXISTS(SELECT 1 FROM routes WHERE id = ?1)",
        "diluents" => "SELECT EXISTS(SELECT 1 FROM diluents WHERE id = ?1)",
        _ => unreachable!("regimen lookup table is allow-listed"),
    };
    transaction.query_row(sql, [id], |row| row.get(0))
}

pub(super) fn insert_regimen(
    transaction: &Transaction<'_>,
    input: &RegimenInput,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO regimens (
            legacy_regcode, regimen_name, marker, flag, cycle_check, auto_mode,
            drug_alert, appointment_alert, counsel_alert
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            input.code,
            input.name,
            input.marker as i64,
            input.flag as i64,
            input.cycle_check as i64,
            input.auto_mode as i64,
            input.drug_alert as i64,
            input.appointment_alert as i64,
            input.counsel_alert as i64,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_regimen(
    transaction: &Transaction<'_>,
    regimen_id: i64,
    input: &RegimenInput,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE regimens SET legacy_regcode=?1, regimen_name=?2, marker=?3,
            flag=?4, cycle_check=?5, auto_mode=?6, drug_alert=?7,
            appointment_alert=?8, counsel_alert=?9 WHERE id=?10",
        params![
            input.code,
            input.name,
            input.marker as i64,
            input.flag as i64,
            input.cycle_check as i64,
            input.auto_mode as i64,
            input.drug_alert as i64,
            input.appointment_alert as i64,
            input.counsel_alert as i64,
            regimen_id,
        ],
    )
}

pub(super) fn insert_group(
    transaction: &Transaction<'_>,
    regimen_id: i64,
    input: &RegimenGroupInput,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO regimen_groups(regimen_id,note,cycle_day,cycle_count)
         VALUES (?1,?2,?3,?4)",
        params![regimen_id, input.note, input.cycle_day, input.cycle_count],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_group(
    transaction: &Transaction<'_>,
    group_id: i64,
    input: &RegimenGroupInput,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE regimen_groups SET note=?1,cycle_day=?2,cycle_count=?3 WHERE id=?4",
        params![input.note, input.cycle_day, input.cycle_count, group_id],
    )
}

pub(super) fn group_item_count(
    transaction: &Transaction<'_>,
    group_id: i64,
) -> rusqlite::Result<u64> {
    transaction.query_row(
        "SELECT COUNT(*) FROM regimen_items WHERE regimen_group_id=?1",
        [group_id],
        |row| row.get(0),
    )
}

pub(super) fn delete_group(
    transaction: &Transaction<'_>,
    group_id: i64,
) -> rusqlite::Result<usize> {
    transaction.execute("DELETE FROM regimen_groups WHERE id=?1", [group_id])
}

pub(super) fn insert_item(
    transaction: &Transaction<'_>,
    normalized: &NormalizedRegimenItemInput,
) -> rusqlite::Result<i64> {
    let input = &normalized.input;
    transaction.execute(
        "INSERT INTO regimen_items (
            regimen_group_id,drug_id,dose,legacy_dose_text,unit_text,route_text,
            details,item_group,duration,start_day,ordering_no,default_diluent_id,
            default_route_id,default_rate
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            input.regimen_group_id,
            input.drug_id,
            normalized.parsed_dose,
            input.dose_text,
            input.unit_text,
            input.route_text,
            input.details,
            input.item_group,
            input.duration,
            input.start_day,
            input.ordering_no,
            input.default_diluent_id,
            input.default_route_id,
            input.default_rate,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_item(
    transaction: &Transaction<'_>,
    item_id: i64,
    normalized: &NormalizedRegimenItemInput,
) -> rusqlite::Result<usize> {
    let input = &normalized.input;
    transaction.execute(
        "UPDATE regimen_items SET regimen_group_id=?1,drug_id=?2,dose=?3,
            legacy_dose_text=?4,unit_text=?5,route_text=?6,details=?7,item_group=?8,
            duration=?9,start_day=?10,ordering_no=?11,default_diluent_id=?12,
            default_route_id=?13,default_rate=?14 WHERE id=?15",
        params![
            input.regimen_group_id,
            input.drug_id,
            normalized.parsed_dose,
            input.dose_text,
            input.unit_text,
            input.route_text,
            input.details,
            input.item_group,
            input.duration,
            input.start_day,
            input.ordering_no,
            input.default_diluent_id,
            input.default_route_id,
            input.default_rate,
            item_id,
        ],
    )
}

pub(super) fn delete_item(transaction: &Transaction<'_>, item_id: i64) -> rusqlite::Result<usize> {
    transaction.execute("DELETE FROM regimen_items WHERE id=?1", [item_id])
}

pub(super) fn reorder_candidates(
    transaction: &Transaction<'_>,
    group_id: i64,
    item_group: Option<&str>,
) -> rusqlite::Result<Vec<i64>> {
    transaction
        .prepare(
            "SELECT id FROM regimen_items
             WHERE regimen_group_id=?1 AND item_group IS ?2
             ORDER BY CASE WHEN ordering_no IS NULL THEN 1 ELSE 0 END, ordering_no, id",
        )?
        .query_map(params![group_id, item_group], |row| row.get(0))?
        .collect()
}

pub(super) fn set_item_order(
    transaction: &Transaction<'_>,
    item_id: i64,
    ordering_no: i64,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE regimen_items SET ordering_no=?1 WHERE id=?2",
        params![ordering_no, item_id],
    )
}

pub(super) fn lookups(connection: &Connection) -> rusqlite::Result<RegimenLookups> {
    Ok(RegimenLookups {
        drugs: load_options(
            connection,
            "SELECT id,legacy_dcode,drug_name FROM drugs ORDER BY drug_name,id",
        )?,
        routes: load_options(
            connection,
            "SELECT id,legacy_rcode,route_name FROM routes ORDER BY route_name,id",
        )?,
        diluents: load_options(
            connection,
            "SELECT id,legacy_dilcode,diluent_name FROM diluents ORDER BY diluent_name,id",
        )?,
    })
}

fn load_options(connection: &Connection, sql: &str) -> rusqlite::Result<Vec<RegimenLookupOption>> {
    connection
        .prepare(sql)?
        .query_map([], |row| {
            Ok(RegimenLookupOption {
                id: row.get(0)?,
                code: row.get(1)?,
                label: row.get(2)?,
            })
        })?
        .collect()
}

fn map_regimen_summary(row: &Row<'_>) -> rusqlite::Result<RegimenSummary> {
    Ok(RegimenSummary {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        marker: row.get::<_, i64>(3)? != 0,
        group_count: row.get(4)?,
        item_count: row.get(5)?,
    })
}

fn map_regimen_item(row: &Row<'_>) -> rusqlite::Result<RegimenItemDetail> {
    Ok(RegimenItemDetail {
        id: row.get(0)?,
        regimen_group_id: row.get(1)?,
        drug_id: row.get(2)?,
        drug_code: row.get(3)?,
        drug_name: row.get(4)?,
        dose: row.get(5)?,
        dose_text: row.get(6)?,
        unit_text: row.get(7)?,
        route_text: row.get(8)?,
        details: row.get(9)?,
        item_group: row.get(10)?,
        duration: row.get(11)?,
        start_day: row.get(12)?,
        ordering_no: row.get(13)?,
        default_diluent_id: row.get(14)?,
        default_diluent: row.get(15)?,
        default_route_id: row.get(16)?,
        default_route: row.get(17)?,
        default_rate: row.get(18)?,
    })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
