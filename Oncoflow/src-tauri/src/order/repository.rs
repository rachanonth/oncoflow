use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use super::{
    DiluentOrderLookupOption, NormalizedOrderItemInput, OrderDetail, OrderInput, OrderItemDetail,
    OrderListRequest, OrderListResponse, OrderLookupOption, OrderLookups, OrderSortField,
    OrderStatusEvent, OrderSummary, OrderSummaryDrug, OrderWorkflowStatus,
    PatientOrderLookupOption, SortDirection,
};

pub(super) struct NewOrderStatusEvent<'a> {
    pub order_id: i64,
    pub event_type: &'a str,
    pub from_status: OrderWorkflowStatus,
    pub to_status: OrderWorkflowStatus,
    pub effective_date: &'a str,
    pub related_date: Option<&'a str>,
    pub actor_user_id: i64,
}

pub(super) fn list_orders(
    connection: &Connection,
    request: &OrderListRequest,
) -> rusqlite::Result<OrderListResponse> {
    let search = request.search.as_deref().unwrap_or("").trim();
    let pattern = format!("%{}%", escape_like(search));
    let limit = request.limit.unwrap_or(100).clamp(1, 200) as i64;
    let offset = request.offset.unwrap_or(0) as i64;
    let direction = match request.sort_direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    let order = match request.sort_by {
        OrderSortField::Date => format!("o.order_time {direction}, o.id {direction}"),
        OrderSortField::OrderId => format!("o.legacy_orderid {direction}, o.id {direction}"),
        OrderSortField::Patient => format!(
            "p.last_name {direction}, p.first_name {direction}, p.legacy_hn {direction}, o.id DESC"
        ),
    };
    let filters = "(?1 = '%%' OR o.legacy_orderid LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                       OR p.legacy_hn LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                       OR p.first_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                       OR p.last_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                       OR r.regimen_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE)
                   AND (?2 IS NULL OR o.patient_id = ?2)
                   AND (?3 IS NULL OR substr(o.order_time,1,10) >= ?3)
                   AND (?4 IS NULL OR substr(o.order_time,1,10) <= ?4)";
    let values = params![
        pattern,
        request.patient_id,
        request.date_from,
        request.date_to
    ];
    let total = connection.query_row(
        &format!(
            "SELECT COUNT(*) FROM orders o
             JOIN patients p ON p.id=o.patient_id
             LEFT JOIN regimens r ON r.id=o.regimen_id
             WHERE {filters}"
        ),
        values,
        |row| row.get::<_, u64>(0),
    )?;
    let sql = format!(
        "SELECT o.id, o.legacy_orderid, o.patient_id, p.legacy_hn,
                trim(COALESCE(p.title,'') || ' ' || COALESCE(p.first_name,'') || ' ' || COALESCE(p.last_name,'')),
                o.order_time, r.regimen_name, d.doctor_name, w.ward_name,
                o.order_type, COUNT(i.id), o.oncoflow_created,o.workflow_status,
                COALESCE((
                    SELECT json_group_array(json_object(
                        'drugName',drug_name,
                        'doseText',dose_text,
                        'unitText',unit_text
                    ))
                    FROM (
                        SELECT history_drug.drug_name,
                               COALESCE(history_item.legacy_dose_text,
                                   CASE WHEN history_item.dose IS NULL THEN NULL ELSE CAST(history_item.dose AS TEXT) END) AS dose_text,
                               history_item.regimen_unit_text AS unit_text
                        FROM order_items history_item
                        JOIN drugs history_drug ON history_drug.id=history_item.drug_id
                        WHERE history_item.order_id=o.id
                        ORDER BY COALESCE(history_item.ordering_no,history_item.id),history_item.id
                    )
                ),'[]')
         FROM orders o
         JOIN patients p ON p.id=o.patient_id
         LEFT JOIN regimens r ON r.id=o.regimen_id
         LEFT JOIN doctors d ON d.id=o.doctor_id
         LEFT JOIN wards w ON w.id=o.ward_id
         LEFT JOIN order_items i ON i.order_id=o.id
         WHERE {filters}
         GROUP BY o.id
         ORDER BY {order}
         LIMIT ?5 OFFSET ?6"
    );
    let items = connection
        .prepare(&sql)?
        .query_map(
            params![
                pattern,
                request.patient_id,
                request.date_from,
                request.date_to,
                limit,
                offset
            ],
            map_order_summary,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OrderListResponse { items, total })
}

