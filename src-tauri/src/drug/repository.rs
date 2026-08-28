use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use super::{
    DrugDetail, DrugFormOptions, DrugInput, DrugListRequest, DrugListResponse, DrugLookupOption,
    DrugSortField, DrugSummary, SortDirection,
};

pub(super) fn list_drugs(
    connection: &Connection,
    request: &DrugListRequest,
) -> rusqlite::Result<DrugListResponse> {
    let search = request.search.as_deref().unwrap_or("").trim();
    let pattern = format!("%{}%", escape_like(search));
    let inventory_filter = request.inventory_enabled.map(i64::from);
    let limit = request.limit.unwrap_or(100).clamp(1, 200) as i64;
    let offset = request.offset.unwrap_or(0) as i64;
    let direction = match request.sort_direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    let order = match request.sort_by {
        DrugSortField::Code => format!("d.legacy_dcode {direction}"),
        DrugSortField::Name => format!("d.drug_name {direction}, d.legacy_dcode ASC"),
        DrugSortField::Unit => format!(
            "CASE WHEN u.unit_name IS NULL THEN 1 ELSE 0 END, u.unit_name {direction}, d.drug_name ASC"
        ),
        DrugSortField::Inventory => format!(
            "d.inventory_enabled {direction},
             CASE WHEN (SELECT SUM(quantity_delta) FROM inventory_movements WHERE drug_id=d.id) IS NULL THEN 1 ELSE 0 END,
             (SELECT SUM(quantity_delta) FROM inventory_movements WHERE drug_id=d.id) {direction},
             d.drug_name ASC"
        ),
    };
    let where_clause = "(?1 = '%%' OR d.legacy_dcode LIKE ?1 ESCAPE '\\' COLLATE NOCASE OR d.drug_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE) AND (?2 IS NULL OR d.inventory_enabled = ?2)";

    let total = connection.query_row(
        &format!("SELECT COUNT(*) FROM drugs d WHERE {where_clause}"),
        params![pattern, inventory_filter],
        |row| row.get::<_, u64>(0),
    )?;
    let sql = format!(
        "SELECT d.id, d.legacy_dcode, d.drug_name, u.unit_name, d.package,
                d.inventory_enabled, d.inventory_min, d.inventory_max,
                (SELECT SUM(quantity_delta) FROM inventory_movements WHERE drug_id=d.id)
         FROM drugs d
         LEFT JOIN units u ON u.id = d.unit_id
         WHERE {where_clause}
         ORDER BY {order}
         LIMIT ?3 OFFSET ?4"
    );
    let mut statement = connection.prepare(&sql)?;
    let items = statement
        .query_map(
            params![pattern, inventory_filter, limit, offset],
            map_drug_summary,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DrugListResponse { items, total })
}

pub(super) fn get_drug(
    connection: &Connection,
    drug_id: i64,
) -> rusqlite::Result<Option<DrugDetail>> {
    connection
        .query_row(
            "SELECT d.id, d.legacy_dcode, d.drug_name, d.unit_id, u.unit_name,
                    d.dose_per_pack, d.volume_per_pack_ml, d.package, d.detail,
                    d.price, d.theory, d.marker, d.default_diluent_id,
                    dl.diluent_name, d.default_route_id, r.route_name,
                    d.default_rate, d.warning, d.storage, d.flag, d.expiry_time,
                    d.expiry_storage, d.max_dose,
                    CASE
                        WHEN d.max_dilution_alert IS NULL THEN NULL
                        WHEN d.max_dilution_alert = 0 THEN 0
                        ELSE 1
                    END,
                    d.max_dilution_hard,
                    CASE
                        WHEN d.cumulative_alert IS NULL THEN NULL
                        WHEN d.cumulative_alert = 0 THEN 0
                        ELSE 1
                    END,
                    d.cumulative_alert_hard, d.dilution_incompatibility,
                    CASE
                        WHEN d.inventory_cut IS NULL THEN NULL
                        WHEN d.inventory_cut = 0 THEN 0
                        ELSE 1
                    END,
                    d.inventory_min, d.inventory_max,
                    (SELECT SUM(quantity_delta) FROM inventory_movements WHERE drug_id=d.id),
                    d.inventory_enabled, d.homc_code,
                    d.legacy_exp, d.legacy_reg
             FROM drugs d
             LEFT JOIN units u ON u.id = d.unit_id
             LEFT JOIN diluents dl ON dl.id = d.default_diluent_id
             LEFT JOIN routes r ON r.id = d.default_route_id
             WHERE d.id = ?1",
            [drug_id],
            map_drug_detail,
        )
        .optional()
}

