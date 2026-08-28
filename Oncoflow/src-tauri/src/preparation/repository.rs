use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use super::{
    PreparationActor, PreparationInventoryPosting, PreparationInventoryPostingStatus,
    PreparationIssueStockState, PreparationQueueItem, PreparationQueueRequest,
    PreparationQueueResponse, PreparationQueueSourceFilter, PreparationState, PreparationTask,
    SafetyAcknowledgement, SourceSnapshot,
};
use crate::{auth::UserRole, safety::SafetyFinding};

pub(super) struct WorkspaceHeader {
    pub order_id: i64,
    pub order_code: String,
    pub patient_hn: String,
    pub patient_name: String,
    pub ward_name: Option<String>,
    pub regimen_name: Option<String>,
    pub treatment_time: Option<String>,
    pub assigned_preparer: Option<PreparationActor>,
    pub editable: bool,
    pub workflow_status: String,
}

pub(super) struct WorkspaceSourceItem {
    pub snapshot: SourceSnapshot,
    pub drug_code: String,
    pub drug_name: String,
    pub amount_per_container: Option<String>,
    pub presentation_unit: Option<String>,
    pub volume_per_container_ml: Option<String>,
    pub package_label: Option<String>,
    pub legacy_stored_quantity: Option<String>,
    pub inventory_tracking_enabled: bool,
    pub current_inventory: Option<String>,
    pub minimum_inventory: Option<String>,
    pub legacy_marker: bool,
}

pub(super) fn list_preparation_pharmacists(
    connection: &Connection,
) -> rusqlite::Result<Vec<PreparationActor>> {
    connection
        .prepare(
            "SELECT id,display_name,role
             FROM users
             WHERE active=1 AND credential_kind='argon2id' AND user_type='pharmacist'
             ORDER BY display_name COLLATE NOCASE,id",
        )?
        .query_map([], |row| {
            Ok(PreparationActor {
                id: row.get(0)?,
                display_name: row.get(1)?,
                role: UserRole::from_database(&row.get::<_, String>(2)?)?,
            })
        })?
        .collect()
}

pub(super) fn load_preparation_pharmacist(
    connection: &Connection,
    user_id: i64,
) -> rusqlite::Result<Option<PreparationActor>> {
    connection
        .query_row(
            "SELECT id,display_name,role
             FROM users
             WHERE id=?1 AND active=1 AND credential_kind='argon2id' AND user_type='pharmacist'",
            [user_id],
            |row| {
                Ok(PreparationActor {
                    id: row.get(0)?,
                    display_name: row.get(1)?,
                    role: UserRole::from_database(&row.get::<_, String>(2)?)?,
                })
            },
        )
        .optional()
}

