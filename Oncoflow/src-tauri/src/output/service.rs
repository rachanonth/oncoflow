use rusqlite::TransactionBehavior;
use serde_json::json;
use std::collections::HashSet;
use thiserror::Error;

use crate::{
    auth::{audit, AuthError, AuthSession},
    db::{Database, DatabaseError},
};

use super::{repository, PreparationOutput};

#[derive(Debug, Error)]
pub(crate) enum OutputError {
    #[error("preparation task was not found")]
    TaskNotFound,
    #[error("only a verified preparation can produce a final label")]
    VerificationRequired,
    #[error("verified preparation provenance is incomplete")]
    IncompleteProvenance,
    #[error("the selected preparations do not belong to this order")]
    InvalidSelection,
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct OutputService<'a> {
    database: &'a Database,
    session: &'a AuthSession,
}

impl<'a> OutputService<'a> {
    pub(crate) fn new(database: &'a Database, session: &'a AuthSession) -> Self {
        Self { database, session }
    }

    pub(crate) fn get_preparation_output(
        &self,
        preparation_id: i64,
    ) -> Result<PreparationOutput, OutputError> {
        self.session.require_user()?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let output = ensure_snapshot(&transaction, preparation_id)?;
        transaction.commit()?;
        Ok(output)
    }

    pub(crate) fn get_order_outputs(
        &self,
        order_id: i64,
        preparation_ids: &[i64],
    ) -> Result<Vec<PreparationOutput>, OutputError> {
        self.session.require_user()?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let available = repository::list_order_preparation_ids(&transaction, order_id)?;
        if available.is_empty() {
            return Err(OutputError::TaskNotFound);
        }
        let selected = if preparation_ids.is_empty() {
            if available.len() as u64
                != repository::count_order_eligible_items(&transaction, order_id)?
            {
                return Err(OutputError::VerificationRequired);
            }
            available
        } else {
            let unique = preparation_ids.iter().copied().collect::<HashSet<_>>();
            if unique.len() != preparation_ids.len()
                || preparation_ids.iter().any(|id| *id <= 0)
                || preparation_ids.iter().any(|id| !available.contains(id))
            {
                return Err(OutputError::InvalidSelection);
            }
            preparation_ids.to_vec()
        };
        let outputs = selected
            .into_iter()
            .map(|id| ensure_snapshot(&transaction, id))
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit()?;
        Ok(outputs)
    }

    #[cfg(test)]
    pub(crate) fn record_label_print_request(
        &self,
        preparation_id: i64,
        transport: &'static str,
        renderer_version: &'static str,
    ) -> Result<PreparationOutput, OutputError> {
        self.record_label_print_request_at(preparation_id, transport, renderer_version, None)
    }

    pub(crate) fn record_rendered_label_print_request(
        &self,
        preparation_id: i64,
        transport: &'static str,
        renderer_version: &'static str,
        label_print_time: &str,
    ) -> Result<PreparationOutput, OutputError> {
        self.record_label_print_request_at(
            preparation_id,
            transport,
            renderer_version,
            Some(label_print_time),
        )
    }

    fn record_label_print_request_at(
        &self,
        preparation_id: i64,
        transport: &'static str,
        renderer_version: &'static str,
        rendered_label_print_time: Option<&str>,
    ) -> Result<PreparationOutput, OutputError> {
        let actor = self.session.require_user()?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let output = ensure_snapshot(&transaction, preparation_id)?;
        let label_print_time = rendered_label_print_time
            .unwrap_or(&output.label.print_time)
            .to_owned();
        let ordinal = output.print_request_count + 1;
        let event_type = if ordinal == 1 {
            "preparation_label_print_requested"
        } else {
            "preparation_label_reprint_requested"
        };
        audit::append_event(
            &transaction,
            Some(actor.id),
            event_type,
            "preparation_task",
            preparation_id,
            &json!({
                "preparation_task_id": preparation_id,
                "output_snapshot_id": output.label.snapshot_id,
                "template_version": output.label.template_version,
                "label_count": output.containers.len(),
                "request_ordinal": ordinal,
                "transport": transport,
                "renderer_version": renderer_version,
                "label_print_time": label_print_time
            }),
        )?;
        let output = repository::load_snapshot(&transaction, preparation_id)?
            .ok_or(OutputError::IncompleteProvenance)?;
        transaction.commit()?;
        Ok(output)
    }
}

