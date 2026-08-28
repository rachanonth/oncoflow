use rusqlite::{params, Connection, OptionalExtension};

pub(super) fn list_order_preparation_ids(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<Vec<i64>> {
    connection
        .prepare(
            "SELECT id FROM preparation_tasks
             WHERE source_order_id=?1
             ORDER BY CASE WHEN snapshot_sequence_no IS NULL THEN 1 ELSE 0 END,
                      snapshot_sequence_no,id",
        )?
        .query_map([order_id], |row| row.get(0))?
        .collect()
}

pub(super) fn count_order_eligible_items(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<u64> {
    connection.query_row(
        "SELECT COUNT(*)
         FROM order_items i
         JOIN drugs d ON d.id=i.drug_id
         WHERE i.order_id=?1 AND d.marker=1",
        [order_id],
        |row| row.get(0),
    )
}

use super::{
    expiration::expiration_at, OutputSource, PreparationContainerLabelData, PreparationLabelData,
    PreparationOutput, PreparationSummaryData, PREPARATION_LABEL_TEMPLATE_VERSION,
};

pub(super) fn load_source(
    connection: &Connection,
    preparation_id: i64,
) -> rusqlite::Result<Option<OutputSource>> {
    connection
        .query_row(
            "SELECT t.id,t.state,t.source_order_id,t.source_order_item_id,
                    o.legacy_orderid,p.legacy_hn,
                    NULLIF(trim(COALESCE(p.title,'') || ' ' || COALESCE(p.first_name,'') || ' ' || COALESCE(p.last_name,'')),''),
                    r.regimen_name,t.preparation_date,t.snapshot_treatment_day,
                    d.legacy_dcode,d.drug_name,t.snapshot_ordered_dose_text,
                    t.snapshot_dose_unit_text,t.snapshot_diluent_name,
                    t.snapshot_diluent_volume_ml,t.preparation_volume_ml,
                    t.snapshot_route_name,t.snapshot_rate_text,
                    t.snapshot_regimen_details,t.preparation_notes,t.snapshot_drug_storage,
                    preparer.display_name,t.prepared_at,verifier.display_name,t.verified_at,
                    posting.status,posting.inventory_movement_id,posting.containers_required,
                    posting.balance_before,posting.balance_after,posting.resulting_stock_state,
                    posting.calculation_ruleset_version,posting.calculation_rule_id,
                    t.final_container_count,settings.hospital_name,d.warning,d.expiry_time,
                    d.expiry_storage,t.withdrawal_volume_ml
             FROM preparation_tasks t
             JOIN orders o ON o.id=t.source_order_id
             JOIN patients p ON p.id=o.patient_id
             LEFT JOIN regimens r ON r.id=o.regimen_id
             JOIN drugs d ON d.id=t.drug_id
             LEFT JOIN users preparer ON preparer.id=t.prepared_by_user_id
             LEFT JOIN users verifier ON verifier.id=t.verified_by_user_id
             LEFT JOIN preparation_inventory_postings posting
               ON posting.preparation_task_id=t.id
             LEFT JOIN application_settings settings ON settings.id=1
             WHERE t.id=?1",
            [preparation_id],
            |row| {
                Ok(OutputSource {
                    preparation_id: row.get(0)?,
                    state: row.get(1)?,
                    order_id: row.get(2)?,
                    order_item_id: row.get(3)?,
                    order_reference: row.get(4)?,
                    patient_identifier: row.get(5)?,
                    patient_name: row.get(6)?,
                    regimen_name: row.get(7)?,
                    treatment_at: row.get(8)?,
                    treatment_day: row.get(9)?,
                    drug_code: row.get(10)?,
                    drug_name: row.get(11)?,
                    ordered_dose_text: row.get(12)?,
                    dose_unit_text: row.get(13)?,
                    diluent_name: row.get(14)?,
                    diluent_volume_ml: row.get(15)?,
                    final_volume_ml: row.get(16)?,
                    route_name: row.get(17)?,
                    infusion_rate_or_duration: row.get(18)?,
                    preparation_instructions: row.get(19)?,
                    preparation_notes: row.get(20)?,
                    storage_reference: row.get(21)?,
                    prepared_by: row.get(22)?,
                    prepared_at: row.get(23)?,
                    verified_by: row.get(24)?,
                    verified_at: row.get(25)?,
                    inventory_posting_status: row.get(26)?,
                    inventory_movement_id: row.get(27)?,
                    containers_required: row.get(28)?,
                    inventory_balance_before: row.get(29)?,
                    inventory_balance_after: row.get(30)?,
                    inventory_stock_state: row.get(31)?,
                    calculation_ruleset_version: row.get(32)?,
                    calculation_rule_id: row.get(33)?,
                    final_container_count: u32::try_from(row.get::<_, i64>(34)?).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(34, rusqlite::types::Type::Integer, Box::new(error))
                    })?,
                    hospital_name: row.get(35)?,
                    warning_text: row.get(36)?,
                    expiry_time_text: row.get(37)?,
                    expiry_storage_text: row.get(38)?,
                    withdrawal_volume_ml: row.get(39)?,
                })
            },
        )
        .optional()
}