pub(super) fn list_queue(
    connection: &Connection,
    request: &PreparationQueueRequest,
) -> rusqlite::Result<PreparationQueueResponse> {
    let pattern = format!(
        "%{}%",
        escape_like(request.search.as_deref().unwrap_or("").trim())
    );
    let limit = request.limit.unwrap_or(100).clamp(1, 200) as i64;
    let offset = request.offset.unwrap_or(0) as i64;
    let active_item = active_item_sql("i");
    let eligible_active_item = active_item_sql("eligible");
    let filters = format!(
        "o.oncoflow_created=1
        AND o.workflow_status='active'
        AND (?1='%%' OR o.legacy_orderid LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             OR p.legacy_hn LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             OR p.first_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             OR p.last_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             OR w.ward_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             OR r.regimen_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE)
        AND (?2 IS NULL OR substr(o.order_time,1,10)>=?2)
        AND (?3 IS NULL OR substr(o.order_time,1,10)<=?3)
        AND (?5='all'
             OR (?5='same_day' AND substr(o.order_time,1,10)=?4)
             OR (?5='continuing' AND substr(o.order_time,1,10)<?4)
             OR (?5='rescheduled' AND EXISTS(
                 SELECT 1 FROM order_status_events filter_event
                 WHERE filter_event.order_id=o.id
                   AND filter_event.event_type='rescheduled'
                   AND filter_event.effective_date=?4
             )))
        AND EXISTS(
            SELECT 1 FROM order_items eligible
            JOIN drugs eligible_drug ON eligible_drug.id=eligible.drug_id
            WHERE eligible.order_id=o.id AND eligible_drug.marker=1
              AND {eligible_active_item}
        )"
    );
    let total = connection.query_row(
        &format!(
            "SELECT COUNT(*) FROM orders o
             JOIN patients p ON p.id=o.patient_id
             LEFT JOIN wards w ON w.id=o.ward_id
             LEFT JOIN regimens r ON r.id=o.regimen_id
             WHERE {filters}"
        ),
        params![
            pattern,
            request.date_from,
            request.date_to,
            request.preparation_date,
            request.source_filter.as_database()
        ],
        |row| row.get::<_, u64>(0),
    )?;
    let sql = format!(
        "SELECT o.id,o.legacy_orderid,p.legacy_hn,
                trim(COALESCE(p.title,'') || ' ' || COALESCE(p.first_name,'') || ' ' || COALESCE(p.last_name,'')),
                w.ward_name,r.regimen_name,o.order_time,
                COALESCE(?4,substr(o.order_time,1,10)),
                CASE
                  WHEN EXISTS(
                    SELECT 1 FROM order_status_events source_event
                    WHERE source_event.order_id=o.id
                      AND source_event.event_type='rescheduled'
                      AND source_event.effective_date=?4
                  ) THEN 'rescheduled'
                  WHEN substr(o.order_time,1,10)=?4 THEN 'same_day'
                  ELSE 'continuing'
                END,
                COUNT(i.id),COUNT(t.id),
                COALESCE(SUM(CASE WHEN t.state='pending' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN t.state='prepared' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN t.state='verified' THEN 1 ELSE 0 END),0),
                COUNT(DISTINCT CASE WHEN EXISTS(
                  SELECT 1 FROM audit_events print_event
                  WHERE print_event.entity_type='preparation_task'
                    AND print_event.entity_id=CAST(t.id AS TEXT)
                    AND print_event.event_type IN (
                      'preparation_label_print_requested',
                      'preparation_label_reprint_requested'
                    )
                ) THEN t.id END)
         FROM orders o
         JOIN patients p ON p.id=o.patient_id
         LEFT JOIN wards w ON w.id=o.ward_id
         LEFT JOIN regimens r ON r.id=o.regimen_id
         JOIN order_items i ON i.order_id=o.id AND {active_item}
         JOIN drugs drug ON drug.id=i.drug_id AND drug.marker=1
         LEFT JOIN preparation_tasks t ON t.source_order_item_id=i.id
              AND (?4 IS NULL OR t.preparation_date=?4)
         WHERE {filters}
         GROUP BY o.id
         ORDER BY CASE WHEN o.order_time IS NULL THEN 1 ELSE 0 END,o.order_time ASC,o.id ASC
         LIMIT ?6 OFFSET ?7"
    );
    let items = connection
        .prepare(&sql)?
        .query_map(
            params![
                pattern,
                request.date_from,
                request.date_to,
                request.preparation_date,
                request.source_filter.as_database(),
                limit,
                offset
            ],
            |row| {
                let source_kind = match row.get::<_, String>(8)?.as_str() {
                    "same_day" => PreparationQueueSourceFilter::SameDay,
                    "rescheduled" => PreparationQueueSourceFilter::Rescheduled,
                    _ => PreparationQueueSourceFilter::Continuing,
                };
                Ok(PreparationQueueItem {
                    order_id: row.get(0)?,
                    order_code: row.get(1)?,
                    patient_hn: row.get(2)?,
                    patient_name: row.get(3)?,
                    ward_name: row.get(4)?,
                    regimen_name: row.get(5)?,
                    treatment_time: row.get(6)?,
                    preparation_date: row.get(7)?,
                    source_kind,
                    eligible_item_count: row.get(9)?,
                    initialized_item_count: row.get(10)?,
                    pending_item_count: row.get(11)?,
                    prepared_item_count: row.get(12)?,
                    verified_item_count: row.get(13)?,
                    printed_label_count: row.get(14)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PreparationQueueResponse { items, total })
}

pub(super) fn load_header(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<Option<WorkspaceHeader>> {
    connection
        .query_row(
            "SELECT o.id,o.legacy_orderid,p.legacy_hn,
                    trim(COALESCE(p.title,'') || ' ' || COALESCE(p.first_name,'') || ' ' || COALESCE(p.last_name,'')),
                    w.ward_name,r.regimen_name,o.order_time,o.oncoflow_created,o.workflow_status,
                    preparer.id,COALESCE(preparer.display_name,preparer.username),preparer.role
             FROM orders o
             JOIN patients p ON p.id=o.patient_id
             LEFT JOIN wards w ON w.id=o.ward_id
             LEFT JOIN regimens r ON r.id=o.regimen_id
             LEFT JOIN users preparer ON preparer.id=o.assigned_preparer_user_id
             WHERE o.id=?1",
            [order_id],
            |row| {
                Ok(WorkspaceHeader {
                    order_id: row.get(0)?,
                    order_code: row.get(1)?,
                    patient_hn: row.get(2)?,
                    patient_name: row.get(3)?,
                    ward_name: row.get(4)?,
                    regimen_name: row.get(5)?,
                    treatment_time: row.get(6)?,
                    editable: row.get::<_, i64>(7)? != 0,
                    workflow_status: row.get(8)?,
                    assigned_preparer: map_actor(row, 9)?,
                })
            },
        )
        .optional()
}

pub(super) fn is_no_show_date(
    connection: &Connection,
    order_id: i64,
    preparation_date: &str,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM order_status_events
             WHERE order_id=?1 AND event_type='no_show' AND effective_date=?2
         )",
        params![order_id, preparation_date],
        |row| row.get(0),
    )
}

pub(super) fn rescheduled_source_date(
    connection: &Connection,
    order_id: i64,
    preparation_date: &str,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT related_date FROM order_status_events
             WHERE order_id=?1 AND event_type='rescheduled' AND effective_date=?2
             ORDER BY id DESC LIMIT 1",
            params![order_id, preparation_date],
            |row| row.get(0),
        )
        .optional()
}

pub(super) fn is_suspended_date(
    connection: &Connection,
    order_id: i64,
    preparation_date: &str,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
             SELECT 1
             FROM order_status_events held
             LEFT JOIN order_status_events resumed
               ON resumed.order_id=held.order_id
              AND resumed.event_type='rescheduled'
              AND resumed.related_date=held.effective_date
             WHERE held.order_id=?1
               AND held.event_type='no_show'
               AND held.effective_date<=?2
               AND (resumed.id IS NULL OR ?2<resumed.effective_date)
         )",
        params![order_id, preparation_date],
        |row| row.get(0),
    )
}

