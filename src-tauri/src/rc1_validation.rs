//! Release Candidate 1 integration checks.
//!
//! These tests use isolated synthetic databases only. They deliberately exercise
//! the accepted Milestone 1–15 boundaries without adding a runtime command or a
//! new clinical rule.

use std::path::Path;

use rusqlite::{params, Connection};

use crate::{
    auth::{AuthService, AuthSession, BootstrapUserInput, LoginInput},
    db::{read_schema_version, validate_connection, Database},
    drug::{DrugListRequest, DrugService},
    hardware::{
        render_preparation_label, LabelPrinterConfig, PrinterLanguage, LABEL_RENDERER_VERSION,
    },
    order::{OrderInput, OrderService},
    output::OutputService,
    patient::{PatientListRequest, PatientService},
    preparation::{
        PreparationError, PreparationInventoryPostingStatus, PreparationIssueStockState,
        PreparationService, PreparationState,
    },
    preparation_calc::PreparationCalculationStatus,
    recovery::{RecoveryService, RestoreInput, StartupState},
    regimen::{RegimenListRequest, RegimenService},
    safety::{SafetyFindingStatus, SafetyService},
};

const PASSWORD: &str = "RC1 synthetic pharmacist password 42!";

#[derive(Clone, Copy)]
struct Scenario {
    ordered_dose: &'static str,
    ordered_unit: &'static str,
    presentation_unit: &'static str,
    opening_stock: f64,
    concentration_warning: bool,
}

struct RcFixture {
    directory: tempfile::TempDir,
    database: Database,
    session: AuthSession,
}

impl RcFixture {
    fn new(scenario: Scenario) -> Self {
        let directory = tempfile::tempdir().expect("RC1 temporary directory");
        let database =
            Database::initialize(directory.path().join("oncoflow.db")).expect("RC1 database");
        let connection = database.open().expect("RC1 connection");
        let dose = scenario
            .ordered_dose
            .parse::<f64>()
            .expect("synthetic dose is numeric");

        connection
            .execute(
                "INSERT INTO patients(id,legacy_hn,title,first_name,last_name)
                 VALUES(1,'RC1-TH-001','นาง','สายรุ้ง','ทดสอบ')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO units(id,legacy_unitcode,unit_name) VALUES(1,'MG',?1)",
                [scenario.presentation_unit],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO diluents(id,legacy_dilcode,diluent_name,volume_ml)
                 VALUES(1,'DIL-RC1','สารละลายทดสอบ',100)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO routes(id,legacy_rcode,route_name)
                 VALUES(1,'IV-RC1','ทางหลอดเลือดดำ')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO regimens(id,legacy_regcode,regimen_name)
                 VALUES(1,'REG-RC1','สูตรยาเคมีบำบัดทดสอบ')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO regimen_groups(id,legacy_code,regimen_id,note)
                 VALUES(1,'GROUP-RC1',1,'กลุ่มสังเคราะห์')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO drugs(
                    id,legacy_dcode,drug_name,unit_id,dose_per_pack,volume_per_pack_ml,
                    package,detail,storage,marker,inventory_enabled,inventory_min,inventory_max,
                    max_dilution_alert,max_dilution_hard
                 ) VALUES(1,'DRUG-RC1','ยาสังเคราะห์สำหรับเตรียม',1,50,10,
                          'ขวด','คำแนะนำการเตรียม','เก็บที่อุณหภูมิทดสอบ',1,1,1,20,?1,0.5)",
                [i64::from(scenario.concentration_warning)],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO regimen_items(
                    id,regimen_group_id,drug_id,dose,legacy_dose_text,unit_text,
                    details,start_day,ordering_no,default_diluent_id,default_route_id,default_rate
                 ) VALUES(1,1,1,?1,?2,?3,'วิธีเตรียมสังเคราะห์',1,1,1,1,'60 นาที')",
                params![dose, scenario.ordered_dose, scenario.ordered_unit],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO inventory_movements(
                    drug_id,movement_type,quantity_delta,reference_type,reference_id,note
                 ) VALUES(1,'opening_balance',?1,'rc1_fixture','opening','Synthetic opening balance')",
                [scenario.opening_stock],
            )
            .unwrap();
        drop(connection);

        let session = AuthSession::default();
        AuthService::new(&database, &session)
            .bootstrap(BootstrapUserInput {
                username: "rc1.pharmacist".into(),
                display_name: "เภสัชกรอาร์ซีหนึ่ง".into(),
                password: PASSWORD.into(),
            })
            .expect("RC1 synthetic bootstrap");

        Self {
            directory,
            database,
            session,
        }
    }

