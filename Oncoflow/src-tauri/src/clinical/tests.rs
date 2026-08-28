use std::collections::BTreeMap;

use rusqlite::{types::Value, Connection};
use serde::Deserialize;

use super::*;
use crate::db::Database;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReferenceFixture {
    rule: String,
    case: String,
    inputs: BTreeMap<String, Option<String>>,
    expected_status: String,
    expected_value: Option<String>,
    source: String,
    evidence_note: String,
    confidence: String,
}

#[test]
fn reference_fixture_corpus_matches_recovered_legacy_rules() {
    let fixtures: Vec<ReferenceFixture> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/clinical/legacy_cytotoxic_v8.json"
    ))
    .expect("clinical fixture corpus should be valid JSON");

    assert!(
        fixtures.len() >= 40,
        "parity matrix should remain comprehensive"
    );
    for fixture in fixtures {
        assert!(!fixture.source.trim().is_empty());
        assert!(!fixture.evidence_note.trim().is_empty());
        let result = run_fixture(&fixture);
        assert_eq!(result.ruleset, LEGACY_RULESET, "{}: ruleset", fixture.case);
        assert_eq!(
            status_name(result.status),
            fixture.expected_status,
            "{}: status",
            fixture.case
        );
        assert_eq!(
            result.value, fixture.expected_value,
            "{}: value",
            fixture.case
        );
        assert_eq!(
            confidence_name(result.confidence),
            fixture.confidence,
            "{}: confidence",
            fixture.case
        );
        assert!(!result.trace.is_empty(), "{}: trace", fixture.case);
    }
}

#[test]
fn repeated_calculations_are_deterministic_and_versioned() {
    let first = standard_dose(Some("10-20"), Some("1.25"));
    let second = standard_dose(Some("10-20"), Some("1.25"));

    assert_eq!(first, second);
    assert_eq!(first.ruleset, "legacy-cytotoxic-v8");
    assert_eq!(first.rule_id, "StandardDose");
    assert_eq!(first.value.as_deref(), Some("12 - 25"));
}

#[test]
fn clinical_calculations_cannot_mutate_clinical_records() {
    let directory = tempfile::tempdir().expect("temporary directory should be created");
    let database = Database::initialize(directory.path().join("oncoflow.db"))
        .expect("synthetic database should initialize");
    let connection = database.open().expect("synthetic database should open");
    seed_synthetic_clinical_records(&connection);
    let before = clinical_record_snapshot(&connection);

    let results = [
        standard_dose(Some("10-20"), Some("1.25")),
        anc_cal(Some("5000"), Some("50")),
        anc_grade(Some("1500")),
        platelet(Some("100000")),
        lab_min_max(Some("12.5")),
        fix_number(Some("1.5")),
    ];
    assert!(results
        .iter()
        .all(|result| result.status == CalculationStatus::Calculated));

    let after = clinical_record_snapshot(&connection);
    assert_eq!(before, after);
}

fn run_fixture(fixture: &ReferenceFixture) -> ClinicalCalculationResult<String> {
    let value = |name: &str| fixture.inputs.get(name).and_then(|value| value.as_deref());
    match fixture.rule.as_str() {
        "StandardDose" => standard_dose(value("dose"), value("surface")),
        "ANCCal" => anc_cal(value("wbc"), value("neutrophil")),
        "ANCGrade" => anc_grade(value("anc")),
        "Platelet" => platelet(value("rawValue")),
        "LabMinMax" => lab_min_max(value("number")),
        "FixNumber" => fix_number(value("number")),
        unknown => panic!("unknown fixture rule {unknown}"),
    }
}

fn status_name(status: CalculationStatus) -> &'static str {
    match status {
        CalculationStatus::Calculated => "calculated",
        CalculationStatus::Unavailable => "unavailable",
        CalculationStatus::Unsupported => "unsupported",
        CalculationStatus::LegacyError => "legacy_error",
    }
}

fn confidence_name(confidence: EvidenceConfidence) -> &'static str {
    match confidence {
        EvidenceConfidence::Confirmed => "CONFIRMED",
        EvidenceConfidence::PartiallyConfirmed => "PARTIALLY_CONFIRMED",
        EvidenceConfidence::Unknown => "UNKNOWN",
    }
}

fn seed_synthetic_clinical_records(connection: &Connection) {
    connection
        .execute_batch(
            "INSERT INTO patients(id, legacy_hn, first_name) VALUES(1, 'SYN-HN', 'Synthetic');
             INSERT INTO drugs(id, legacy_dcode, drug_name) VALUES(1, 'SYN-D', 'Synthetic drug');
             INSERT INTO regimens(id, legacy_regcode, regimen_name) VALUES(1, 'SYN-R', 'Synthetic regimen');
             INSERT INTO regimen_groups(id, legacy_code, regimen_id) VALUES(1, 'SYN-G', 1);
             INSERT INTO regimen_items(
               id, regimen_group_id, drug_id, dose, ordering_no, legacy_dose_text
             ) VALUES(1, 1, 1, 12.5, 1, 'legacy raw dose');
             INSERT INTO orders(
               id, legacy_orderid, patient_id, regimen_id, note, oncoflow_created
             ) VALUES(1, 'SYN-O', 1, 1, 'synthetic note', 0);
             INSERT INTO order_items(
               id, order_id, drug_id, dose, ordering_no, legacy_dose_text,
               source_regimen_item_id, regimen_dose_text
             ) VALUES(1, 1, 1, 12.5, 1, 'historical raw dose', 1, 'legacy raw dose');",
        )
        .expect("synthetic clinical rows should insert");
}

fn clinical_record_snapshot(connection: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    [
        "patients",
        "drugs",
        "regimens",
        "regimen_groups",
        "regimen_items",
        "orders",
        "order_items",
    ]
    .into_iter()
    .map(|table| (table.to_owned(), table_rows(connection, table)))
    .collect()
}

fn table_rows(connection: &Connection, table: &str) -> Vec<Vec<Value>> {
    let sql = format!("SELECT * FROM {table} ORDER BY id");
    let mut statement = connection
        .prepare(&sql)
        .expect("trusted test table should prepare");
    let column_count = statement.column_count();
    statement
        .query_map([], |row| {
            (0..column_count)
                .map(|index| row.get::<_, Value>(index))
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("snapshot query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("snapshot rows should decode")
}