fn active_item_sql(alias: &str) -> String {
    let normal_due = due_on_date_sql(alias, "?4");
    let rescheduled_due = due_on_date_sql(alias, "source_event.related_date");
    format!(
        "(
          ?4 IS NULL OR (
            (
              ({normal_due})
              AND NOT EXISTS(
                SELECT 1 FROM order_status_events missed
                WHERE missed.order_id=o.id AND missed.event_type='no_show'
                  AND missed.effective_date=?4
              )
              AND NOT EXISTS(
                SELECT 1
                FROM order_status_events held
                LEFT JOIN order_status_events resumed
                  ON resumed.order_id=held.order_id
                 AND resumed.event_type='rescheduled'
                 AND resumed.related_date=held.effective_date
                WHERE held.order_id=o.id
                  AND held.event_type='no_show'
                  AND held.effective_date<=?4
                  AND (resumed.id IS NULL OR ?4<resumed.effective_date)
              )
            )
            OR EXISTS(
              SELECT 1 FROM order_status_events source_event
              WHERE source_event.order_id=o.id
                AND source_event.event_type='rescheduled'
                AND source_event.effective_date=?4
                AND ({rescheduled_due})
            )
          )
        )"
    )
}

fn due_on_date_sql(alias: &str, date_expression: &str) -> String {
    format!(
        "(
          ({alias}.start_date IS NOT NULL AND {alias}.stop_date IS NOT NULL
           AND substr({alias}.start_date,1,10)<={date_expression}
           AND substr({alias}.stop_date,1,10)>={date_expression})
          OR ({alias}.start_date IS NOT NULL AND {alias}.stop_date IS NULL
              AND substr({alias}.start_date,1,10)={date_expression})
          OR ({alias}.start_date IS NULL AND {alias}.stop_date IS NOT NULL
              AND substr({alias}.stop_date,1,10)={date_expression})
          OR ({alias}.start_date IS NULL AND {alias}.stop_date IS NULL
              AND substr(o.order_time,1,10)={date_expression})
        )"
    )
}