#[cfg(test)]
pub(super) fn get_drug_by_code(
    connection: &Connection,
    code: &str,
) -> rusqlite::Result<Option<DrugDetail>> {
    let id = connection
        .query_row(
            "SELECT id FROM drugs WHERE legacy_dcode = ?1",
            [code],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    id.map(|id| get_drug(connection, id))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn code_exists(
    transaction: &Transaction<'_>,
    code: &str,
    excluding_id: Option<i64>,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM drugs
            WHERE legacy_dcode = ?1 COLLATE NOCASE
              AND (?2 IS NULL OR id <> ?2)
         )",
        params![code, excluding_id],
        |row| row.get(0),
    )
}

pub(super) fn lookup_exists(
    transaction: &Transaction<'_>,
    table: &str,
    id: i64,
) -> rusqlite::Result<bool> {
    let sql = match table {
        "units" => "SELECT EXISTS(SELECT 1 FROM units WHERE id = ?1)",
        "routes" => "SELECT EXISTS(SELECT 1 FROM routes WHERE id = ?1)",
        "diluents" => "SELECT EXISTS(SELECT 1 FROM diluents WHERE id = ?1)",
        _ => unreachable!("drug lookup table is allow-listed"),
    };
    transaction.query_row(sql, [id], |row| row.get(0))
}

pub(super) fn insert_drug(
    transaction: &Transaction<'_>,
    input: &DrugInput,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO drugs (
            legacy_dcode, drug_name, unit_id, dose_per_pack, volume_per_pack_ml,
            package, detail, price, theory, marker, default_diluent_id,
            default_route_id, default_rate, warning, storage, flag, expiry_time,
            expiry_storage, max_dose, max_dilution_alert, max_dilution_hard,
            cumulative_alert, cumulative_alert_hard, dilution_incompatibility,
            inventory_cut, inventory_min, inventory_max, inventory_enabled
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26,
            ?27, ?28
         )",
        params![
            input.code,
            input.name,
            input.unit_id,
            input.dose_per_pack,
            input.volume_per_pack_ml,
            input.package,
            input.detail,
            input.price,
            input.theory,
            input.marker as i64,
            input.default_diluent_id,
            input.default_route_id,
            input.default_rate,
            input.warning,
            input.storage,
            input.flag as i64,
            input.expiry_time,
            input.expiry_storage,
            input.max_dose,
            input.max_dilution_alert.map(i64::from),
            input.max_dilution_hard,
            input.cumulative_alert.map(i64::from),
            input.cumulative_alert_hard,
            input.dilution_incompatibility,
            input.inventory_cut.map(i64::from),
            input.inventory_min,
            input.inventory_max,
            input.inventory_enabled as i64,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_drug(
    transaction: &Transaction<'_>,
    drug_id: i64,
    input: &DrugInput,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE drugs SET
            legacy_dcode = ?1, drug_name = ?2, unit_id = ?3, dose_per_pack = ?4,
            volume_per_pack_ml = ?5, package = ?6, detail = ?7, price = ?8,
            theory = ?9, marker = ?10, default_diluent_id = ?11,
            default_route_id = ?12, default_rate = ?13, warning = ?14,
            storage = ?15, flag = ?16, expiry_time = ?17,
            expiry_storage = ?18, max_dose = ?19, max_dilution_alert = ?20,
            max_dilution_hard = ?21, cumulative_alert = ?22,
            cumulative_alert_hard = ?23, dilution_incompatibility = ?24,
            inventory_cut = ?25, inventory_min = ?26, inventory_max = ?27,
            inventory_enabled = ?28
         WHERE id = ?29",
        params![
            input.code,
            input.name,
            input.unit_id,
            input.dose_per_pack,
            input.volume_per_pack_ml,
            input.package,
            input.detail,
            input.price,
            input.theory,
            input.marker as i64,
            input.default_diluent_id,
            input.default_route_id,
            input.default_rate,
            input.warning,
            input.storage,
            input.flag as i64,
            input.expiry_time,
            input.expiry_storage,
            input.max_dose,
            input.max_dilution_alert.map(i64::from),
            input.max_dilution_hard,
            input.cumulative_alert.map(i64::from),
            input.cumulative_alert_hard,
            input.dilution_incompatibility,
            input.inventory_cut.map(i64::from),
            input.inventory_min,
            input.inventory_max,
            input.inventory_enabled as i64,
            drug_id,
        ],
    )
}

pub(super) fn form_options(connection: &Connection) -> rusqlite::Result<DrugFormOptions> {
    let units = load_options(
        connection,
        "SELECT id, legacy_unitcode, unit_name, NULL FROM units ORDER BY unit_name, id",
    )?;
    let routes = load_options(
        connection,
        "SELECT id, legacy_rcode, route_name, NULL FROM routes ORDER BY route_name, id",
    )?;
    let diluents = load_options(
        connection,
        "SELECT id, legacy_dilcode, diluent_name, volume_ml FROM diluents ORDER BY diluent_name, volume_ml, id",
    )?;
    Ok(DrugFormOptions {
        suggested_code: next_generated_code(connection)?,
        units,
        routes,
        diluents,
    })
}

fn next_generated_code(connection: &Connection) -> rusqlite::Result<String> {
    let mut sequence: i64 =
        connection.query_row("SELECT COALESCE(MAX(id),0)+1 FROM drugs", [], |row| {
            row.get(0)
        })?;
    loop {
        let candidate = format!("OF-D{sequence:06}");
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM drugs WHERE legacy_dcode=?1 COLLATE NOCASE)",
            [&candidate],
            |row| row.get(0),
        )?;
        if !exists {
            return Ok(candidate);
        }
        sequence = sequence.saturating_add(1);
    }
}