fn ensure_snapshot(
    connection: &rusqlite::Connection,
    preparation_id: i64,
) -> Result<PreparationOutput, OutputError> {
    if let Some(output) = repository::load_snapshot(connection, preparation_id)? {
        return Ok(output);
    }
    let source =
        repository::load_source(connection, preparation_id)?.ok_or(OutputError::TaskNotFound)?;
    if source.state != "verified" {
        return Err(OutputError::VerificationRequired);
    }
    if source.verified_at.is_none() {
        return Err(OutputError::IncompleteProvenance);
    }
    repository::insert_snapshot(connection, &source)?;
    repository::load_snapshot(connection, preparation_id)?.ok_or(OutputError::IncompleteProvenance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthService, BootstrapUserInput};

    struct Fixture {
        _directory: tempfile::TempDir,
        database: Database,
        session: AuthSession,
        user_id: i64,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database = Database::initialize(directory.path().join("output.db")).unwrap();
            let session = AuthSession::default();
            let state = AuthService::new(&database, &session)
                .bootstrap(BootstrapUserInput {
                    username: "output.admin".into(),
                    display_name: "เภสัชกรทดสอบ".into(),
                    password: "Synthetic!Passphrase42".into(),
                })
                .unwrap();
            let user_id = state.current_user.unwrap().id;
            let connection = database.open().unwrap();
            connection
                .execute_batch(&format!(
                    "INSERT INTO regimens(id,legacy_regcode,regimen_name)
                     VALUES(1,'REG-SYN','สูตรทดสอบ');
                     INSERT INTO patients(
                       id,legacy_hn,title,first_name,last_name,regimen_id
                     ) VALUES(1,'SYN-HN-001','นาง','ทดสอบ','ระบบ',1);
                     UPDATE application_settings SET hospital_name='โรงพยาบาลทดสอบ' WHERE id=1;
                     INSERT INTO drugs(
                       id,legacy_dcode,drug_name,marker,inventory_enabled,
                       inventory_qty,inventory_min,warning,expiry_time,expiry_storage
                     ) VALUES
                       (1,'SYN-D1','ยาเคมีบำบัดทดสอบ',1,1,1,1,'คำเตือนทดสอบ','8 hr','ป้องกันแสง'),
                       (2,'SYN-D2','Synthetic pending drug',1,0,NULL,NULL,NULL,NULL,NULL),
                       (3,'SYN-D3','Synthetic prepared drug',1,0,NULL,NULL,NULL,NULL,NULL),
                       (4,'SYN-D4','Synthetic rollback drug',1,0,NULL,NULL,NULL,NULL,NULL);
                     INSERT INTO orders(
                       id,legacy_orderid,patient_id,regimen_id,order_time,oncoflow_created
                     ) VALUES(10,'OF-SYN-10',1,1,'2026-08-23T09:00:00',1);
                     INSERT INTO order_items(id,order_id,drug_id,dose,ordering_no)
                     VALUES(11,10,1,100.5,1),(12,10,2,10,2),(13,10,3,20,3),
                           (14,10,4,30,4);
                     INSERT INTO preparation_tasks(
                       id,source_order_id,source_order_item_id,preparation_date,drug_id,state,
                       snapshot_ordered_dose_text,snapshot_dose_unit_text,
                       snapshot_diluent_name,snapshot_diluent_volume_ml,
                       snapshot_route_name,snapshot_rate_text,snapshot_treatment_day,
                       snapshot_regimen_details,snapshot_drug_storage,
                       preparation_volume_ml,preparation_notes,prepared_at,verified_at,
                       prepared_by_user_id,verified_by_user_id
                     ) VALUES(
                       10,10,11,'2026-08-23',1,'verified','100.5','mg','สารละลายทดสอบ',100,
                       'IV','60 min','Day 1','คำแนะนำการเตรียม','เก็บแบบทดสอบ',
                       120,'บันทึกสังเคราะห์','2026-08-23T09:15:00',
                       '2026-08-23T09:20:00',{user_id},{user_id}),
                       (12,10,12,'2026-08-23',2,'pending','10','mg',NULL,NULL,NULL,NULL,NULL,
                        NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL),
                       (13,10,13,'2026-08-23',3,'prepared','20','mg',NULL,NULL,NULL,NULL,NULL,
                        NULL,NULL,NULL,NULL,'2026-08-23T09:15:00',NULL,{user_id},NULL),
                       (14,10,14,'2026-08-23',4,'verified','30','mg',NULL,NULL,NULL,NULL,NULL,
                        NULL,NULL,NULL,NULL,'2026-08-23T09:15:00',
                       '2026-08-23T09:20:00',{user_id},{user_id});
                     UPDATE preparation_tasks SET withdrawal_volume_ml='20' WHERE id=10;
                     INSERT INTO inventory_movements(
                       drug_id,movement_type,quantity_delta,occurred_at,actor_user_id
                     ) VALUES(1,'opening_balance',1,NULL,NULL);
                     INSERT INTO inventory_movements(
                       drug_id,movement_type,quantity_delta,occurred_at,actor_user_id,
                       reference_type,reference_id,preparation_task_id
                     ) VALUES(1,'preparation_issue',-3,'2026-08-23T09:20:00',{user_id},
                              'preparation_task','10',10);
                     INSERT INTO preparation_inventory_postings(
                       preparation_task_id,status,inventory_movement_id,
                       containers_required,balance_before,balance_after,
                       resulting_stock_state,calculation_status,
                       calculation_ruleset_version,calculation_rule_id,
                       workflow_rule_id,reason_code,actor_user_id,created_at
                     ) VALUES(
                       10,'posted',2,3,1,-2,'shortage','calculated',
                       'legacy-cytotoxic-v8',
                       'legacy-cytotoxic-v8:preparation-container-use',
                       'oncoflow-preparation-inventory-v1',
                       'supported_container_requirement',{user_id},
                       '2026-08-23T09:20:00');"
                ))
                .unwrap();
            Self {
                _directory: directory,
                database,
                session,
                user_id,
            }
        }

        fn service(&self) -> OutputService<'_> {
            OutputService::new(&self.database, &self.session)
        }
    }

    #[test]
    fn verified_preparation_creates_typed_deterministic_thai_snapshot() {
        let fixture = Fixture::new();
        let first = fixture.service().get_preparation_output(10).unwrap();
        assert_eq!(
            first.label.template_version,
            "oncoflow-preparation-label-v1"
        );
        assert_eq!(first.label.patient_identifier, "SYN-HN-001");
        assert_eq!(first.label.patient_name.as_deref(), Some("นาง ทดสอบ ระบบ"));
        assert_eq!(first.label.treatment_at.as_deref(), Some("2026-08-23"));
        assert_eq!(first.label.drug_name, "ยาเคมีบำบัดทดสอบ");
        assert_eq!(first.label.ordered_dose_text.as_deref(), Some("100.5"));
        assert_eq!(first.label.withdrawal_volume_ml.as_deref(), Some("20"));
        assert_eq!(first.label.hospital_name.as_deref(), Some("โรงพยาบาลทดสอบ"));
        assert_eq!(first.label.warning_text.as_deref(), Some("คำเตือนทดสอบ"));
        assert_eq!(first.label.expiry_time_text.as_deref(), Some("8 hr"));
        assert_eq!(first.label.expiry_storage_text.as_deref(), Some("ป้องกันแสง"));
        assert!(first.label.expiration_at.is_some());
        assert_eq!(first.summary.containers_required, Some(3));
        assert_eq!(first.summary.inventory_balance_after, Some(-2.0));
        assert_eq!(
            first.summary.inventory_stock_state.as_deref(),
            Some("shortage")
        );
        assert_eq!(first.print_request_count, 0);

        let connection = fixture.database.open().unwrap();
        connection
            .execute_batch(
                "UPDATE patients SET first_name='Changed';
                 UPDATE regimens SET regimen_name='Changed';
                 UPDATE drugs SET drug_name='Changed',warning='Changed',expiry_time='1 hr';
                 UPDATE application_settings SET hospital_name='Changed';",
            )
            .unwrap();
        let second = fixture.service().get_preparation_output(10).unwrap();
        let mut normalized_second = second.clone();
        normalized_second.label.print_time = first.label.print_time.clone();
        normalized_second.label.expiration_at = first.label.expiration_at.clone();
        assert_eq!(
            first, normalized_second,
            "the frozen clinical output must be deterministic"
        );
        let snapshot_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM preparation_output_snapshots WHERE preparation_task_id=10",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(snapshot_count, 1);
    }

    #[test]
    fn final_container_count_is_frozen_for_identical_numbered_labels() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        connection
            .execute(
                "UPDATE preparation_tasks SET final_container_count=2 WHERE id=10",
                [],
            )
            .unwrap();
        let first = fixture.service().get_preparation_output(10).unwrap();
        assert_eq!(first.containers.len(), 2);
        assert_eq!(first.containers[0].container_index, 1);
        assert_eq!(first.containers[1].container_index, 2);
        assert_eq!(first.label.ordered_dose_text.as_deref(), Some("100.5"));
        assert_eq!(first.label.final_volume_ml, Some(120.0));

        connection
            .execute(
                "UPDATE preparation_tasks SET final_container_count=1 WHERE id=10",
                [],
            )
            .unwrap();
        let second = fixture.service().get_preparation_output(10).unwrap();
        assert_eq!(
            first, second,
            "reprint allocation must come from the frozen snapshot"
        );
    }

    #[test]
    fn pending_and_prepared_tasks_cannot_masquerade_as_final_labels() {
        let fixture = Fixture::new();
        assert!(matches!(
            fixture.service().get_preparation_output(12),
            Err(OutputError::VerificationRequired)
        ));
        assert!(matches!(
            fixture.service().get_preparation_output(13),
            Err(OutputError::VerificationRequired)
        ));
        let count: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM preparation_output_snapshots",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn order_output_batch_is_order_scoped_deterministic_and_requires_checked_items() {
        let fixture = Fixture::new();
        let outputs = fixture.service().get_order_outputs(10, &[10, 14]).unwrap();
        assert_eq!(
            outputs
                .iter()
                .map(|output| output.label.preparation_id)
                .collect::<Vec<_>>(),
            vec![10, 14]
        );
        assert!(matches!(
            fixture.service().get_order_outputs(10, &[]),
            Err(OutputError::VerificationRequired)
        ));
        assert!(matches!(
            fixture.service().get_order_outputs(10, &[10, 999]),
            Err(OutputError::InvalidSelection)
        ));
        assert!(matches!(
            fixture.service().get_order_outputs(10, &[10, 10]),
            Err(OutputError::InvalidSelection)
        ));

        fixture
            .database
            .open()
            .unwrap()
            .execute(
                "UPDATE preparation_tasks
                 SET state='verified',
                     prepared_at=COALESCE(prepared_at,'2026-08-23T09:15:00'),
                     prepared_by_user_id=COALESCE(prepared_by_user_id,?1),
                     verified_at='2026-08-23T09:20:00',
                     verified_by_user_id=?1
                 WHERE source_order_id=10",
                [fixture.user_id],
            )
            .unwrap();
        let full_order = fixture.service().get_order_outputs(10, &[]).unwrap();
        assert_eq!(
            full_order
                .iter()
                .map(|output| output.label.preparation_id)
                .collect::<Vec<_>>(),
            vec![10, 12, 13, 14]
        );
    }

    #[test]
    fn print_and_reprint_are_authenticated_audited_and_non_mutating() {
        let fixture = Fixture::new();
        let unauthenticated = AuthSession::default();
        assert!(matches!(
            OutputService::new(&fixture.database, &unauthenticated).record_label_print_request(
                10,
                "synthetic_spooler",
                "synthetic-renderer-v1"
            ),
            Err(OutputError::Auth(AuthError::AuthenticationRequired))
        ));
        let connection = fixture.database.open().unwrap();
        let before: (i64, i64, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM patients),
                        (SELECT COUNT(*) FROM regimens),
                        (SELECT COUNT(*) FROM orders),
                        (SELECT COUNT(*) FROM preparation_tasks),
                        (SELECT COUNT(*) FROM inventory_movements),
                        (SELECT COUNT(*) FROM safety_acknowledgements)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();

        let first = fixture
            .service()
            .record_label_print_request(10, "synthetic_spooler", "synthetic-renderer-v1")
            .unwrap();
        let second = fixture
            .service()
            .record_label_print_request(10, "synthetic_spooler", "synthetic-renderer-v1")
            .unwrap();
        assert_eq!(first.label, second.label);
        assert_eq!(first.summary, second.summary);
        assert_eq!(first.print_request_count, 1);
        assert_eq!(second.print_request_count, 2);
        let after: (i64, i64, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM patients),
                        (SELECT COUNT(*) FROM regimens),
                        (SELECT COUNT(*) FROM orders),
                        (SELECT COUNT(*) FROM preparation_tasks),
                        (SELECT COUNT(*) FROM inventory_movements),
                        (SELECT COUNT(*) FROM safety_acknowledgements)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(before, after);
        let events = connection
            .prepare(
                "SELECT event_type,user_id,metadata_json FROM audit_events
                 WHERE event_type LIKE 'preparation_label_%' ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, "preparation_label_print_requested");
        assert_eq!(events[1].0, "preparation_label_reprint_requested");
        assert_eq!(events[0].1, fixture.user_id);
        assert!(events.iter().all(|event| {
            !event.2.contains("SYN-HN")
                && !event.2.contains("ยาเคมี")
                && !event.2.contains("ทดสอบ ระบบ")
        }));
    }

    #[test]
    fn reprint_expiration_is_anchored_to_the_first_rendered_label_time() {
        let fixture = Fixture::new();
        let first = fixture
            .service()
            .record_rendered_label_print_request(
                10,
                "synthetic_spooler",
                "synthetic-renderer-v1",
                "2026-08-23T10:00:00",
            )
            .unwrap();
        let reprint = fixture
            .service()
            .record_rendered_label_print_request(
                10,
                "synthetic_spooler",
                "synthetic-renderer-v1",
                "2026-08-23T12:30:00",
            )
            .unwrap();

        assert_eq!(first.label.print_time, "2026-08-23T10:00:00");
        assert_eq!(
            first.label.expiration_at.as_deref(),
            Some("2026-08-23T18:00:00")
        );
        assert_eq!(reprint.label.print_time, first.label.print_time);
        assert_eq!(reprint.label.expiration_at, first.label.expiration_at);
        assert_eq!(reprint.print_request_count, 2);
    }

    #[test]
    fn output_snapshot_is_append_only() {
        let fixture = Fixture::new();
        fixture.service().get_preparation_output(10).unwrap();
        let connection = fixture.database.open().unwrap();
        assert!(connection
            .execute(
                "UPDATE preparation_output_snapshots SET patient_name='changed' WHERE preparation_task_id=10",
                [],
            )
            .is_err());
        assert!(connection
            .execute(
                "DELETE FROM preparation_output_snapshots WHERE preparation_task_id=10",
                [],
            )
            .is_err());
    }

    #[test]
    fn audit_failure_rolls_back_first_snapshot_and_print_request() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER reject_output_audit
                 BEFORE INSERT ON audit_events
                 WHEN NEW.event_type LIKE 'preparation_label_%'
                 BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().record_label_print_request(
                14,
                "synthetic_spooler",
                "synthetic-renderer-v1"
            ),
            Err(OutputError::Sqlite(_))
        ));
        let snapshot_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM preparation_output_snapshots WHERE preparation_task_id=14",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(snapshot_count, 0);
    }
}