pub(super) fn insert_snapshot(
    connection: &Connection,
    source: &OutputSource,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT OR IGNORE INTO preparation_output_snapshots(
           preparation_task_id,template_version,source_order_id,source_order_item_id,
           order_reference,patient_identifier,patient_name,regimen_name,treatment_at,
           treatment_day,drug_code,drug_name,ordered_dose_text,dose_unit_text,
           diluent_name,diluent_volume_ml,final_volume_ml,route_name,
           infusion_rate_or_duration,preparation_instructions,preparation_notes,
           storage_reference,prepared_by_display_name,prepared_at,
           verified_by_display_name,verified_at,inventory_posting_status,
           inventory_movement_id,containers_required,inventory_balance_before,
           inventory_balance_after,inventory_stock_state,calculation_ruleset_version,
           calculation_rule_id,final_container_count,hospital_name,warning_text,
           expiry_time_text,expiry_storage_text,withdrawal_volume_ml
         ) VALUES(
           ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,
           ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,
           ?35,?36,?37,?38,?39,?40
         )",
        params![
            source.preparation_id,
            PREPARATION_LABEL_TEMPLATE_VERSION,
            source.order_id,
            source.order_item_id,
            source.order_reference,
            source.patient_identifier,
            source.patient_name,
            source.regimen_name,
            source.treatment_at,
            source.treatment_day,
            source.drug_code,
            source.drug_name,
            source.ordered_dose_text,
            source.dose_unit_text,
            source.diluent_name,
            source.diluent_volume_ml,
            source.final_volume_ml,
            source.route_name,
            source.infusion_rate_or_duration,
            source.preparation_instructions,
            source.preparation_notes,
            source.storage_reference,
            source.prepared_by,
            source.prepared_at,
            source.verified_by,
            source.verified_at,
            source.inventory_posting_status,
            source.inventory_movement_id,
            source.containers_required,
            source.inventory_balance_before,
            source.inventory_balance_after,
            source.inventory_stock_state,
            source.calculation_ruleset_version,
            source.calculation_rule_id,
            source.final_container_count,
            source.hospital_name,
            source.warning_text,
            source.expiry_time_text,
            source.expiry_storage_text,
            source.withdrawal_volume_ml,
        ],
    )?;
    Ok(())
}