pub(super) fn get_order(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<Option<OrderDetail>> {
    let detail = connection
        .query_row(
            "SELECT o.id, o.legacy_orderid, o.patient_id, p.legacy_hn,
                    trim(COALESCE(p.title,'') || ' ' || COALESCE(p.first_name,'') || ' ' || COALESCE(p.last_name,'')),
                    o.ward_id, w.ward_name, o.doctor_id, d.doctor_name,
                    o.regimen_id, r.regimen_name, o.note, o.order_time, o.order_type,
                    o.appointment_flag, o.legacy_worker, o.edit_worker,
                    o.side_effect_text, o.side_effect_recorder, o.side_effect_record_time,
                    o.medication_error_text, o.oncoflow_created,
                    o.weight_kg, o.height_cm,o.assigned_preparer_user_id,
                    COALESCE(preparer.display_name,preparer.username),
                    o.workflow_status,o.workflow_status_reason,o.workflow_status_changed_at,
                    COALESCE(status_actor.display_name,status_actor.username)
             FROM orders o
             JOIN patients p ON p.id=o.patient_id
             LEFT JOIN wards w ON w.id=o.ward_id
             LEFT JOIN doctors d ON d.id=o.doctor_id
             LEFT JOIN regimens r ON r.id=o.regimen_id
             LEFT JOIN users preparer ON preparer.id=o.assigned_preparer_user_id
             LEFT JOIN users status_actor ON status_actor.id=o.workflow_status_changed_by_user_id
             WHERE o.id=?1",
            [order_id],
            |row| {
                Ok(OrderDetail {
                    id: row.get(0)?,
                    order_id: row.get(1)?,
                    patient_id: row.get(2)?,
                    patient_hn: row.get(3)?,
                    patient_name: row.get(4)?,
                    ward_id: row.get(5)?,
                    ward_name: row.get(6)?,
                    doctor_id: row.get(7)?,
                    doctor_name: row.get(8)?,
                    regimen_id: row.get(9)?,
                    regimen_name: row.get(10)?,
                    note: row.get(11)?,
                    order_time: row.get(12)?,
                    order_type: row.get(13)?,
                    appointment_flag: row.get::<_, i64>(14)? != 0,
                    legacy_worker: row.get(15)?,
                    edit_worker: row.get(16)?,
                    side_effect_text: row.get(17)?,
                    side_effect_recorder: row.get(18)?,
                    side_effect_record_time: row.get(19)?,
                    medication_error_text: row.get(20)?,
                    editable: row.get::<_, i64>(21)? != 0,
                    weight_kg: row.get(22)?,
                    height_cm: row.get(23)?,
                    assigned_preparer_user_id: row.get(24)?,
                    assigned_preparer_name: row.get(25)?,
                    workflow_status: OrderWorkflowStatus::from_database(
                        &row.get::<_, String>(26)?,
                    )?,
                    workflow_status_reason: row.get(27)?,
                    workflow_status_changed_at: row.get(28)?,
                    workflow_status_changed_by: row.get(29)?,
                    status_events: Vec::new(),
                    cumulative_doses: Vec::new(),
                    items: Vec::new(),
                })
            },
        )
        .optional()?;
    let Some(mut detail) = detail else {
        return Ok(None);
    };
    detail.items = connection
        .prepare(
            "SELECT i.id, i.drug_id, d.drug_name, i.diluent_id, dl.diluent_name,
                    i.diluent_volume_ml, i.route_id, r.route_name, i.start_date, i.stop_date, i.dose,
                    COALESCE(i.legacy_dose_text, CASE WHEN i.dose IS NULL THEN NULL ELSE CAST(i.dose AS TEXT) END),
                    i.schedule_time, i.number_of_drug, i.missing, i.printed, i.rate,
                    i.ordering_no, i.running_no, i.running_sum, i.inventory_date,
                    i.source_regimen_item_id, i.regimen_dose_text, i.regimen_unit_text,
                    i.regimen_route_text, i.regimen_details, i.regimen_item_group,
                    i.regimen_duration, i.regimen_start_day, i.regimen_ordering_no
             FROM order_items i
             JOIN drugs d ON d.id=i.drug_id
             LEFT JOIN diluents dl ON dl.id=i.diluent_id
             LEFT JOIN routes r ON r.id=i.route_id
             WHERE i.order_id=?1
             ORDER BY CASE WHEN i.ordering_no IS NULL THEN 1 ELSE 0 END, i.ordering_no, i.id",
        )?
        .query_map([order_id], map_order_item)?
        .collect::<Result<Vec<_>, _>>()?;
    detail.status_events = load_status_events(connection, order_id)?;
    detail.cumulative_doses =
        crate::safety::cumulative_dose_summaries(connection, detail.patient_id)?;
    Ok(Some(detail))
}

pub(super) fn load_status_events(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<Vec<OrderStatusEvent>> {
    connection
        .prepare(
            "SELECT e.id,e.event_type,e.from_status,e.to_status,e.effective_date,
                    e.related_date,COALESCE(u.display_name,u.username),e.occurred_at
             FROM order_status_events e
             JOIN users u ON u.id=e.actor_user_id
             WHERE e.order_id=?1
             ORDER BY e.id DESC",
        )?
        .query_map([order_id], |row| {
            Ok(OrderStatusEvent {
                id: row.get(0)?,
                event_type: row.get(1)?,
                from_status: OrderWorkflowStatus::from_database(&row.get::<_, String>(2)?)?,
                to_status: OrderWorkflowStatus::from_database(&row.get::<_, String>(3)?)?,
                effective_date: row.get(4)?,
                related_date: row.get(5)?,
                actor_display_name: row.get(6)?,
                occurred_at: row.get(7)?,
            })
        })?
        .collect()
}

pub(super) fn load_workflow_status(
    transaction: &Transaction<'_>,
    order_id: i64,
) -> rusqlite::Result<Option<(bool, OrderWorkflowStatus)>> {
    transaction
        .query_row(
            "SELECT oncoflow_created,workflow_status FROM orders WHERE id=?1",
            [order_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? != 0,
                    OrderWorkflowStatus::from_database(&row.get::<_, String>(1)?)?,
                ))
            },
        )
        .optional()
}