pub(super) fn load_source_items(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<Vec<WorkspaceSourceItem>> {
    connection
        .prepare(
            "SELECT i.order_id,i.id,i.drug_id,
                    COALESCE(i.legacy_dose_text,CASE WHEN i.dose IS NULL THEN NULL ELSE CAST(i.dose AS TEXT) END),
                    COALESCE(i.regimen_unit_text,u.unit_name),
                    i.diluent_id,dl.diluent_name,COALESCE(i.diluent_volume_ml,dl.volume_ml),
                    i.route_id,rt.route_name,i.rate,
                    COALESCE(CAST(i.regimen_start_day AS TEXT),i.start_date),
                    i.start_date,i.stop_date,i.ordering_no,
                    i.regimen_details,d.detail,d.storage,
                    d.legacy_dcode,d.drug_name,
                    CAST(d.dose_per_pack AS TEXT),u.unit_name,
                    CAST(d.volume_per_pack_ml AS TEXT),d.package,
                    CAST(i.number_of_drug AS TEXT),d.inventory_enabled,
                    CAST((SELECT SUM(m.quantity_delta) FROM inventory_movements m WHERE m.drug_id=d.id) AS TEXT),
                    CAST(d.inventory_min AS TEXT),d.marker
             FROM order_items i
             JOIN drugs d ON d.id=i.drug_id
             LEFT JOIN units u ON u.id=d.unit_id
             LEFT JOIN diluents dl ON dl.id=i.diluent_id
             LEFT JOIN routes rt ON rt.id=i.route_id
             WHERE i.order_id=?1
             ORDER BY CASE WHEN i.ordering_no IS NULL THEN 1 ELSE 0 END,i.ordering_no,i.id",
        )?
        .query_map([order_id], |row| {
            Ok(WorkspaceSourceItem {
                snapshot: map_source_snapshot(row)?,
                drug_code: row.get(18)?,
                drug_name: row.get(19)?,
                amount_per_container: row.get(20)?,
                presentation_unit: row.get(21)?,
                volume_per_container_ml: row.get(22)?,
                package_label: row.get(23)?,
                legacy_stored_quantity: row.get(24)?,
                inventory_tracking_enabled: row.get::<_, i64>(25)? != 0,
                current_inventory: row.get(26)?,
                minimum_inventory: row.get(27)?,
                legacy_marker: row.get::<_, i64>(28)? != 0,
            })
        })?
        .collect()
}

pub(super) fn load_source_snapshot(
    connection: &Connection,
    order_item_id: i64,
) -> rusqlite::Result<Option<SourceSnapshot>> {
    connection
        .query_row(
            "SELECT i.order_id,i.id,i.drug_id,
                    COALESCE(i.legacy_dose_text,CASE WHEN i.dose IS NULL THEN NULL ELSE CAST(i.dose AS TEXT) END),
                    COALESCE(i.regimen_unit_text,u.unit_name),
                    i.diluent_id,dl.diluent_name,COALESCE(i.diluent_volume_ml,dl.volume_ml),
                    i.route_id,rt.route_name,i.rate,
                    COALESCE(CAST(i.regimen_start_day AS TEXT),i.start_date),
                    i.start_date,i.stop_date,i.ordering_no,
                    i.regimen_details,d.detail,d.storage
             FROM order_items i
             JOIN drugs d ON d.id=i.drug_id
             LEFT JOIN units u ON u.id=d.unit_id
             LEFT JOIN diluents dl ON dl.id=i.diluent_id
             LEFT JOIN routes rt ON rt.id=i.route_id
             WHERE i.id=?1",
            [order_item_id],
            map_source_snapshot,
        )
        .optional()
}

fn map_source_snapshot(row: &Row<'_>) -> rusqlite::Result<SourceSnapshot> {
    Ok(SourceSnapshot {
        source_order_id: row.get(0)?,
        source_order_item_id: row.get(1)?,
        drug_id: row.get(2)?,
        ordered_dose_text: row.get(3)?,
        dose_unit_text: row.get(4)?,
        diluent_id: row.get(5)?,
        diluent_name: row.get(6)?,
        diluent_volume_ml: row.get(7)?,
        route_id: row.get(8)?,
        route_name: row.get(9)?,
        rate_text: row.get(10)?,
        treatment_day: row.get(11)?,
        start_date: row.get(12)?,
        stop_date: row.get(13)?,
        sequence_no: row.get(14)?,
        regimen_details: row.get(15)?,
        drug_detail: row.get(16)?,
        drug_storage: row.get(17)?,
    })
}

pub(super) fn insert_task(
    transaction: &Transaction<'_>,
    snapshot: &SourceSnapshot,
    preparation_date: &str,
) -> rusqlite::Result<Option<i64>> {
    let inserted = transaction.execute(
        "INSERT OR IGNORE INTO preparation_tasks(
            source_order_id,source_order_item_id,preparation_date,drug_id,
            snapshot_ordered_dose_text,snapshot_dose_unit_text,
            snapshot_diluent_id,snapshot_diluent_name,snapshot_diluent_volume_ml,
            snapshot_route_id,snapshot_route_name,snapshot_rate_text,
            snapshot_treatment_day,snapshot_start_date,snapshot_stop_date,
            snapshot_sequence_no,snapshot_regimen_details,
            snapshot_drug_detail,snapshot_drug_storage
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
        params![
            snapshot.source_order_id,
            snapshot.source_order_item_id,
            preparation_date,
            snapshot.drug_id,
            snapshot.ordered_dose_text,
            snapshot.dose_unit_text,
            snapshot.diluent_id,
            snapshot.diluent_name,
            snapshot.diluent_volume_ml,
            snapshot.route_id,
            snapshot.route_name,
            snapshot.rate_text,
            snapshot.treatment_day,
            snapshot.start_date,
            snapshot.stop_date,
            snapshot.sequence_no,
            snapshot.regimen_details,
            snapshot.drug_detail,
            snapshot.drug_storage,
        ],
    )?;
    Ok((inserted == 1).then(|| transaction.last_insert_rowid()))
}

pub(super) fn refresh_pending_task_snapshot(
    transaction: &Transaction<'_>,
    task_id: i64,
    snapshot: &SourceSnapshot,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE preparation_tasks
         SET drug_id=?1,
             snapshot_ordered_dose_text=?2,snapshot_dose_unit_text=?3,
             snapshot_diluent_id=?4,snapshot_diluent_name=?5,snapshot_diluent_volume_ml=?6,
             snapshot_route_id=?7,snapshot_route_name=?8,snapshot_rate_text=?9,
             snapshot_treatment_day=?10,snapshot_start_date=?11,snapshot_stop_date=?12,
             snapshot_sequence_no=?13,snapshot_regimen_details=?14,
             snapshot_drug_detail=?15,snapshot_drug_storage=?16,
             preparation_volume_ml=NULL,preparation_notes=NULL,
             withdrawal_volume_ml=NULL,
             final_container_count=1,updated_at=CURRENT_TIMESTAMP
         WHERE id=?17 AND source_order_id=?18 AND source_order_item_id=?19 AND state='pending'",
        params![
            snapshot.drug_id,
            snapshot.ordered_dose_text,
            snapshot.dose_unit_text,
            snapshot.diluent_id,
            snapshot.diluent_name,
            snapshot.diluent_volume_ml,
            snapshot.route_id,
            snapshot.route_name,
            snapshot.rate_text,
            snapshot.treatment_day,
            snapshot.start_date,
            snapshot.stop_date,
            snapshot.sequence_no,
            snapshot.regimen_details,
            snapshot.drug_detail,
            snapshot.drug_storage,
            task_id,
            snapshot.source_order_id,
            snapshot.source_order_item_id,
        ],
    )
}