pub(super) fn load_snapshot(
    connection: &Connection,
    preparation_id: i64,
) -> rusqlite::Result<Option<PreparationOutput>> {
    connection
        .query_row(
            "SELECT id,template_version,generated_at,preparation_task_id,
                    source_order_id,order_reference,patient_identifier,patient_name,
                    regimen_name,treatment_at,treatment_day,drug_code,drug_name,
                    ordered_dose_text,dose_unit_text,diluent_name,diluent_volume_ml,
                    final_volume_ml,route_name,infusion_rate_or_duration,
                    prepared_by_display_name,prepared_at,verified_by_display_name,verified_at,
                    preparation_instructions,preparation_notes,storage_reference,
                    inventory_posting_status,inventory_movement_id,containers_required,
                    inventory_balance_before,inventory_balance_after,inventory_stock_state,
                    calculation_ruleset_version,calculation_rule_id,
                    final_container_count,hospital_name,warning_text,expiry_time_text,
                    expiry_storage_text,withdrawal_volume_ml,
                    COALESCE(
                      (SELECT COALESCE(
                         NULLIF(json_extract(a.metadata_json,'$.label_print_time'),''),
                         strftime('%Y-%m-%dT%H:%M:%S',a.occurred_at,'localtime')
                       )
                       FROM audit_events a
                       WHERE a.entity_type='preparation_task'
                         AND a.entity_id=CAST(preparation_output_snapshots.preparation_task_id AS TEXT)
                         AND a.event_type IN (
                           'preparation_label_print_requested',
                           'preparation_label_reprint_requested'
                         )
                       ORDER BY a.id
                       LIMIT 1),
                      strftime('%Y-%m-%dT%H:%M:%S','now','localtime')
                    ),
                    (SELECT COUNT(*) FROM audit_events a
                     WHERE a.entity_type='preparation_task'
                       AND a.entity_id=CAST(preparation_output_snapshots.preparation_task_id AS TEXT)
                       AND a.event_type IN (
                         'preparation_label_print_requested',
                         'preparation_label_reprint_requested'
                       ))
             FROM preparation_output_snapshots
             WHERE preparation_task_id=?1",
            [preparation_id],
            |row| {
                let print_time = row.get::<_, String>(41)?;
                let expiry_time_text = row.get::<_, Option<String>>(38)?;
                Ok(PreparationOutput {
                    label: PreparationLabelData {
                        snapshot_id: row.get(0)?,
                        template_version: row.get(1)?,
                        generated_at: row.get(2)?,
                        print_time: print_time.clone(),
                        expiration_at: expiration_at(
                            &print_time,
                            expiry_time_text.as_deref(),
                        ),
                        preparation_id: row.get(3)?,
                        order_id: row.get(4)?,
                        order_reference: row.get(5)?,
                        patient_identifier: row.get(6)?,
                        patient_name: row.get(7)?,
                        hospital_name: row.get(36)?,
                        regimen_name: row.get(8)?,
                        treatment_at: row.get(9)?,
                        treatment_day: row.get(10)?,
                        drug_code: row.get(11)?,
                        drug_name: row.get(12)?,
                        ordered_dose_text: row.get(13)?,
                        dose_unit_text: row.get(14)?,
                        diluent_name: row.get(15)?,
                        diluent_volume_ml: row.get(16)?,
                        withdrawal_volume_ml: row.get(40)?,
                        final_volume_ml: row.get(17)?,
                        route_name: row.get(18)?,
                        infusion_rate_or_duration: row.get(19)?,
                        warning_text: row.get(37)?,
                        expiry_time_text,
                        expiry_storage_text: row.get(39)?,
                        prepared_by: row.get(20)?,
                        prepared_at: row.get(21)?,
                        verified_by: row.get(22)?,
                        verified_at: row.get(23)?,
                    },
                    containers: snapshot_containers(row.get(35)?)?,
                    summary: PreparationSummaryData {
                        preparation_instructions: row.get(24)?,
                        preparation_notes: row.get(25)?,
                        storage_reference: row.get(26)?,
                        safety_review_status: "verified_workflow_complete",
                        inventory_posting_status: row.get(27)?,
                        inventory_movement_id: row.get(28)?,
                        containers_required: row.get(29)?,
                        inventory_balance_before: row.get(30)?,
                        inventory_balance_after: row.get(31)?,
                        inventory_stock_state: row.get(32)?,
                        calculation_ruleset_version: row.get(33)?,
                        calculation_rule_id: row.get(34)?,
                        presentation_notice: "Only persisted verification values are shown; no preparation calculation runs during output rendering.",
                    },
                    print_request_count: row.get(42)?,
                })
            },
        )
        .optional()
}

fn snapshot_containers(count: i64) -> rusqlite::Result<Vec<PreparationContainerLabelData>> {
    let count = u32::try_from(count).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            35,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })?;
    if !(1..=20).contains(&count) {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            35,
            rusqlite::types::Type::Integer,
            "final container snapshot count is outside the supported range".into(),
        ));
    }
    Ok((1..=count)
        .map(|container_index| PreparationContainerLabelData { container_index })
        .collect())
}