pub(super) fn has_material_preparation_on_date(
    transaction: &Transaction<'_>,
    order_id: i64,
    preparation_date: &str,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM preparation_tasks
             WHERE source_order_id=?1 AND preparation_date=?2
               AND state IN ('prepared','verified')
         )",
        params![order_id, preparation_date],
        |row| row.get(0),
    )
}

pub(super) fn no_show_event_exists(
    transaction: &Transaction<'_>,
    order_id: i64,
    scheduled_date: &str,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM order_status_events
             WHERE order_id=?1 AND event_type='no_show' AND effective_date=?2
         )",
        params![order_id, scheduled_date],
        |row| row.get(0),
    )
}

pub(super) fn reschedule_event_exists(
    transaction: &Transaction<'_>,
    order_id: i64,
    missed_date: &str,
    new_date: &str,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM order_status_events
             WHERE order_id=?1 AND event_type='rescheduled'
               AND related_date=?2 AND effective_date=?3
         )",
        params![order_id, missed_date, new_date],
        |row| row.get(0),
    )
}

pub(super) fn insert_status_event(
    transaction: &Transaction<'_>,
    event: NewOrderStatusEvent<'_>,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO order_status_events(
             order_id,event_type,from_status,to_status,effective_date,related_date,actor_user_id
         ) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            event.order_id,
            event.event_type,
            event.from_status.as_database(),
            event.to_status.as_database(),
            event.effective_date,
            event.related_date,
            event.actor_user_id
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_workflow_status(
    transaction: &Transaction<'_>,
    order_id: i64,
    status: OrderWorkflowStatus,
    reason: Option<&str>,
    actor_user_id: i64,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE orders
         SET workflow_status=?1,workflow_status_reason=?2,
             workflow_status_changed_at=CURRENT_TIMESTAMP,
             workflow_status_changed_by_user_id=?3
         WHERE id=?4 AND oncoflow_created=1",
        params![status.as_database(), reason, actor_user_id, order_id],
    )
}

