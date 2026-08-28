use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use super::{
    InventoryDetail, InventoryListRequest, InventoryListResponse, InventoryMovement,
    InventoryMovementListRequest, InventoryMovementListResponse, InventoryMovementType,
    InventorySortField, InventorySummary, SortDirection, StockState,
};

const BALANCE_CTE: &str = "WITH balances AS (
        SELECT drug_id,SUM(quantity_delta) AS current_stock
        FROM inventory_movements
        GROUP BY drug_id
     )";

pub(super) fn list_inventory(
    connection: &Connection,
    request: &InventoryListRequest,
) -> rusqlite::Result<InventoryListResponse> {
    let search = request.search.as_deref().unwrap_or("").trim();
    let pattern = format!("%{}%", escape_like(search));
    let tracked_only = i64::from(request.tracked_only);
    let low_stock_only = i64::from(request.low_stock_only);
    let limit = request.limit.unwrap_or(100).clamp(1, 200) as i64;
    let offset = request.offset.unwrap_or(0) as i64;
    let direction = match request.sort_direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    let state_rank = "CASE
        WHEN d.inventory_enabled=0 THEN 5
        WHEN b.current_stock IS NULL THEN 4
        WHEN b.current_stock < 0 THEN 0
        WHEN b.current_stock = 0 THEN 1
        WHEN d.inventory_min IS NOT NULL AND b.current_stock <= d.inventory_min THEN 2
        ELSE 3 END";
    let order = match request.sort_by {
        InventorySortField::Code => format!("d.legacy_dcode {direction}"),
        InventorySortField::Name => {
            format!("d.drug_name {direction}, d.legacy_dcode ASC")
        }
        InventorySortField::CurrentStock => format!(
            "CASE WHEN b.current_stock IS NULL THEN 1 ELSE 0 END,
             b.current_stock {direction}, d.drug_name ASC"
        ),
        InventorySortField::Minimum => format!(
            "CASE WHEN d.inventory_min IS NULL THEN 1 ELSE 0 END,
             d.inventory_min {direction}, d.drug_name ASC"
        ),
        InventorySortField::Maximum => format!(
            "CASE WHEN d.inventory_max IS NULL THEN 1 ELSE 0 END,
             d.inventory_max {direction}, d.drug_name ASC"
        ),
        InventorySortField::State => {
            format!("{state_rank} {direction}, d.drug_name ASC")
        }
    };
    let where_clause = "
        (?1='%%'
          OR d.legacy_dcode LIKE ?1 ESCAPE '\\' COLLATE NOCASE
          OR d.drug_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE)
        AND (?2=0 OR d.inventory_enabled=1)
        AND (
          ?3=0 OR (
            d.inventory_enabled=1
            AND b.current_stock IS NOT NULL
            AND (
              b.current_stock <= 0
              OR (d.inventory_min IS NOT NULL AND b.current_stock <= d.inventory_min)
            )
          )
        )";
    let total_sql = format!(
        "{BALANCE_CTE}
         SELECT COUNT(*)
         FROM drugs d
         LEFT JOIN balances b ON b.drug_id=d.id
         WHERE {where_clause}"
    );
    let total = connection.query_row(
        &total_sql,
        params![pattern, tracked_only, low_stock_only],
        |row| row.get::<_, u64>(0),
    )?;
    let sql = format!(
        "{BALANCE_CTE}
         SELECT d.id,d.legacy_dcode,d.drug_name,u.unit_name,d.package,
                b.current_stock,d.inventory_min,d.inventory_max,d.inventory_enabled
         FROM drugs d
         LEFT JOIN units u ON u.id=d.unit_id
         LEFT JOIN balances b ON b.drug_id=d.id
         WHERE {where_clause}
         ORDER BY {order}
         LIMIT ?4 OFFSET ?5"
    );
    let mut statement = connection.prepare(&sql)?;
    let items = statement
        .query_map(
            params![pattern, tracked_only, low_stock_only, limit, offset],
            map_inventory_summary,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(InventoryListResponse { items, total })
}

pub(super) fn get_inventory_item(
    connection: &Connection,
    drug_id: i64,
) -> rusqlite::Result<Option<InventoryDetail>> {
    let sql = format!(
        "{BALANCE_CTE}
         SELECT d.id,d.legacy_dcode,d.drug_name,u.unit_name,d.package,
                b.current_stock,d.inventory_min,d.inventory_max,d.inventory_enabled,
                d.inventory_qty,
                CASE
                  WHEN d.inventory_cut IS NULL THEN NULL
                  WHEN d.inventory_cut=0 THEN 0
                  ELSE 1
                END,
                d.dose_per_pack,d.volume_per_pack_ml,
                (SELECT COUNT(*) FROM inventory_events e WHERE e.drug_id=d.id)
         FROM drugs d
         LEFT JOIN units u ON u.id=d.unit_id
         LEFT JOIN balances b ON b.drug_id=d.id
         WHERE d.id=?1"
    );
    connection
        .query_row(&sql, [drug_id], |row| {
            Ok(InventoryDetail {
                summary: map_inventory_summary(row)?,
                legacy_inventory_snapshot: row.get(9)?,
                legacy_inventory_cutoff: row.get::<_, Option<i64>>(10)?.map(|value| value != 0),
                dose_per_pack: row.get(11)?,
                volume_per_pack_ml: row.get(12)?,
                legacy_inventory_event_count: row.get(13)?,
                quantity_semantics: "unresolved_legacy_inventory_unit",
            })
        })
        .optional()
}

pub(super) fn list_movements(
    connection: &Connection,
    request: &InventoryMovementListRequest,
) -> rusqlite::Result<InventoryMovementListResponse> {
    let limit = request.limit.unwrap_or(100).clamp(1, 500) as i64;
    let offset = request.offset.unwrap_or(0) as i64;
    let total = connection.query_row(
        "SELECT COUNT(*) FROM inventory_movements WHERE drug_id=?1",
        [request.drug_id],
        |row| row.get::<_, u64>(0),
    )?;
    let mut statement = connection.prepare(
        "WITH history AS (
           SELECT m.*,
                  SUM(m.quantity_delta) OVER (
                    PARTITION BY m.drug_id ORDER BY m.id
                    ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                  ) AS resulting_balance
           FROM inventory_movements m
           WHERE m.drug_id=?1
         )
         SELECT h.id,h.movement_type,h.quantity_delta,h.resulting_balance,
                h.occurred_at,h.created_at,u.display_name,
                h.reference_type,h.reference_id,h.note,h.preparation_task_id
         FROM history h
         LEFT JOIN users u ON u.id=h.actor_user_id
         ORDER BY h.id DESC
         LIMIT ?2 OFFSET ?3",
    )?;
    let items = statement
        .query_map(params![request.drug_id, limit, offset], map_movement)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(InventoryMovementListResponse { items, total })
}