pub(super) fn load_task_for_item_on_date(
    connection: &Connection,
    order_item_id: i64,
    preparation_date: &str,
) -> rusqlite::Result<Option<PreparationTask>> {
    connection
        .query_row(
            &task_select("WHERE t.source_order_item_id=?1 AND t.preparation_date=?2"),
            params![order_item_id, preparation_date],
            map_task,
        )
        .optional()
}

pub(super) fn load_task(
    connection: &Connection,
    task_id: i64,
) -> rusqlite::Result<Option<PreparationTask>> {
    connection
        .query_row(&task_select("WHERE t.id=?1"), [task_id], map_task)
        .optional()
}

pub(super) fn load_tasks_for_order_on_date(
    connection: &Connection,
    order_id: i64,
    preparation_date: &str,
) -> rusqlite::Result<Vec<PreparationTask>> {
    connection
        .prepare(&task_select(
            "WHERE t.source_order_id=?1 AND t.preparation_date=?2 ORDER BY t.id",
        ))?
        .query_map(params![order_id, preparation_date], map_task)?
        .collect()
}

fn task_select(filter: &str) -> String {
    format!(
        "SELECT t.id,t.source_order_id,t.source_order_item_id,t.preparation_date,t.drug_id,t.state,
                t.snapshot_ordered_dose_text,t.snapshot_dose_unit_text,
                t.snapshot_diluent_id,t.snapshot_diluent_name,t.snapshot_diluent_volume_ml,
                t.snapshot_route_id,t.snapshot_route_name,t.snapshot_rate_text,
                t.snapshot_treatment_day,t.snapshot_start_date,t.snapshot_stop_date,
                t.snapshot_sequence_no,t.snapshot_regimen_details,
                t.snapshot_drug_detail,t.snapshot_drug_storage,
                t.preparation_volume_ml,t.preparation_notes,t.created_at,t.updated_at,
                t.prepared_at,t.verified_at,
                pu.id,COALESCE(pu.display_name,pu.username),pu.role,
                vu.id,COALESCE(vu.display_name,vu.username),vu.role,
                ip.id,ip.status,ip.inventory_movement_id,
                CAST(ip.containers_required AS TEXT),
                CAST(ip.balance_before AS TEXT),CAST(ip.balance_after AS TEXT),
                ip.resulting_stock_state,ip.calculation_status,
                ip.calculation_ruleset_version,ip.calculation_rule_id,
                ip.workflow_rule_id,ip.reason_code,im.occurred_at,ip.created_at,
                iu.id,COALESCE(iu.display_name,iu.username),iu.role,
                t.final_container_count
         FROM preparation_tasks t
         LEFT JOIN users pu ON pu.id=t.prepared_by_user_id
         LEFT JOIN users vu ON vu.id=t.verified_by_user_id
         LEFT JOIN preparation_inventory_postings ip ON ip.preparation_task_id=t.id
         LEFT JOIN inventory_movements im ON im.id=ip.inventory_movement_id
         LEFT JOIN users iu ON iu.id=ip.actor_user_id
         {filter}"
    )
}