    fn create_order(&self) -> i64 {
        OrderService::new(&self.database)
            .create_from_regimen(OrderInput {
                patient_id: 1,
                regimen_id: Some(1),
                order_time: Some("2026-08-23T09:30".into()),
                note: Some("บันทึกสังเคราะห์สำหรับ RC1".into()),
                assigned_preparer_user_id: Some(1),
                ..OrderInput::default()
            })
            .expect("RC1 regimen-derived order")
            .id
    }

    fn initialize(&self, order_id: i64) -> (i64, i64) {
        let workspace = PreparationService::new(&self.database, &self.session)
            .initialize(order_id)
            .expect("RC1 preparation initialization");
        assert_eq!(workspace.items.len(), 1);
        let item = &workspace.items[0];
        (
            item.order_item_id,
            item.task.as_ref().expect("initialized task").id,
        )
    }

    fn acknowledge_required_findings(&self, order_id: i64) {
        let service = PreparationService::new(&self.database, &self.session);
        let workspace = service.get_workspace(order_id).unwrap();
        for finding_id in workspace
            .safety
            .findings
            .iter()
            .filter(|finding| finding.acknowledgement_required)
            .map(|finding| finding.id.clone())
            .collect::<Vec<_>>()
        {
            service
                .acknowledge_safety_finding(order_id, finding_id)
                .expect("RC1 warning acknowledgement");
        }
    }

    fn complete_preparation(&self, order_id: i64, task_id: i64) {
        let service = PreparationService::new(&self.database, &self.session);
        self.acknowledge_required_findings(order_id);
        service.mark_prepared(task_id).expect("mark prepared");
        let verified = service.verify(task_id).expect("verify preparation");
        assert_eq!(verified.state, PreparationState::Verified);
    }

    fn backup_directory(&self) -> &Path {
        self.directory.path()
    }
}

