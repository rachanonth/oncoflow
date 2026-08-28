use rusqlite::Connection;
use thiserror::Error;

use crate::{
    clinical::LEGACY_RULESET,
    db::{Database, DatabaseError},
};

use super::{
    model::{
        evidence, CumulativeDoseSummary, SafetyEvaluation, SafetyFinding, SafetyFindingStatus,
        SafetySeverity,
    },
    repository,
    rules::{
        evaluate_concentration, evaluate_cumulative, evaluate_dilution_incompatibility,
        is_milligram_unit, ConcentrationInput, CumulativeInput, RuleOutcome, RuleOutcomeStatus,
    },
};

const SOURCE_DRUG: &str = "legacy Tbldrug safety configuration";
const SOURCE_ORDER_FORM: &str = "legacy order/order-details form source";
const SOURCE_ALERT_FORM: &str = "legacy order alert presence checks";

#[derive(Debug, Error)]
pub(crate) enum SafetyError {
    #[error("order record was not found")]
    OrderNotFound,
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct SafetyService<'a> {
    database: &'a Database,
}

impl<'a> SafetyService<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub(crate) fn evaluate_order(&self, order_id: i64) -> Result<SafetyEvaluation, SafetyError> {
        let connection = self.database.open()?;
        evaluate_order(&connection, order_id)
    }
}

pub(crate) fn cumulative_dose_summaries(
    connection: &Connection,
    patient_id: i64,
) -> rusqlite::Result<Vec<CumulativeDoseSummary>> {
    repository::cumulative_drug_candidates(connection, patient_id)?
        .into_iter()
        .map(|candidate| {
            let exposures =
                repository::cumulative_dose_exposures(connection, patient_id, candidate.drug_id)?;
            Ok(CumulativeDoseSummary {
                drug_id: candidate.drug_id,
                drug_name: candidate.drug_name,
                total_dose: normalized_cumulative_total(&exposures),
                threshold: optional_numeric(candidate.threshold),
            })
        })
        .collect()
}

fn normalized_cumulative_total(exposures: &[repository::CumulativeDoseExposure]) -> Option<String> {
    if exposures.is_empty() {
        return None;
    }
    let mut total = 0.0;
    for exposure in exposures {
        if !is_milligram_unit(exposure.unit_name.as_deref()) {
            return None;
        }
        let (dose, weight_kg, height_cm) =
            (exposure.dose?, exposure.weight_kg?, exposure.height_cm?);
        if !dose.is_finite()
            || dose < 0.0
            || !weight_kg.is_finite()
            || weight_kg <= 0.0
            || !height_cm.is_finite()
            || height_cm <= 0.0
        {
            return None;
        }
        let bsa = ((weight_kg * height_cm) / 3600.0).sqrt();
        if !bsa.is_finite() || bsa <= 0.0 {
            return None;
        }
        total += dose / bsa;
    }
    finite_numeric(total)
}

fn finite_numeric(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    let value = format!("{value:.6}");
    Some(value.trim_end_matches('0').trim_end_matches('.').to_owned())
}