fn map_task(row: &Row<'_>) -> rusqlite::Result<PreparationTask> {
    let state = row.get::<_, String>(5)?;
    let ordered_dose_text = row.get::<_, Option<String>>(6)?;
    let preparation_volume_ml = row.get::<_, Option<f64>>(21)?;
    let final_container_count = u32::try_from(row.get::<_, i64>(50)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            50,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })?;
    Ok(PreparationTask {
        id: row.get(0)?,
        source_order_id: row.get(1)?,
        source_order_item_id: row.get(2)?,
        preparation_date: row.get(3)?,
        drug_id: row.get(4)?,
        state: PreparationState::from_database(&state)?,
        ordered_dose_text,
        dose_unit_text: row.get(7)?,
        diluent_id: row.get(8)?,
        diluent_name: row.get(9)?,
        diluent_volume_ml: row.get(10)?,
        route_id: row.get(11)?,
        route_name: row.get(12)?,
        rate_text: row.get(13)?,
        treatment_day: row.get(14)?,
        start_date: row.get(15)?,
        stop_date: row.get(16)?,
        sequence_no: row.get(17)?,
        regimen_details: row.get(18)?,
        drug_detail: row.get(19)?,
        drug_storage: row.get(20)?,
        preparation_volume_ml,
        preparation_notes: row.get(22)?,
        final_container_count,
        created_at: row.get(23)?,
        updated_at: row.get(24)?,
        prepared_at: row.get(25)?,
        verified_at: row.get(26)?,
        prepared_by: map_actor(row, 27)?,
        verified_by: map_actor(row, 30)?,
        inventory_posting: map_inventory_posting(row)?,
    })
}

