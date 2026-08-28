use rusqlite::{Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SafetyOrderHeader {
    pub patient_id: i64,
    pub regimen_id: Option<i64>,
    pub editable: bool,
    pub legacy_bsa: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SafetyOrderItem {
    pub id: i64,
    pub drug_id: i64,
    pub drug_name: String,
    pub dose: Option<f64>,
    pub unit_name: Option<String>,
    pub dose_per_pack: Option<f64>,
    pub volume_per_pack: Option<f64>,
    pub diluent_name: Option<String>,
    pub diluent_volume: Option<f64>,
    pub warning: Option<String>,
    pub max_dose: Option<f64>,
    pub max_dilution_enabled: bool,
    pub max_dilution_threshold: Option<f64>,
    pub cumulative_enabled: bool,
    pub cumulative_threshold: Option<f64>,
    pub incompatibility_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CumulativeDrugCandidate {
    pub drug_id: i64,
    pub drug_name: String,
    pub threshold: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CumulativeDoseExposure {
    pub dose: Option<f64>,
    pub weight_kg: Option<f64>,
    pub height_cm: Option<f64>,
    pub unit_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct AlertSettings {
    pub note_alert: bool,
    pub side_effect_alert: bool,
    pub soap_alert: bool,
    pub new_order_alert: bool,
    pub cycle_alert: bool,
    pub plan_alert: bool,
    pub wbc_threshold: Option<f64>,
    pub anc_threshold: Option<f64>,
    pub platelet_threshold: Option<f64>,
    pub haemoglobin_threshold: Option<f64>,
    pub ast_threshold: Option<f64>,
    pub bilirubin_threshold: Option<f64>,
    pub creatinine_threshold: Option<f64>,
}

impl AlertSettings {
    pub(super) fn has_lab_thresholds(self) -> bool {
        [
            self.wbc_threshold,
            self.anc_threshold,
            self.platelet_threshold,
            self.haemoglobin_threshold,
            self.ast_threshold,
            self.bilirubin_threshold,
            self.creatinine_threshold,
        ]
        .into_iter()
        .any(|value| value.is_some())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RegimenAlertSettings {
    pub drug_alert: bool,
    pub appointment_alert: bool,
    pub counsel_alert: bool,
    pub cycle_check: bool,
}

pub(super) fn load_order_header(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<Option<SafetyOrderHeader>> {
    connection
        .query_row(
            "SELECT o.patient_id,o.regimen_id,o.oncoflow_created,p.legacy_bsa
             FROM orders o JOIN patients p ON p.id=o.patient_id WHERE o.id=?1",
            [order_id],
            |row| {
                Ok(SafetyOrderHeader {
                    patient_id: row.get(0)?,
                    regimen_id: row.get(1)?,
                    editable: row.get::<_, i64>(2)? != 0,
                    legacy_bsa: row.get(3)?,
                })
            },
        )
        .optional()
}

pub(super) fn load_order_items(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<Vec<SafetyOrderItem>> {
    connection
        .prepare(
            "SELECT i.id,i.drug_id,d.drug_name,i.dose,u.unit_name,
                    d.dose_per_pack,d.volume_per_pack_ml,dl.diluent_name,
                    COALESCE(i.diluent_volume_ml,dl.volume_ml),
                    d.warning,d.max_dose,COALESCE(d.max_dilution_alert,0)<>0,
                    d.max_dilution_hard,COALESCE(d.cumulative_alert,0)<>0,
                    d.cumulative_alert_hard,d.dilution_incompatibility
             FROM order_items i
             JOIN drugs d ON d.id=i.drug_id
             LEFT JOIN units u ON u.id=d.unit_id
             LEFT JOIN diluents dl ON dl.id=i.diluent_id
             WHERE i.order_id=?1
             ORDER BY CASE WHEN i.ordering_no IS NULL THEN 1 ELSE 0 END,i.ordering_no,i.id",
        )?
        .query_map([order_id], |row| {
            Ok(SafetyOrderItem {
                id: row.get(0)?,
                drug_id: row.get(1)?,
                drug_name: row.get(2)?,
                dose: row.get(3)?,
                unit_name: row.get(4)?,
                dose_per_pack: row.get(5)?,
                volume_per_pack: row.get(6)?,
                diluent_name: row.get(7)?,
                diluent_volume: row.get(8)?,
                warning: row.get(9)?,
                max_dose: row.get(10)?,
                max_dilution_enabled: row.get::<_, i64>(11)? != 0,
                max_dilution_threshold: row.get(12)?,
                cumulative_enabled: row.get::<_, i64>(13)? != 0,
                cumulative_threshold: row.get(14)?,
                incompatibility_code: row.get(15)?,
            })
        })?
        .collect()
}

pub(super) fn load_alert_settings(
    connection: &Connection,
) -> rusqlite::Result<Option<AlertSettings>> {
    connection
        .query_row(
            "SELECT note_alert<>0,side_effect_alert<>0,soap_alert<>0,
                    new_order_alert<>0,cycle_alert<>0,plan_alert<>0,
                    wbc_threshold,anc_threshold,platelet_threshold,
                    haemoglobin_threshold,ast_threshold,bilirubin_threshold,
                    creatinine_threshold
             FROM alert_settings WHERE id=1",
            [],
            |row| {
                Ok(AlertSettings {
                    note_alert: row.get::<_, i64>(0)? != 0,
                    side_effect_alert: row.get::<_, i64>(1)? != 0,
                    soap_alert: row.get::<_, i64>(2)? != 0,
                    new_order_alert: row.get::<_, i64>(3)? != 0,
                    cycle_alert: row.get::<_, i64>(4)? != 0,
                    plan_alert: row.get::<_, i64>(5)? != 0,
                    wbc_threshold: row.get(6)?,
                    anc_threshold: row.get(7)?,
                    platelet_threshold: row.get(8)?,
                    haemoglobin_threshold: row.get(9)?,
                    ast_threshold: row.get(10)?,
                    bilirubin_threshold: row.get(11)?,
                    creatinine_threshold: row.get(12)?,
                })
            },
        )
        .optional()
}

pub(super) fn load_regimen_alert_settings(
    connection: &Connection,
    regimen_id: i64,
) -> rusqlite::Result<Option<RegimenAlertSettings>> {
    connection
        .query_row(
            "SELECT drug_alert<>0,appointment_alert<>0,counsel_alert<>0,cycle_check<>0
             FROM regimens WHERE id=?1",
            [regimen_id],
            |row| {
                Ok(RegimenAlertSettings {
                    drug_alert: row.get::<_, i64>(0)? != 0,
                    appointment_alert: row.get::<_, i64>(1)? != 0,
                    counsel_alert: row.get::<_, i64>(2)? != 0,
                    cycle_check: row.get::<_, i64>(3)? != 0,
                })
            },
        )
        .optional()
}

pub(super) fn compatible_cumulative_total(
    connection: &Connection,
    patient_id: i64,
    drug_id: i64,
) -> rusqlite::Result<Option<f64>> {
    connection.query_row(
        "SELECT SUM(i.dose)
         FROM order_items i
         JOIN orders o ON o.id=i.order_id
         JOIN patients p ON p.id=o.patient_id
         JOIN wards w ON w.id=o.ward_id
         JOIN doctors doc ON doc.id=o.doctor_id
         JOIN drugs d ON d.id=i.drug_id
         JOIN units u ON u.id=d.unit_id
         JOIN diluents dl ON dl.id=i.diluent_id
         JOIN routes rt ON rt.id=i.route_id
         JOIN diagnoses diagnosis ON diagnosis.id=p.diagnosis_id
         JOIN regimens regimen ON regimen.id=p.regimen_id
         WHERE o.patient_id=?1 AND i.drug_id=?2",
        [patient_id, drug_id],
        |row| row.get(0),
    )
}

pub(super) fn cumulative_drug_candidates(
    connection: &Connection,
    patient_id: i64,
) -> rusqlite::Result<Vec<CumulativeDrugCandidate>> {
    connection
        .prepare(
            "SELECT DISTINCT d.id,d.drug_name,d.cumulative_alert_hard
             FROM orders o
             JOIN order_items i ON i.order_id=o.id
             JOIN drugs d ON d.id=i.drug_id
             WHERE o.patient_id=?1 AND COALESCE(d.cumulative_alert,0)<>0
             ORDER BY d.drug_name COLLATE NOCASE,d.id",
        )?
        .query_map([patient_id], |row| {
            Ok(CumulativeDrugCandidate {
                drug_id: row.get(0)?,
                drug_name: row.get(1)?,
                threshold: row.get(2)?,
            })
        })?
        .collect()
}

pub(super) fn cumulative_dose_exposures(
    connection: &Connection,
    patient_id: i64,
    drug_id: i64,
) -> rusqlite::Result<Vec<CumulativeDoseExposure>> {
    connection
        .prepare(
            "SELECT i.dose,o.weight_kg,o.height_cm,u.unit_name
             FROM order_items i
             JOIN orders o ON o.id=i.order_id
             JOIN drugs d ON d.id=i.drug_id
             LEFT JOIN units u ON u.id=d.unit_id
             WHERE o.patient_id=?1 AND i.drug_id=?2
             ORDER BY o.id,i.id",
        )?
        .query_map([patient_id, drug_id], |row| {
            Ok(CumulativeDoseExposure {
                dose: row.get(0)?,
                weight_kg: row.get(1)?,
                height_cm: row.get(2)?,
                unit_name: row.get(3)?,
            })
        })?
        .collect()
}

pub(super) fn prior_order_note_exists(
    connection: &Connection,
    patient_id: i64,
    current_order_id: i64,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM orders
         WHERE patient_id=?1 AND id<>?2 AND trim(COALESCE(note,''))<>'')",
        [patient_id, current_order_id],
        |row| row.get(0),
    )
}

pub(super) fn side_effect_exists(
    connection: &Connection,
    patient_id: i64,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM side_effect_records WHERE patient_id=?1)",
        [patient_id],
        |row| row.get(0),
    )
}

pub(super) fn soap_exists(connection: &Connection, patient_id: i64) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pharmcare_soap WHERE patient_id=?1)",
        [patient_id],
        |row| row.get(0),
    )
}

pub(super) fn appointment_exists(
    connection: &Connection,
    patient_id: i64,
    regimen_id: i64,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM appointments
         WHERE patient_id=?1 AND regimen_id=?2)",
        [patient_id, regimen_id],
        |row| row.get(0),
    )
}

pub(super) fn counselling_exists(
    connection: &Connection,
    patient_id: i64,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pharmcare_records
         WHERE patient_id=?1 AND p2<>0)",
        [patient_id],
        |row| row.get(0),
    )
}