#[test]
fn rc1_case_a_normal_stock_runs_once_from_regimen_to_label_backup() {
    let fixture = RcFixture::new(Scenario {
        ordered_dose: "100",
        ordered_unit: "mg",
        presentation_unit: "mg",
        opening_stock: 5.0,
        concentration_warning: false,
    });
    let order_id = fixture.create_order();
    let order_before = OrderService::new(&fixture.database).get(order_id).unwrap();
    let safety = SafetyService::new(&fixture.database);
    assert_eq!(
        safety.evaluate_order(order_id).unwrap(),
        safety.evaluate_order(order_id).unwrap()
    );

    let (_, task_id) = fixture.initialize(order_id);
    let preview = PreparationService::new(&fixture.database, &fixture.session)
        .get_workspace(order_id)
        .unwrap();
    let calculation = &preview.items[0].calculation;
    assert_eq!(calculation.status, PreparationCalculationStatus::Calculated);
    assert_eq!(calculation.withdrawal_volume_ml.as_deref(), Some("20"));
    assert_eq!(calculation.containers_required.as_deref(), Some("2"));
    assert_eq!(
        calculation.inventory_projection.projected_stock.as_deref(),
        Some("3")
    );

    fixture.complete_preparation(order_id, task_id);
    let verified = PreparationService::new(&fixture.database, &fixture.session)
        .verify(task_id)
        .unwrap();
    let posting = verified.inventory_posting.expect("automatic issue");
    assert_eq!(posting.status, PreparationInventoryPostingStatus::Posted);
    assert_eq!(posting.containers_required.as_deref(), Some("2"));
    assert_eq!(posting.balance_before.as_deref(), Some("5.0"));
    assert_eq!(posting.balance_after.as_deref(), Some("3.0"));

    let retry_session = AuthSession::default();
    AuthService::new(&fixture.database, &retry_session)
        .login(LoginInput {
            username: "rc1.pharmacist".into(),
            password: PASSWORD.into(),
        })
        .unwrap();
    PreparationService::new(&fixture.database, &retry_session)
        .verify(task_id)
        .unwrap();

    let output_service = OutputService::new(&fixture.database, &fixture.session);
    let preview_output = output_service.get_preparation_output(task_id).unwrap();
    let printed = output_service
        .record_label_print_request(task_id, "rc1-print-simulation", LABEL_RENDERER_VERSION)
        .unwrap();
    let reprinted = output_service
        .record_label_print_request(task_id, "rc1-print-simulation", LABEL_RENDERER_VERSION)
        .unwrap();
    assert_eq!(preview_output.label, printed.label);
    assert_eq!(printed.label, reprinted.label);
    assert_eq!(reprinted.print_request_count, 2);

    let connection = fixture.database.open().unwrap();
    let issues: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM inventory_movements
             WHERE movement_type='preparation_issue' AND preparation_task_id=?1",
            [task_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(issues, 1);
    assert!(connection
        .execute(
            "UPDATE preparation_output_snapshots SET drug_name='forbidden' WHERE preparation_task_id=?1",
            [task_id],
        )
        .is_err());
    assert!(connection
        .execute(
            "DELETE FROM inventory_movements WHERE movement_type='preparation_issue'",
            [],
        )
        .is_err());
    assert!(connection
        .execute("UPDATE audit_events SET event_type='forbidden'", [])
        .is_err());
    drop(connection);

    let order_after = OrderService::new(&fixture.database).get(order_id).unwrap();
    assert_eq!(
        order_before.items[0].dose_text,
        order_after.items[0].dose_text
    );
    assert_eq!(order_before.items[0].dose, order_after.items[0].dose);

    let startup = StartupState::ready();
    let backup = RecoveryService::new(&fixture.database, &fixture.session, &startup)
        .create_backup(fixture.backup_directory())
        .unwrap();
    let backup_connection = Connection::open(&backup.location).unwrap();
    validate_connection(&backup_connection).unwrap();
    assert_eq!(
        read_schema_version(&backup_connection).unwrap(),
        Some(crate::db::LATEST_SCHEMA_VERSION)
    );
    assert_eq!(backup.foreign_key_violations, 0);
    assert!(Path::new(&backup.manifest_location).is_file());
}

#[test]
fn rc1_case_b_shortage_is_non_blocking_and_printable() {
    let fixture = RcFixture::new(Scenario {
        ordered_dose: "150",
        ordered_unit: "mg",
        presentation_unit: "mg",
        opening_stock: 1.0,
        concentration_warning: false,
    });
    let order_id = fixture.create_order();
    let (_, task_id) = fixture.initialize(order_id);
    let workspace = PreparationService::new(&fixture.database, &fixture.session)
        .get_workspace(order_id)
        .unwrap();
    assert_eq!(
        workspace.items[0]
            .calculation
            .containers_required
            .as_deref(),
        Some("3")
    );
    assert_eq!(
        workspace.items[0]
            .calculation
            .inventory_projection
            .projected_stock
            .as_deref(),
        Some("-2")
    );

    fixture.complete_preparation(order_id, task_id);
    let verified = PreparationService::new(&fixture.database, &fixture.session)
        .verify(task_id)
        .unwrap();
    let posting = verified.inventory_posting.unwrap();
    assert_eq!(posting.balance_after.as_deref(), Some("-2.0"));
    assert_eq!(
        posting.resulting_stock_state,
        Some(PreparationIssueStockState::Shortage)
    );
    let output = OutputService::new(&fixture.database, &fixture.session)
        .record_label_print_request(task_id, "rc1-print-simulation", LABEL_RENDERER_VERSION)
        .unwrap();
    assert_eq!(
        output.summary.inventory_stock_state.as_deref(),
        Some("shortage")
    );
}

#[test]
fn rc1_case_c_unsupported_units_verify_without_guessed_inventory_issue() {
    let fixture = RcFixture::new(Scenario {
        ordered_dose: "100",
        ordered_unit: "mcg",
        presentation_unit: "mg",
        opening_stock: 5.0,
        concentration_warning: false,
    });
    let order_id = fixture.create_order();
    let (_, task_id) = fixture.initialize(order_id);
    let workspace = PreparationService::new(&fixture.database, &fixture.session)
        .get_workspace(order_id)
        .unwrap();
    assert_eq!(
        workspace.items[0].calculation.status,
        PreparationCalculationStatus::Unsupported
    );
    assert!(workspace.items[0].calculation.containers_required.is_none());

    fixture.complete_preparation(order_id, task_id);
    let verified = PreparationService::new(&fixture.database, &fixture.session)
        .verify(task_id)
        .unwrap();
    let posting = verified.inventory_posting.unwrap();
    assert_eq!(
        posting.status,
        PreparationInventoryPostingStatus::ManualReconciliationRequired
    );
    assert!(posting.inventory_movement_id.is_none());
    let issue_count: i64 = fixture
        .database
        .open()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM inventory_movements WHERE movement_type='preparation_issue'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(issue_count, 0);
}

#[test]
fn rc1_case_d_warning_requires_review_and_changed_input_invalidates_fingerprint() {
    let fixture = RcFixture::new(Scenario {
        ordered_dose: "100",
        ordered_unit: "mg.",
        presentation_unit: "mg.",
        opening_stock: 5.0,
        concentration_warning: true,
    });
    let order_id = fixture.create_order();
    let (order_item_id, task_id) = fixture.initialize(order_id);
    let safety_service = SafetyService::new(&fixture.database);
    let first = safety_service.evaluate_order(order_id).unwrap();
    assert_eq!(first, safety_service.evaluate_order(order_id).unwrap());
    let warning = first
        .findings
        .iter()
        .find(|finding| finding.rule_id == "legacy.max_dilution_concentration")
        .expect("confirmed concentration finding");
    assert_eq!(warning.status, SafetyFindingStatus::Triggered);
    assert!(warning.acknowledgement_required);
    let original_fingerprint = warning.fingerprint.clone();

    let preparation = PreparationService::new(&fixture.database, &fixture.session);
    preparation.mark_prepared(task_id).unwrap();
    assert!(matches!(
        preparation.verify(task_id),
        Err(PreparationError::SafetyReviewRequired { .. })
    ));
    let acknowledged = preparation
        .acknowledge_safety_finding(order_id, warning.id.clone())
        .unwrap();
    assert!(acknowledged
        .safety_acknowledgements
        .iter()
        .any(|value| value.finding_fingerprint == original_fingerprint));

    let dose_before_evaluation: String = fixture
        .database
        .open()
        .unwrap()
        .query_row(
            "SELECT legacy_dose_text FROM order_items WHERE id=?1",
            [order_item_id],
            |row| row.get(0),
        )
        .unwrap();
    safety_service.evaluate_order(order_id).unwrap();
    let dose_after_evaluation: String = fixture
        .database
        .open()
        .unwrap()
        .query_row(
            "SELECT legacy_dose_text FROM order_items WHERE id=?1",
            [order_item_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dose_before_evaluation, dose_after_evaluation);

    fixture
        .database
        .open()
        .unwrap()
        .execute(
            "UPDATE order_items SET dose=125,legacy_dose_text='125' WHERE id=?1",
            [order_item_id],
        )
        .unwrap();
    let changed = safety_service.evaluate_order(order_id).unwrap();
    let changed_warning = changed
        .findings
        .iter()
        .find(|finding| finding.rule_id == "legacy.max_dilution_concentration")
        .unwrap();
    assert_ne!(changed_warning.fingerprint, original_fingerprint);
    assert!(!acknowledged
        .safety_acknowledgements
        .iter()
        .any(|value| value.finding_fingerprint == changed_warning.fingerprint));
}

#[test]
fn rc1_case_e_thai_search_persistence_output_and_rasterization_are_lossless() {
    let fixture = RcFixture::new(Scenario {
        ordered_dose: "100",
        ordered_unit: "mg",
        presentation_unit: "mg",
        opening_stock: 5.0,
        concentration_warning: false,
    });
    assert_eq!(
        PatientService::new(&fixture.database)
            .list(PatientListRequest {
                search: Some("สายรุ้ง".into()),
                ..PatientListRequest::default()
            })
            .unwrap()
            .total,
        1
    );
    assert_eq!(
        DrugService::new(&fixture.database)
            .list(DrugListRequest {
                search: Some("ยาสังเคราะห์".into()),
                ..DrugListRequest::default()
            })
            .unwrap()
            .total,
        1
    );
    assert_eq!(
        RegimenService::new(&fixture.database)
            .list(RegimenListRequest {
                search: Some("เคมีบำบัด".into()),
                ..RegimenListRequest::default()
            })
            .unwrap()
            .total,
        1
    );

    let order_id = fixture.create_order();
    let (_, task_id) = fixture.initialize(order_id);
    fixture.complete_preparation(order_id, task_id);
    let output = OutputService::new(&fixture.database, &fixture.session)
        .get_preparation_output(task_id)
        .unwrap();
    let serialized = serde_json::to_string(&output).unwrap();
    assert!(serialized.contains("สายรุ้ง"));
    assert!(serialized.contains("ยาสังเคราะห์"));
    assert!(serialized.contains("สูตรยาเคมีบำบัด"));
    assert!(!serialized.contains('\u{fffd}'));

    let config = LabelPrinterConfig {
        spooler_name: "RC1 simulation only".into(),
        language: PrinterLanguage::Tspl,
        width_mm: 100.0,
        height_mm: 70.0,
        dpi: 203,
        gap_mm: 3.0,
        preprint_header_spacing_mm: 5.0,
        font_sizes: Default::default(),
    };
    let first = render_preparation_label(&output, &config).unwrap();
    let second = render_preparation_label(&output, &config).unwrap();
    assert_eq!(first, second);
    assert!(first.len() > 1_000);
}

#[derive(Debug, PartialEq, Eq)]
struct RestoredDomainSnapshot {
    username_and_hash: String,
    order_count: i64,
    preparation_count: i64,
    movement_count: i64,
    movement_sum: String,
    acknowledgement_fingerprints: Vec<String>,
    output_snapshot_count: i64,
}

fn restored_domain_snapshot(connection: &Connection) -> RestoredDomainSnapshot {
    RestoredDomainSnapshot {
        username_and_hash: connection
            .query_row(
                "SELECT username || '|' || password_hash FROM users ORDER BY id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        order_count: connection
            .query_row("SELECT COUNT(*) FROM orders", [], |row| row.get(0))
            .unwrap(),
        preparation_count: connection
            .query_row("SELECT COUNT(*) FROM preparation_tasks", [], |row| {
                row.get(0)
            })
            .unwrap(),
        movement_count: connection
            .query_row("SELECT COUNT(*) FROM inventory_movements", [], |row| {
                row.get(0)
            })
            .unwrap(),
        movement_sum: connection
            .query_row(
                "SELECT CAST(COALESCE(SUM(quantity_delta),0) AS TEXT) FROM inventory_movements",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        acknowledgement_fingerprints: connection
            .prepare("SELECT finding_fingerprint FROM safety_acknowledgements ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap(),
        output_snapshot_count: connection
            .query_row(
                "SELECT COUNT(*) FROM preparation_output_snapshots",
                [],
                |row| row.get(0),
            )
            .unwrap(),
    }
}

fn audit_rows(connection: &Connection) -> Vec<(i64, String, String, String, String)> {
    connection
        .prepare(
            "SELECT id,event_type,entity_type,entity_id,metadata_json
             FROM audit_events ORDER BY id",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
fn rc1_case_f_backup_restore_recovers_complete_domain_and_authoritative_users() {
    let fixture = RcFixture::new(Scenario {
        ordered_dose: "100",
        ordered_unit: "mg.",
        presentation_unit: "mg.",
        opening_stock: 5.0,
        concentration_warning: true,
    });
    let order_id = fixture.create_order();
    let (_, task_id) = fixture.initialize(order_id);
    fixture.complete_preparation(order_id, task_id);
    OutputService::new(&fixture.database, &fixture.session)
        .get_preparation_output(task_id)
        .unwrap();

    let startup = StartupState::ready();
    let recovery = RecoveryService::new(&fixture.database, &fixture.session, &startup);
    let backup = recovery.create_backup(fixture.backup_directory()).unwrap();
    let backup_connection = Connection::open(&backup.location).unwrap();
    let expected = restored_domain_snapshot(&backup_connection);
    let expected_audit = audit_rows(&backup_connection);
    assert_eq!(expected.acknowledgement_fingerprints.len(), 1);
    assert_eq!(expected.output_snapshot_count, 1);
    drop(backup_connection);

    let connection = fixture.database.open().unwrap();
    connection
        .execute(
            "UPDATE patients SET first_name='mutated after backup' WHERE id=1",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO users(
                legacy_user,username,display_name,role,password_hash,active,
                credential_kind,created_at,updated_at
             ) SELECT 'MUTATED','mutated.user','mutated user','user',password_hash,1,
                      'argon2id',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP
               FROM users WHERE id=1",
            [],
        )
        .unwrap();
    drop(connection);

    let preflight = recovery.preflight_restore(&backup.location).unwrap();
    let restored = recovery
        .restore(RestoreInput {
            backup_path: preflight.location,
            expected_sha256: preflight.sha256,
            confirmation_token: preflight.confirmation_token,
            confirmed: true,
        })
        .unwrap();
    assert!(restored.session_cleared);
    assert!(Path::new(&restored.recovery_backup_location).is_file());

    let active = fixture.database.open().unwrap();
    validate_connection(&active).unwrap();
    assert_eq!(
        read_schema_version(&active).unwrap(),
        Some(crate::db::LATEST_SCHEMA_VERSION)
    );
    assert_eq!(restored_domain_snapshot(&active), expected);
    let restored_audit = audit_rows(&active);
    let preserved = restored_audit
        .iter()
        .filter(|row| row.1 != "database_restore_completed")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(preserved, expected_audit);
    assert_eq!(
        restored_audit
            .iter()
            .filter(|row| row.1 == "database_restore_completed")
            .count(),
        1
    );
    assert!(fixture.session.current_user().unwrap().is_none());
}