fn map_inventory_posting(row: &Row<'_>) -> rusqlite::Result<Option<PreparationInventoryPosting>> {
    let Some(id) = row.get::<_, Option<i64>>(33)? else {
        return Ok(None);
    };
    let status = PreparationInventoryPostingStatus::from_database(&row.get::<_, String>(34)?)?;
    let resulting_stock_state = row
        .get::<_, Option<String>>(39)?
        .map(|value| PreparationIssueStockState::from_database(&value))
        .transpose()?;
    let actor = map_actor(row, 47)?.ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            47,
            rusqlite::types::Type::Null,
            "inventory posting is missing its actor".into(),
        )
    })?;
    Ok(Some(PreparationInventoryPosting {
        id,
        status,
        inventory_movement_id: row.get(35)?,
        containers_required: row.get(36)?,
        balance_before: row.get(37)?,
        balance_after: row.get(38)?,
        resulting_stock_state,
        calculation_status: row.get(40)?,
        calculation_ruleset_version: row.get(41)?,
        calculation_rule_id: row.get(42)?,
        workflow_rule_id: row.get(43)?,
        reason_code: row.get(44)?,
        issued_at: row.get(45)?,
        recorded_at: row.get(46)?,
        actor,
    }))
}

fn map_actor(row: &Row<'_>, start: usize) -> rusqlite::Result<Option<PreparationActor>> {
    let id = row.get::<_, Option<i64>>(start)?;
    let Some(id) = id else {
        return Ok(None);
    };
    let role = row.get::<_, String>(start + 2)?;
    Ok(Some(PreparationActor {
        id,
        display_name: row.get(start + 1)?,
        role: UserRole::from_database(&role)?,
    }))
}

pub(super) fn update_task(
    transaction: &Transaction<'_>,
    task_id: i64,
    preparation_volume_ml: Option<f64>,
    notes: Option<&str>,
    withdrawal_volume_ml: Option<&str>,
    final_container_count: u32,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE preparation_tasks
         SET preparation_volume_ml=?1,preparation_notes=?2,
             withdrawal_volume_ml=?3,final_container_count=?4,
             updated_at=CURRENT_TIMESTAMP
         WHERE id=?5 AND state IN ('pending','prepared')",
        params![
            preparation_volume_ml,
            notes,
            withdrawal_volume_ml,
            final_container_count,
            task_id
        ],
    )
}

pub(super) fn mark_prepared(
    transaction: &Transaction<'_>,
    task_id: i64,
    user_id: i64,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE preparation_tasks
         SET state='prepared',prepared_at=CURRENT_TIMESTAMP,prepared_by_user_id=?2,
             verified_at=NULL,verified_by_user_id=NULL,updated_at=CURRENT_TIMESTAMP
         WHERE id=?1 AND state='pending'",
        params![task_id, user_id],
    )
}

pub(super) fn mark_verified(
    transaction: &Transaction<'_>,
    task_id: i64,
    user_id: i64,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE preparation_tasks
         SET state='verified',verified_at=CURRENT_TIMESTAMP,verified_by_user_id=?2,
             updated_at=CURRENT_TIMESTAMP
         WHERE id=?1 AND state='prepared'",
        params![task_id, user_id],
    )
}