pub(super) fn lookup_exists(
    transaction: &Transaction<'_>,
    table: &'static str,
    id: i64,
) -> rusqlite::Result<bool> {
    let sql = match table {
        "patients" => "SELECT EXISTS(SELECT 1 FROM patients WHERE id=?1)",
        "wards" => "SELECT EXISTS(SELECT 1 FROM wards WHERE id=?1)",
        "doctors" => "SELECT EXISTS(SELECT 1 FROM doctors WHERE id=?1)",
        "regimens" => "SELECT EXISTS(SELECT 1 FROM regimens WHERE id=?1)",
        "drugs" => "SELECT EXISTS(SELECT 1 FROM drugs WHERE id=?1)",
        "diluents" => "SELECT EXISTS(SELECT 1 FROM diluents WHERE id=?1)",
        "routes" => "SELECT EXISTS(SELECT 1 FROM routes WHERE id=?1)",
        "preparation_pharmacists" => {
            "SELECT EXISTS(
            SELECT 1 FROM users
            WHERE id=?1 AND active=1 AND credential_kind='argon2id' AND user_type='pharmacist'
        )"
        }
        _ => unreachable!("lookup table is fixed by the service"),
    };
    transaction.query_row(sql, [id], |row| row.get(0))
}

pub(super) fn is_editable(
    transaction: &Transaction<'_>,
    order_id: i64,
) -> rusqlite::Result<Option<bool>> {
    transaction
        .query_row(
            "SELECT oncoflow_created FROM orders WHERE id=?1",
            [order_id],
            |row| Ok(row.get::<_, i64>(0)? != 0),
        )
        .optional()
}

pub(super) fn insert_order(
    transaction: &Transaction<'_>,
    input: &OrderInput,
) -> rusqlite::Result<i64> {
    let id: i64 = transaction.query_row("SELECT COALESCE(MAX(id),0)+1 FROM orders", [], |row| {
        row.get(0)
    })?;
    let legacy_orderid = format!("OF-{id:08}");
    let (weight_kg, height_cm): (Option<f64>, Option<f64>) = transaction.query_row(
        "SELECT weight_kg,height_cm FROM patients WHERE id=?1",
        [input.patient_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    transaction.execute(
        "INSERT INTO orders(
             id,legacy_orderid,patient_id,ward_id,doctor_id,note,order_time,
             regimen_id,order_type,appointment_flag,oncoflow_created,weight_kg,height_cm,
             assigned_preparer_user_id
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,1,?11,?12,?13)",
        params![
            id,
            legacy_orderid,
            input.patient_id,
            input.ward_id,
            input.doctor_id,
            input.note,
            input.order_time,
            input.regimen_id,
            input.order_type,
            input.appointment_flag as i64,
            weight_kg,
            height_cm,
            input.assigned_preparer_user_id
        ],
    )?;
    Ok(id)
}

pub(super) fn update_order(
    transaction: &Transaction<'_>,
    order_id: i64,
    input: &OrderInput,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE orders SET
             weight_kg=CASE WHEN patient_id<>?1 THEN (SELECT weight_kg FROM patients WHERE id=?1) ELSE weight_kg END,
             height_cm=CASE WHEN patient_id<>?1 THEN (SELECT height_cm FROM patients WHERE id=?1) ELSE height_cm END,
             patient_id=?1,ward_id=?2,doctor_id=?3,note=?4,
             order_time=?5,regimen_id=?6,order_type=?7,appointment_flag=?8,
             assigned_preparer_user_id=?9
         WHERE id=?10 AND oncoflow_created=1",
        params![
            input.patient_id,
            input.ward_id,
            input.doctor_id,
            input.note,
            input.order_time,
            input.regimen_id,
            input.order_type,
            input.appointment_flag as i64,
            input.assigned_preparer_user_id,
            order_id
        ],
    )
}

pub(super) fn update_order_weight(
    transaction: &Transaction<'_>,
    order_id: i64,
    weight_kg: Option<f64>,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE orders SET weight_kg=?1 WHERE id=?2 AND oncoflow_created=1",
        params![weight_kg, order_id],
    )
}

pub(super) fn insert_order_item(
    transaction: &Transaction<'_>,
    order_id: i64,
    value: &NormalizedOrderItemInput,
) -> rusqlite::Result<()> {
    let ordering: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(ordering_no),0)+1 FROM order_items WHERE order_id=?1",
        [order_id],
        |row| row.get(0),
    )?;
    let input = &value.input;
    transaction.execute(
        "INSERT INTO order_items(
             order_id,drug_id,diluent_id,diluent_volume_ml,start_date,stop_date,dose,route_id,
             schedule_time,number_of_drug,missing,printed,rate,ordering_no,legacy_dose_text
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,0,?12,?13,?14)",
        params![
            order_id,
            input.drug_id,
            input.diluent_id,
            input.diluent_volume_ml,
            input.start_date,
            input.stop_date,
            value.parsed_dose,
            input.route_id,
            input.schedule_time,
            input.number_of_drug,
            input.missing as i64,
            input.rate,
            ordering,
            input.dose_text,
        ],
    )?;
    Ok(())
}