pub(super) fn get_movement(
    connection: &Connection,
    drug_id: i64,
    movement_id: i64,
) -> rusqlite::Result<Option<InventoryMovement>> {
    connection
        .query_row(
            "WITH history AS (
               SELECT m.*,
                      SUM(m.quantity_delta) OVER (
                        PARTITION BY m.drug_id ORDER BY m.id
                        ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                      ) AS resulting_balance
               FROM inventory_movements m
               WHERE m.drug_id=?1
             )
             SELECT h.id,h.movement_type,h.quantity_delta,h.resulting_balance,
                    h.occurred_at,h.created_at,u.display_name,
                    h.reference_type,h.reference_id,h.note,h.preparation_task_id
             FROM history h
             LEFT JOIN users u ON u.id=h.actor_user_id
             WHERE h.id=?2",
            params![drug_id, movement_id],
            map_movement,
        )
        .optional()
}

pub(super) fn drug_exists(transaction: &Transaction<'_>, drug_id: i64) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM drugs WHERE id=?1)",
        [drug_id],
        |row| row.get(0),
    )
}

pub(super) fn normalize_timestamp(
    transaction: &Transaction<'_>,
    value: Option<&str>,
) -> rusqlite::Result<Option<String>> {
    match value {
        Some(value) => {
            transaction.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ',?1)", [value], |row| {
                row.get(0)
            })
        }
        None => transaction.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now')", [], |row| {
            row.get(0)
        }),
    }
}

pub(crate) struct NewMovement<'a> {
    pub drug_id: i64,
    pub movement_type: InventoryMovementType,
    pub quantity_delta: f64,
    pub occurred_at: &'a str,
    pub actor_user_id: i64,
    pub reference_type: Option<&'a str>,
    pub reference_id: Option<&'a str>,
    pub note: Option<&'a str>,
    pub preparation_task_id: Option<i64>,
}

pub(crate) fn insert_movement(
    transaction: &Transaction<'_>,
    movement: &NewMovement<'_>,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO inventory_movements(
           drug_id,movement_type,quantity_delta,occurred_at,actor_user_id,
           reference_type,reference_id,note,preparation_task_id
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            movement.drug_id,
            movement.movement_type.as_database(),
            movement.quantity_delta,
            movement.occurred_at,
            movement.actor_user_id,
            movement.reference_type,
            movement.reference_id,
            movement.note,
            movement.preparation_task_id,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(crate) fn current_balance(
    connection: &Connection,
    drug_id: i64,
) -> rusqlite::Result<Option<f64>> {
    connection.query_row(
        "SELECT SUM(quantity_delta) FROM inventory_movements WHERE drug_id=?1",
        [drug_id],
        |row| row.get(0),
    )
}

fn map_inventory_summary(row: &Row<'_>) -> rusqlite::Result<InventorySummary> {
    let current_stock = row.get(5)?;
    let minimum_stock = row.get(6)?;
    let tracking_enabled = row.get::<_, i64>(8)? != 0;
    Ok(InventorySummary {
        drug_id: row.get(0)?,
        drug_code: row.get(1)?,
        drug_name: row.get(2)?,
        legacy_drug_unit: row.get(3)?,
        package: row.get(4)?,
        current_stock,
        minimum_stock,
        maximum_stock: row.get(7)?,
        tracking_enabled,
        stock_state: stock_state(tracking_enabled, current_stock, minimum_stock),
    })
}

fn map_movement(row: &Row<'_>) -> rusqlite::Result<InventoryMovement> {
    let movement_type = row.get::<_, String>(1)?;
    Ok(InventoryMovement {
        id: row.get(0)?,
        movement_type: InventoryMovementType::from_database(&movement_type)?,
        quantity_delta: row.get(2)?,
        resulting_balance: row.get(3)?,
        occurred_at: row.get(4)?,
        created_at: row.get(5)?,
        actor_display_name: row.get(6)?,
        reference_type: row.get(7)?,
        reference_id: row.get(8)?,
        note: row.get(9)?,
        preparation_task_id: row.get(10)?,
    })
}

pub(crate) fn stock_state(
    tracking_enabled: bool,
    current_stock: Option<f64>,
    minimum_stock: Option<f64>,
) -> StockState {
    if !tracking_enabled {
        return StockState::Untracked;
    }
    let Some(current_stock) = current_stock else {
        return StockState::Unknown;
    };
    if current_stock < 0.0 {
        return StockState::Shortage;
    }
    if current_stock == 0.0 {
        return StockState::Out;
    }
    if minimum_stock.is_some_and(|minimum| current_stock <= minimum) {
        return StockState::Low;
    }
    StockState::Normal
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