pub(crate) fn evaluate_order(
    connection: &Connection,
    order_id: i64,
) -> Result<SafetyEvaluation, SafetyError> {
    let header =
        repository::load_order_header(connection, order_id)?.ok_or(SafetyError::OrderNotFound)?;
    if !header.editable {
        return Ok(SafetyEvaluation::historical());
    }

    let mut findings = Vec::new();
    let mut evaluated_rule_count = 0_usize;
    for item in repository::load_order_items(connection, order_id)? {
        if let Some(warning) = item
            .warning
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            evaluated_rule_count += 1;
            findings.push(finding(
                format!("legacy.drug_warning:item:{}", item.id),
                "legacy.drug_warning",
                SafetySeverity::Info,
                SafetyFindingStatus::Advisory,
                format!("Drug advisory — {}", item.drug_name),
                warning.to_owned(),
                vec![evidence("Configuration", "Tbldrug.warning (display only)")],
                SOURCE_DRUG,
                Some(item.id),
            ));
        }

        if let Some(maximum) = item.max_dose {
            findings.push(unsupported_finding(
                format!("legacy.maximum_dose:item:{}", item.id),
                "legacy.maximum_dose",
                format!("Maximum-dose configuration pending — {}", item.drug_name),
                "A stored maximum-dose value exists, but no recovered legacy call site establishes a safe comparison rule.",
                vec![evidence("Stored maximum-dose value", numeric(maximum))],
                SOURCE_DRUG,
                Some(item.id),
            ));
        }

        evaluated_rule_count += 1;
        let dose = optional_numeric(item.dose);
        let dose_per_pack = optional_numeric(item.dose_per_pack);
        let volume_per_pack = optional_numeric(item.volume_per_pack);
        let diluent_volume = optional_numeric(item.diluent_volume);
        let max_dilution = optional_numeric(item.max_dilution_threshold);
        push_rule_outcome(
            &mut findings,
            format!("legacy.max_dilution_concentration:item:{}", item.id),
            "legacy.max_dilution_concentration",
            format!("Review concentration — {}", item.drug_name),
            SOURCE_ORDER_FORM,
            Some(item.id),
            evaluate_concentration(ConcentrationInput {
                enabled: item.max_dilution_enabled,
                dose: dose.as_deref(),
                dose_per_pack: dose_per_pack.as_deref(),
                volume_per_pack: volume_per_pack.as_deref(),
                diluent_volume: diluent_volume.as_deref(),
                threshold: max_dilution.as_deref(),
                unit: item.unit_name.as_deref(),
            }),
        );

        evaluated_rule_count += 1;
        let cumulative_total = if item.cumulative_enabled {
            repository::compatible_cumulative_total(connection, header.patient_id, item.drug_id)?
        } else {
            None
        };
        let cumulative_total = optional_numeric(cumulative_total);
        let legacy_bsa = optional_numeric(header.legacy_bsa);
        let cumulative_threshold = optional_numeric(item.cumulative_threshold);
        push_rule_outcome(
            &mut findings,
            format!("legacy.cumulative_dose:item:{}", item.id),
            "legacy.cumulative_dose",
            format!("Review cumulative exposure — {}", item.drug_name),
            SOURCE_ORDER_FORM,
            Some(item.id),
            evaluate_cumulative(CumulativeInput {
                enabled: item.cumulative_enabled,
                compatible_total_dose: cumulative_total.as_deref(),
                bsa: legacy_bsa.as_deref(),
                threshold: cumulative_threshold.as_deref(),
                unit: item.unit_name.as_deref(),
            }),
        );

        evaluated_rule_count += 1;
        push_rule_outcome(
            &mut findings,
            format!("legacy.dilution_incompatibility:item:{}", item.id),
            "legacy.dilution_incompatibility",
            format!("Review diluent compatibility — {}", item.drug_name),
            SOURCE_ORDER_FORM,
            Some(item.id),
            evaluate_dilution_incompatibility(
                item.incompatibility_code.as_deref(),
                item.diluent_name.as_deref(),
            ),
        );
    }

    let settings = repository::load_alert_settings(connection)?;
    if let Some(settings) = settings {
        if settings.note_alert {
            evaluated_rule_count += 1;
            if repository::prior_order_note_exists(connection, header.patient_id, order_id)? {
                findings.push(advisory_presence(
                    "legacy.prior_order_note",
                    "Prior pharmacist/order note exists",
                    "A prior local order contains a note. Review it in its original context; note text is not copied into this finding.",
                ));
            }
        }
        if settings.side_effect_alert {
            evaluated_rule_count += 1;
            if repository::side_effect_exists(connection, header.patient_id)? {
                findings.push(advisory_presence(
                    "legacy.side_effect_history",
                    "Side-effect history is recorded",
                    "A local side-effect record exists. Review the source record; its contents are not copied into this finding.",
                ));
            }
        }
        if settings.soap_alert {
            evaluated_rule_count += 1;
            if repository::soap_exists(connection, header.patient_id)? {
                findings.push(advisory_presence(
                    "legacy.soap_history",
                    "SOAP history is recorded",
                    "A local pharmaceutical-care SOAP record exists. Review the source record; its contents are not copied into this finding.",
                ));
            }
        }
        if settings.new_order_alert {
            findings.push(unsupported_finding(
                "legacy.new_order_queue:order".into(),
                "legacy.new_order_queue",
                "New-order queue alert pending".into(),
                "The recovered rule is a startup/unprinted-work queue check, not a per-order clinical rule.",
                Vec::new(),
                SOURCE_ALERT_FORM,
                None,
            ));
        }
        if settings.plan_alert {
            findings.push(unsupported_finding(
                "legacy.plan_alert:order".into(),
                "legacy.plan_alert",
                "Planning alert pending".into(),
                "The current order workflow has no confirmed equivalent for the legacy Plan alert.",
                Vec::new(),
                SOURCE_ALERT_FORM,
                None,
            ));
        }
        if settings.has_lab_thresholds() {
            findings.push(unsupported_finding(
                "legacy.lab_thresholds:order".into(),
                "legacy.lab_thresholds",
                "Laboratory thresholds not evaluated".into(),
                "Thresholds are preserved locally, but no local laboratory input or confirmed order action exists in this milestone.",
                Vec::new(),
                SOURCE_ALERT_FORM,
                None,
            ));
        }
    }

    if let Some(regimen_id) = header.regimen_id {
        if let Some(regimen) = repository::load_regimen_alert_settings(connection, regimen_id)? {
            if regimen.appointment_alert {
                evaluated_rule_count += 1;
                if !repository::appointment_exists(connection, header.patient_id, regimen_id)? {
                    findings.push(triggered_presence(
                        "legacy.appointment_required",
                        "Appointment record not found",
                        "The selected regimen enables the legacy appointment alert, but no local appointment for this patient and regimen was found.",
                    ));
                }
            }
            if regimen.counsel_alert {
                evaluated_rule_count += 1;
                if !repository::counselling_exists(connection, header.patient_id)? {
                    findings.push(triggered_presence(
                        "legacy.counselling_required",
                        "Counselling record not found",
                        "The selected regimen enables the legacy counselling alert, but no local pharmaceutical-care record has the confirmed counselling flag.",
                    ));
                }
            }
            if regimen.drug_alert {
                findings.push(unsupported_finding(
                    "legacy.regimen_dose_variance:order".into(),
                    "legacy.regimen_dose_variance",
                    "Regimen dose-variance alert pending".into(),
                    "The legacy percentage source was not migrated and missing unit/BSA behavior is not fully established.",
                    Vec::new(),
                    SOURCE_ORDER_FORM,
                    None,
                ));
            }
            if settings.is_some_and(|settings| settings.cycle_alert) && regimen.cycle_check {
                findings.push(unsupported_finding(
                    "legacy.cycle_timing:order".into(),
                    "legacy.cycle_timing",
                    "Cycle timing alert pending".into(),
                    "Legacy date/query behavior is not sufficiently confirmed for the local order workflow.",
                    Vec::new(),
                    SOURCE_ALERT_FORM,
                    None,
                ));
            }
        }
    }

    Ok(SafetyEvaluation::active(findings, evaluated_rule_count))
}