pub(super) fn mark_checked(
    transaction: &Transaction<'_>,
    task_id: i64,
    prepared_by_user_id: i64,
    checked_by_user_id: i64,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE preparation_tasks
         SET state='verified',
             prepared_at=COALESCE(prepared_at,CURRENT_TIMESTAMP),
             prepared_by_user_id=COALESCE(prepared_by_user_id,?2),
             verified_at=CURRENT_TIMESTAMP,verified_by_user_id=?3,
             updated_at=CURRENT_TIMESTAMP
         WHERE id=?1 AND state IN ('pending','prepared')",
        params![task_id, prepared_by_user_id, checked_by_user_id],
    )
}

pub(super) struct NewPreparationInventoryPosting<'a> {
    pub preparation_task_id: i64,
    pub status: PreparationInventoryPostingStatus,
    pub inventory_movement_id: Option<i64>,
    pub containers_required: Option<i64>,
    pub balance_before: Option<f64>,
    pub balance_after: Option<f64>,
    pub resulting_stock_state: Option<PreparationIssueStockState>,
    pub calculation_status: &'a str,
    pub calculation_ruleset_version: &'a str,
    pub calculation_rule_id: &'a str,
    pub workflow_rule_id: &'a str,
    pub reason_code: &'a str,
    pub actor_user_id: i64,
}

pub(super) fn insert_inventory_posting(
    transaction: &Transaction<'_>,
    posting: &NewPreparationInventoryPosting<'_>,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO preparation_inventory_postings(
            preparation_task_id,status,inventory_movement_id,containers_required,
            balance_before,balance_after,resulting_stock_state,calculation_status,
            calculation_ruleset_version,calculation_rule_id,workflow_rule_id,
            reason_code,actor_user_id
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            posting.preparation_task_id,
            posting.status.as_database(),
            posting.inventory_movement_id,
            posting.containers_required,
            posting.balance_before,
            posting.balance_after,
            posting
                .resulting_stock_state
                .map(PreparationIssueStockState::as_database),
            posting.calculation_status,
            posting.calculation_ruleset_version,
            posting.calculation_rule_id,
            posting.workflow_rule_id,
            posting.reason_code,
            posting.actor_user_id,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn insert_acknowledgement(
    transaction: &Transaction<'_>,
    order_id: i64,
    task: Option<&PreparationTask>,
    finding: &SafetyFinding,
    user_id: i64,
) -> rusqlite::Result<Option<i64>> {
    let inserted = transaction.execute(
        "INSERT OR IGNORE INTO safety_acknowledgements(
            order_id,preparation_task_id,order_item_id,finding_id,finding_fingerprint,
            rule_id,ruleset_version,user_id,source_snapshot_stale
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,0)",
        params![
            order_id,
            task.map(|value| value.id),
            finding.order_item_id,
            finding.id,
            finding.fingerprint,
            finding.rule_id,
            finding.ruleset_version,
            user_id,
        ],
    )?;
    Ok((inserted == 1).then(|| transaction.last_insert_rowid()))
}

pub(super) fn load_acknowledgements(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<Vec<SafetyAcknowledgement>> {
    connection
        .prepare(
            "SELECT a.preparation_task_id,a.order_item_id,a.finding_id,
                    a.finding_fingerprint,a.rule_id,a.ruleset_version,
                    u.id,COALESCE(u.display_name,u.username),u.role,
                    a.acknowledged_at,a.source_snapshot_stale
             FROM safety_acknowledgements a
             JOIN users u ON u.id=a.user_id
             WHERE a.order_id=?1
             ORDER BY a.acknowledged_at,a.id",
        )?
        .query_map([order_id], |row| {
            let role = row.get::<_, String>(8)?;
            Ok(SafetyAcknowledgement {
                preparation_task_id: row.get(0)?,
                order_item_id: row.get(1)?,
                finding_id: row.get(2)?,
                finding_fingerprint: row.get(3)?,
                rule_id: row.get(4)?,
                ruleset_version: row.get(5)?,
                user: PreparationActor {
                    id: row.get(6)?,
                    display_name: row.get(7)?,
                    role: UserRole::from_database(&role)?,
                },
                acknowledged_at: row.get(9)?,
                source_snapshot_stale: row.get::<_, i64>(10)? != 0,
            })
        })?
        .collect()
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