fn load_options(connection: &Connection, sql: &str) -> rusqlite::Result<Vec<DrugLookupOption>> {
    connection
        .prepare(sql)?
        .query_map([], |row| {
            Ok(DrugLookupOption {
                id: row.get(0)?,
                code: row.get(1)?,
                label: row.get(2)?,
                volume_ml: row.get(3)?,
            })
        })?
        .collect()
}

fn map_drug_summary(row: &Row<'_>) -> rusqlite::Result<DrugSummary> {
    Ok(DrugSummary {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        unit: row.get(3)?,
        package: row.get(4)?,
        inventory_enabled: row.get::<_, i64>(5)? != 0,
        inventory_min: row.get(6)?,
        inventory_max: row.get(7)?,
        inventory_quantity: row.get(8)?,
    })
}

fn map_drug_detail(row: &Row<'_>) -> rusqlite::Result<DrugDetail> {
    Ok(DrugDetail {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        unit_id: row.get(3)?,
        unit: row.get(4)?,
        dose_per_pack: row.get(5)?,
        volume_per_pack_ml: row.get(6)?,
        package: row.get(7)?,
        detail: row.get(8)?,
        price: row.get(9)?,
        theory: row.get(10)?,
        marker: row.get::<_, i64>(11)? != 0,
        default_diluent_id: row.get(12)?,
        default_diluent: row.get(13)?,
        default_route_id: row.get(14)?,
        default_route: row.get(15)?,
        default_rate: row.get(16)?,
        warning: row.get(17)?,
        storage: row.get(18)?,
        flag: row.get::<_, i64>(19)? != 0,
        expiry_time: row.get(20)?,
        expiry_storage: row.get(21)?,
        max_dose: row.get(22)?,
        max_dilution_alert: row.get::<_, Option<i64>>(23)?.map(|value| value != 0),
        max_dilution_hard: row.get(24)?,
        cumulative_alert: row.get::<_, Option<i64>>(25)?.map(|value| value != 0),
        cumulative_alert_hard: row.get(26)?,
        dilution_incompatibility: row.get(27)?,
        inventory_cut: row.get::<_, Option<i64>>(28)?.map(|value| value != 0),
        inventory_min: row.get(29)?,
        inventory_max: row.get(30)?,
        inventory_quantity: row.get(31)?,
        inventory_enabled: row.get::<_, i64>(32)? != 0,
        legacy_mapping_code: row.get(33)?,
        legacy_exp: row.get(34)?,
        legacy_reg: row.get(35)?,
    })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