fn push_rule_outcome(
    findings: &mut Vec<SafetyFinding>,
    id: String,
    rule_id: &'static str,
    title: String,
    source: &'static str,
    order_item_id: Option<i64>,
    outcome: RuleOutcome,
) {
    let (severity, status) = match outcome.status {
        RuleOutcomeStatus::Clear => return,
        RuleOutcomeStatus::Triggered => (SafetySeverity::Warning, SafetyFindingStatus::Triggered),
        RuleOutcomeStatus::Unsupported => (SafetySeverity::Info, SafetyFindingStatus::Unsupported),
    };
    findings.push(finding(
        id,
        rule_id,
        severity,
        status,
        title,
        format!(
            "{} OncoFlow did not change any entered value.",
            outcome.detail
        ),
        outcome.evidence,
        source,
        order_item_id,
    ));
}

#[allow(clippy::too_many_arguments)]
fn finding(
    id: String,
    rule_id: &'static str,
    severity: SafetySeverity,
    status: SafetyFindingStatus,
    title: String,
    message: String,
    evidence: Vec<super::SafetyEvidence>,
    source: &'static str,
    order_item_id: Option<i64>,
) -> SafetyFinding {
    let mut finding = SafetyFinding {
        id,
        fingerprint: String::new(),
        rule_id,
        ruleset_version: LEGACY_RULESET,
        severity,
        title,
        message,
        evidence,
        source,
        status,
        order_item_id,
        acknowledgement_required: severity == SafetySeverity::Warning,
    };
    finding.fingerprint = super::finding_fingerprint(&finding);
    finding
}