pub(super) fn update_order_item(
    transaction: &Transaction<'_>,
    order_id: i64,
    item_id: i64,
    value: &NormalizedOrderItemInput,
) -> rusqlite::Result<usize> {
    let input = &value.input;
    transaction.execute(
        "UPDATE order_items SET drug_id=?1,diluent_id=?2,diluent_volume_ml=?3,
             start_date=?4,stop_date=?5,dose=?6,route_id=?7,schedule_time=?8,number_of_drug=?9,missing=?10,
             rate=?11,legacy_dose_text=?12
         WHERE id=?13 AND order_id=?14",
        params![
            input.drug_id,
            input.diluent_id,
            input.diluent_volume_ml,
            input.start_date,
            input.stop_date,
            value.parsed_dose,
            input.route_id,
            input.schedule_time,
            input.number_of_drug,
            input.missing as i64,
            input.rate,
            input.dose_text,
            item_id,
            order_id,
        ],
    )
}

pub(super) fn delete_order_item(
    transaction: &Transaction<'_>,
    order_id: i64,
    item_id: i64,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "DELETE FROM order_items WHERE id=?1 AND order_id=?2",
        params![item_id, order_id],
    )
}

pub(super) fn order_item_ids(
    transaction: &Transaction<'_>,
    order_id: i64,
) -> rusqlite::Result<Vec<i64>> {
    transaction
        .prepare("SELECT id FROM order_items WHERE order_id=?1")?
        .query_map([order_id], |row| row.get(0))?
        .collect()
}

pub(super) fn set_item_order(
    transaction: &Transaction<'_>,
    order_id: i64,
    item_id: i64,
    ordering_no: i64,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE order_items SET ordering_no=?1 WHERE id=?2 AND order_id=?3",
        params![ordering_no, item_id, order_id],
    )
}

pub(super) fn regimen_has_unusable_items(
    transaction: &Transaction<'_>,
    regimen_id: i64,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM regimen_items i
           JOIN regimen_groups g ON g.id=i.regimen_group_id
           WHERE g.regimen_id=?1 AND (i.drug_id IS NULL OR NOT EXISTS(SELECT 1 FROM drugs d WHERE d.id=i.drug_id))
         )",
        [regimen_id],
        |row| row.get(0),
    )
}

pub(super) fn copy_regimen_items(
    transaction: &Transaction<'_>,
    order_id: i64,
    regimen_id: i64,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "INSERT INTO order_items(
             order_id,drug_id,diluent_id,dose,route_id,rate,ordering_no,
             legacy_dose_text,source_regimen_item_id,regimen_dose_text,
             regimen_unit_text,regimen_route_text,regimen_details,regimen_item_group,
             regimen_duration,regimen_start_day,regimen_ordering_no
         )
         SELECT ?1,i.drug_id,i.default_diluent_id,i.dose,i.default_route_id,i.default_rate,
                ROW_NUMBER() OVER (
                  ORDER BY g.id,
                    CASE WHEN i.item_group IS NULL THEN 0 ELSE 1 END,
                    i.item_group COLLATE NOCASE,
                    CASE WHEN i.ordering_no IS NULL THEN 1 ELSE 0 END,
                    i.ordering_no,i.id
                ),
                COALESCE(i.legacy_dose_text,CASE WHEN i.dose IS NULL THEN NULL ELSE CAST(i.dose AS TEXT) END),
                i.id,COALESCE(i.legacy_dose_text,CASE WHEN i.dose IS NULL THEN NULL ELSE CAST(i.dose AS TEXT) END),
                i.unit_text,i.route_text,i.details,i.item_group,i.duration,i.start_day,i.ordering_no
         FROM regimen_items i
         JOIN regimen_groups g ON g.id=i.regimen_group_id
         WHERE g.regimen_id=?2
         ORDER BY g.id,
           CASE WHEN i.item_group IS NULL THEN 0 ELSE 1 END,
           i.item_group COLLATE NOCASE,
           CASE WHEN i.ordering_no IS NULL THEN 1 ELSE 0 END,
           i.ordering_no,i.id",
        params![order_id, regimen_id],
    )
}

pub(super) fn get_lookups(connection: &Connection) -> rusqlite::Result<OrderLookups> {
    let patients = connection
        .prepare(
            "SELECT id,legacy_hn,
                    COALESCE(NULLIF(trim(COALESCE(title,'') || ' ' || COALESCE(first_name,'') || ' ' || COALESCE(last_name,'')),''),legacy_hn)
             FROM patients ORDER BY last_name,first_name,legacy_hn",
        )?
        .query_map([], |row| {
            Ok(PatientOrderLookupOption {
                id: row.get(0)?,
                hn: row.get(1)?,
                label: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OrderLookups {
        patients,
        regimens: lookup_options(
            connection,
            "SELECT id,regimen_name FROM regimens ORDER BY regimen_name",
        )?,
        drugs: lookup_options(
            connection,
            "SELECT id,drug_name FROM drugs ORDER BY drug_name",
        )?,
        routes: lookup_options(
            connection,
            "SELECT id,route_name FROM routes ORDER BY route_name",
        )?,
        diluents: connection
            .prepare("SELECT id,diluent_name,volume_ml FROM diluents ORDER BY diluent_name")?
            .query_map([], |row| {
                Ok(DiluentOrderLookupOption {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    volume_ml: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?,
        doctors: lookup_options(
            connection,
            "SELECT id,doctor_name FROM doctors ORDER BY doctor_name",
        )?,
        wards: lookup_options(
            connection,
            "SELECT id,ward_name FROM wards ORDER BY ward_name",
        )?,
        preparation_pharmacists: lookup_options(
            connection,
            "SELECT id,COALESCE(display_name,username) FROM users
             WHERE active=1 AND credential_kind='argon2id' AND user_type='pharmacist'
             ORDER BY COALESCE(display_name,username) COLLATE NOCASE,id",
        )?,
    })
}

fn lookup_options(connection: &Connection, sql: &str) -> rusqlite::Result<Vec<OrderLookupOption>> {
    connection
        .prepare(sql)?
        .query_map([], |row| {
            Ok(OrderLookupOption {
                id: row.get(0)?,
                label: row.get(1)?,
            })
        })?
        .collect()
}

fn map_order_summary(row: &Row<'_>) -> rusqlite::Result<OrderSummary> {
    Ok(OrderSummary {
        id: row.get(0)?,
        order_id: row.get(1)?,
        patient_id: row.get(2)?,
        patient_hn: row.get(3)?,
        patient_name: row.get(4)?,
        order_time: row.get(5)?,
        regimen_name: row.get(6)?,
        doctor_name: row.get(7)?,
        ward_name: row.get(8)?,
        order_type: row.get(9)?,
        item_count: row.get(10)?,
        editable: row.get::<_, i64>(11)? != 0,
        workflow_status: OrderWorkflowStatus::from_database(&row.get::<_, String>(12)?)?,
        drugs: serde_json::from_str::<Vec<OrderSummaryDrug>>(&row.get::<_, String>(13)?)
            .unwrap_or_default(),
    })
}

fn map_order_item(row: &Row<'_>) -> rusqlite::Result<OrderItemDetail> {
    Ok(OrderItemDetail {
        id: row.get(0)?,
        drug_id: row.get(1)?,
        drug_name: row.get(2)?,
        diluent_id: row.get(3)?,
        diluent_name: row.get(4)?,
        diluent_volume_ml: row.get(5)?,
        route_id: row.get(6)?,
        route_name: row.get(7)?,
        start_date: row.get(8)?,
        stop_date: row.get(9)?,
        dose: row.get(10)?,
        dose_text: row.get(11)?,
        schedule_time: row.get(12)?,
        number_of_drug: row.get(13)?,
        missing: row.get::<_, i64>(14)? != 0,
        printed: row.get::<_, i64>(15)? != 0,
        rate: row.get(16)?,
        ordering_no: row.get(17)?,
        running_no: row.get(18)?,
        running_sum: row.get(19)?,
        inventory_date: row.get(20)?,
        source_regimen_item_id: row.get(21)?,
        regimen_dose_text: row.get(22)?,
        regimen_unit_text: row.get(23)?,
        regimen_route_text: row.get(24)?,
        regimen_details: row.get(25)?,
        regimen_item_group: row.get(26)?,
        regimen_duration: row.get(27)?,
        regimen_start_day: row.get(28)?,
        regimen_ordering_no: row.get(29)?,
    })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