fn unsupported_finding(
    id: String,
    rule_id: &'static str,
    title: String,
    message: &str,
    evidence: Vec<super::SafetyEvidence>,
    source: &'static str,
    order_item_id: Option<i64>,
) -> SafetyFinding {
    finding(
        id,
        rule_id,
        SafetySeverity::Info,
        SafetyFindingStatus::Unsupported,
        title,
        message.into(),
        evidence,
        source,
        order_item_id,
    )
}

fn advisory_presence(rule_id: &'static str, title: &str, message: &str) -> SafetyFinding {
    finding(
        format!("{rule_id}:order"),
        rule_id,
        SafetySeverity::Info,
        SafetyFindingStatus::Advisory,
        title.into(),
        message.into(),
        Vec::new(),
        SOURCE_ALERT_FORM,
        None,
    )
}

fn triggered_presence(rule_id: &'static str, title: &str, message: &str) -> SafetyFinding {
    finding(
        format!("{rule_id}:order"),
        rule_id,
        SafetySeverity::Warning,
        SafetyFindingStatus::Triggered,
        title.into(),
        message.into(),
        Vec::new(),
        SOURCE_ALERT_FORM,
        None,
    )
}

fn numeric(value: f64) -> String {
    value.to_string()
}

fn optional_numeric(value: Option<f64>) -> Option<String> {
    value.map(numeric)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::db::{apply_migrations, configure_connection};

    fn fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        configure_connection(&connection).unwrap();
        apply_migrations(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO units(id,legacy_unitcode,unit_name) VALUES(1,'M','mg.');
                 INSERT INTO routes(id,legacy_rcode,route_name) VALUES(1,'I','IV');
                 INSERT INTO diluents(id,legacy_dilcode,diluent_name,volume_ml)
                   VALUES(1,'D1','D5W',90);
                 INSERT INTO diagnoses(id,legacy_diagcode,diagnosis)
                   VALUES(1,'SYN-DX','Synthetic diagnosis');
                 INSERT INTO doctors(id,legacy_doccode,doctor_name)
                   VALUES(1,'SYN-DOC','Synthetic doctor');
                 INSERT INTO wards(id,legacy_wcode,ward_name)
                   VALUES(1,'SYN-W','Synthetic ward');
                 INSERT INTO regimens(
                   id,legacy_regcode,regimen_name,drug_alert,appointment_alert,
                   counsel_alert,cycle_check
                 ) VALUES(1,'SYN-R','Synthetic regimen',0,1,1,0);
                 INSERT INTO patients(
                   id,legacy_hn,first_name,diagnosis_id,regimen_id,legacy_bsa
                 ) VALUES(1,'SYN-HN','Synthetic',1,1,2);
                 INSERT INTO drugs(
                   id,legacy_dcode,drug_name,unit_id,dose_per_pack,volume_per_pack_ml,
                   warning,max_dose,max_dilution_alert,max_dilution_hard,
                   cumulative_alert,cumulative_alert_hard,dilution_incompatibility
                 ) VALUES(
                   1,'SYN-D','Synthetic drug',1,100,10,'Synthetic advisory',500,1,0.5,
                   1,100,'D'
                 );
                 INSERT INTO orders(
                   id,legacy_orderid,patient_id,ward_id,doctor_id,regimen_id,note,
                   oncoflow_created,
                   weight_kg,height_cm
                 ) VALUES(1,'SYN-HIST',1,1,1,1,'Synthetic prior note',0,90,160);
                 INSERT INTO order_items(
                   id,order_id,drug_id,diluent_id,route_id,dose,ordering_no
                 ) VALUES(1,1,1,1,1,100,1);
                 INSERT INTO orders(
                   id,legacy_orderid,patient_id,ward_id,doctor_id,regimen_id,
                   oncoflow_created,weight_kg,height_cm
                 ) VALUES(2,'SYN-LOCAL',1,1,1,1,1,90,160);
                 INSERT INTO order_items(
                   id,order_id,drug_id,diluent_id,route_id,dose,ordering_no
                 ) VALUES(2,2,1,1,1,100,1);
                 INSERT INTO side_effect_catalog(id,legacy_secode,side_effect_name)
                   VALUES(1,'SYN-SE','Synthetic effect');
                 INSERT INTO side_effect_records(id,patient_id,side_effect_id)
                   VALUES(1,1,1);
                 INSERT INTO pharmcare_soap(id,legacy_soapcode,patient_id,problem)
                   VALUES(1,'SYN-SOAP',1,'Synthetic problem');
                 UPDATE alert_settings SET
                   note_alert=1,side_effect_alert=1,soap_alert=1,new_order_alert=0,
                   cycle_alert=0,plan_alert=0,wbc_threshold=NULL,anc_threshold=NULL,
                   platelet_threshold=NULL,haemoglobin_threshold=NULL,ast_threshold=NULL,
                   bilirubin_threshold=NULL,creatinine_threshold=NULL
                 WHERE id=1;",
            )
            .unwrap();
        connection
    }

    #[test]
    fn evaluates_confirmed_rules_and_presence_alerts_deterministically() {
        let connection = fixture();
        let first = evaluate_order(&connection, 2).unwrap();
        let second = evaluate_order(&connection, 2).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.ruleset_version, LEGACY_RULESET);
        assert_eq!(first.findings.len(), 10);
        assert_eq!(first.evaluated_rule_count, 9);
        assert_eq!(first.unsupported_rule_count, 1);
        assert!(first
            .findings
            .iter()
            .all(|finding| finding.ruleset_version == LEGACY_RULESET));
        for rule in [
            "legacy.max_dilution_concentration",
            "legacy.cumulative_dose",
            "legacy.dilution_incompatibility",
            "legacy.appointment_required",
            "legacy.counselling_required",
        ] {
            assert!(first.findings.iter().any(|finding| {
                finding.rule_id == rule
                    && finding.status == SafetyFindingStatus::Triggered
                    && finding.acknowledgement_required
            }));
        }
    }

    #[test]
    fn cumulative_summary_is_read_only_and_does_not_evaluate_the_threshold() {
        let connection = fixture();
        connection
            .execute_batch(
                "INSERT INTO drugs(id,legacy_dcode,drug_name,cumulative_alert)
                   VALUES(2,'NO-CUM','Disabled cumulative drug',0);
                 INSERT INTO order_items(id,order_id,drug_id,dose,ordering_no)
                   VALUES(3,2,2,50,2);",
            )
            .unwrap();

        let summaries = cumulative_dose_summaries(&connection, 1).unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].drug_name, "Synthetic drug");
        assert_eq!(summaries[0].total_dose.as_deref(), Some("100"));
        assert_eq!(summaries[0].threshold.as_deref(), Some("100"));
    }

    #[test]
    fn cumulative_summary_normalizes_each_dose_with_its_own_order_snapshot() {
        let exposures = [
            repository::CumulativeDoseExposure {
                dose: Some(100.0),
                weight_kg: Some(90.0),
                height_cm: Some(160.0),
                unit_name: Some("mg".into()),
            },
            repository::CumulativeDoseExposure {
                dose: Some(90.0),
                weight_kg: Some(90.0),
                height_cm: Some(90.0),
                unit_name: Some("mg.".into()),
            },
        ];

        assert_eq!(
            normalized_cumulative_total(&exposures).as_deref(),
            Some("110")
        );

        let incomplete = [repository::CumulativeDoseExposure {
            height_cm: None,
            ..exposures[0].clone()
        }];
        assert_eq!(normalized_cumulative_total(&incomplete), None);
    }

    #[test]
    fn order_item_diluent_volume_overrides_the_master_volume() {
        let connection = fixture();
        connection
            .execute(
                "UPDATE order_items SET diluent_volume_ml=500 WHERE id=2",
                [],
            )
            .unwrap();

        let evaluation = evaluate_order(&connection, 2).unwrap();

        assert!(!evaluation.findings.iter().any(|finding| {
            finding.rule_id == "legacy.max_dilution_concentration"
                && finding.status == SafetyFindingStatus::Triggered
        }));
    }

    #[test]
    fn existing_appointment_and_counselling_remove_missing_record_warnings() {
        let connection = fixture();
        connection
            .execute_batch(
                "INSERT INTO appointments(
                   id,legacy_appid,patient_id,appointment_date,regimen_id
                 ) VALUES(1,'SYN-APP',1,'2026-08-23',1);
                 INSERT INTO pharmcare_records(
                   id,legacy_prcode,patient_id,p2
                 ) VALUES(1,'SYN-PC',1,1);",
            )
            .unwrap();

        let evaluation = evaluate_order(&connection, 2).unwrap();
        assert!(!evaluation.findings.iter().any(|finding| {
            matches!(
                finding.rule_id,
                "legacy.appointment_required" | "legacy.counselling_required"
            )
        }));
    }

    #[test]
    fn historical_orders_are_not_recomputed_or_modified() {
        let connection = fixture();
        let before: (f64, i64, String) = connection
            .query_row(
                "SELECT i.dose,o.regimen_id,o.note FROM orders o
                 JOIN order_items i ON i.order_id=o.id WHERE o.id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        let evaluation = evaluate_order(&connection, 1).unwrap();
        let after: (f64, i64, String) = connection
            .query_row(
                "SELECT i.dose,o.regimen_id,o.note FROM orders o
                 JOIN order_items i ON i.order_id=o.id WHERE o.id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(
            evaluation.mode,
            super::super::SafetyEvaluationMode::HistoricalNotEvaluated
        );
        assert!(evaluation.findings.is_empty());
        assert_eq!(before, after);
    }

    #[test]
    fn active_evaluation_never_changes_dose_regimen_or_records() {
        let connection = fixture();
        let snapshot = || {
            connection
                .query_row(
                    "SELECT i.dose,o.regimen_id,
                            (SELECT COUNT(*) FROM orders),
                            (SELECT COUNT(*) FROM order_items),
                            (SELECT COUNT(*) FROM alert_records)
                     FROM orders o JOIN order_items i ON i.order_id=o.id WHERE o.id=2",
                    [],
                    |row| {
                        Ok((
                            row.get::<_, f64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, i64>(4)?,
                        ))
                    },
                )
                .unwrap()
        };
        let before = snapshot();
        evaluate_order(&connection, 2).unwrap();
        let after = snapshot();
        assert_eq!(before, after);
    }

    #[test]
    fn failed_evaluation_does_not_corrupt_order_transaction_state() {
        let mut connection = fixture();
        let before: f64 = connection
            .query_row("SELECT dose FROM order_items WHERE id=2", [], |row| {
                row.get(0)
            })
            .unwrap();
        connection.execute("DROP TABLE alert_settings", []).unwrap();

        assert!(matches!(
            evaluate_order(&connection, 2),
            Err(SafetyError::Sqlite(_))
        ));
        let transaction = connection.transaction().unwrap();
        transaction
            .execute("UPDATE order_items SET dose=999 WHERE id=2", [])
            .unwrap();
        transaction.rollback().unwrap();
        let after: f64 = connection
            .query_row("SELECT dose FROM order_items WHERE id=2", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(before, after);
    }
}
