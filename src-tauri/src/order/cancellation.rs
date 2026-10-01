use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::{repository, OrderDetail, OrderError, OrderService};
use crate::auth::audit;

#[cfg(test)]
pub(crate) fn remove_schema_for_migration_test(connection: &Connection) {
    connection
        .execute_batch(
            "DROP TRIGGER order_cancellations_no_update;
         DROP TRIGGER order_cancellations_no_delete;
         DROP TRIGGER cancelled_order_no_update;
         DROP TRIGGER cancelled_item_no_update;
         DROP TRIGGER cancelled_task_no_update;
         DROP TRIGGER cancelled_task_no_delete;
         DROP TRIGGER cancelled_item_no_delete;
         DROP TRIGGER cancelled_order_no_item;
         DROP TRIGGER cancelled_source_no_preparation;
         ALTER TABLE orders DROP COLUMN cancellation_id;
         ALTER TABLE order_items DROP COLUMN cancellation_id;
         ALTER TABLE preparation_tasks DROP COLUMN cancellation_id;
         ALTER TABLE preparation_tasks DROP COLUMN cancelled_after_preparation;
         DROP TABLE order_cancellations;",
        )
        .unwrap();
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancellationTask {
    pub id: i64,
    pub item_id: i64,
    pub drug_name: String,
    pub preparation_date: String,
    pub state: String,
    pub final_container_count: i64,
    pub inventory_status: Option<String>,
    pub printed: bool,
    pub actually_prepared: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancellationEvent {
    pub id: i64,
    pub item_id: Option<i64>,
    pub reason: String,
    pub actor_name: String,
    pub occurred_at: String,
    pub tasks: Vec<CancellationTask>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancellationPreview {
    pub revision: String,
    pub order: OrderDetail,
    pub item_id: Option<i64>,
    pub tasks: Vec<CancellationTask>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancellationDecision {
    pub task_id: i64,
    pub actually_prepared: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancelOrderInput {
    pub item_id: Option<i64>,
    pub revision: String,
    pub reason: String,
    pub tasks: Vec<CancellationDecision>,
    pub acknowledged: bool,
}

fn validation(message: &str) -> OrderError {
    OrderError::Validation {
        field: "cancellation",
        message: message.into(),
    }
}

// Hash complete rows, not second-resolution timestamps. The token contains no PHI.
fn fingerprint(connection: &Connection, order_id: i64) -> rusqlite::Result<String> {
    let mut hash = Sha256::new();
    for sql in [
        "SELECT * FROM orders WHERE id=?1",
        "SELECT * FROM order_items WHERE order_id=?1 ORDER BY id",
        "SELECT * FROM preparation_tasks WHERE source_order_id=?1 ORDER BY id",
        "SELECT p.* FROM preparation_inventory_postings p JOIN preparation_tasks t ON t.id=p.preparation_task_id WHERE t.source_order_id=?1 ORDER BY p.id",
        "SELECT a.* FROM audit_events a JOIN preparation_tasks t ON CAST(t.id AS TEXT)=a.entity_id WHERE a.entity_type='preparation_task' AND t.source_order_id=?1 ORDER BY a.id",
    ] {
        hash.update(sql.as_bytes());
        let mut statement = connection.prepare(sql)?;
        let count = statement.column_count();
        let mut rows = statement.query([order_id])?;
        while let Some(row) = rows.next()? {
            for index in 0..count {
                let value: rusqlite::types::Value = row.get(index)?;
                let bytes = format!("{value:?}");
                hash.update((bytes.len() as u64).to_le_bytes());
                hash.update(bytes.as_bytes());
            }
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn tasks(
    connection: &Connection,
    order_id: i64,
    item_id: Option<i64>,
    event_id: Option<i64>,
) -> rusqlite::Result<Vec<CancellationTask>> {
    connection
        .prepare(
            "SELECT t.id,t.source_order_item_id,d.drug_name,t.preparation_date,t.state,
                t.final_container_count,p.status,
                EXISTS(SELECT 1 FROM audit_events a WHERE a.entity_type='preparation_task'
                  AND a.entity_id=CAST(t.id AS TEXT) AND a.event_type IN
                    ('preparation_label_print_requested','preparation_label_reprint_requested')),
                t.cancelled_after_preparation
         FROM preparation_tasks t JOIN drugs d ON d.id=t.drug_id
         LEFT JOIN preparation_inventory_postings p ON p.preparation_task_id=t.id
         WHERE t.source_order_id=?1 AND (?2 IS NULL OR t.source_order_item_id=?2)
           AND ((?3 IS NULL AND t.cancellation_id IS NULL) OR t.cancellation_id=?3)
         ORDER BY t.preparation_date,t.id",
        )?
        .query_map(params![order_id, item_id, event_id], |row| {
            Ok(CancellationTask {
                id: row.get(0)?,
                item_id: row.get(1)?,
                drug_name: row.get(2)?,
                preparation_date: row.get(3)?,
                state: row.get(4)?,
                final_container_count: row.get(5)?,
                inventory_status: row.get(6)?,
                printed: row.get(7)?,
                actually_prepared: row.get(8)?,
            })
        })?
        .collect()
}

pub(super) fn history(
    connection: &Connection,
    order_id: i64,
) -> rusqlite::Result<Vec<CancellationEvent>> {
    let mut events = connection
        .prepare(
            "SELECT c.id,c.item_id,c.reason,COALESCE(u.display_name,u.username),c.occurred_at
         FROM order_cancellations c JOIN users u ON u.id=c.actor_user_id
         WHERE c.order_id=?1 ORDER BY c.id DESC",
        )?
        .query_map([order_id], |row| {
            Ok(CancellationEvent {
                id: row.get(0)?,
                item_id: row.get(1)?,
                reason: row.get(2)?,
                actor_name: row.get(3)?,
                occurred_at: row.get(4)?,
                tasks: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for event in &mut events {
        event.tasks = tasks(connection, order_id, event.item_id, Some(event.id))?;
    }
    Ok(events)
}

fn preview(
    connection: &Connection,
    order_id: i64,
    item_id: Option<i64>,
) -> Result<CancellationPreview, OrderError> {
    let order = repository::get_order(connection, order_id)?.ok_or(OrderError::OrderNotFound)?;
    if !order.editable {
        return Err(validation(
            "รายการนี้เป็นข้อมูลย้อนหลังหรือยกเลิกแล้ว กรุณาโหลดข้อมูลใหม่",
        ));
    }
    if let Some(id) = item_id {
        let item = order
            .items
            .iter()
            .find(|item| item.id == id)
            .ok_or(OrderError::ItemNotFound)?;
        if item.cancelled {
            return Err(validation("รายการยานี้ถูกยกเลิกแล้ว"));
        }
    }
    Ok(CancellationPreview {
        revision: fingerprint(connection, order_id)?,
        order,
        item_id,
        tasks: tasks(connection, order_id, item_id, None)?,
    })
}

impl OrderService<'_> {
    pub(crate) fn cancellation_preview(
        &self,
        order_id: i64,
        item_id: Option<i64>,
    ) -> Result<CancellationPreview, OrderError> {
        let mut connection = self.database.open()?;
        let transaction = connection.transaction()?;
        let result = preview(&transaction, order_id, item_id)?;
        transaction.commit()?;
        Ok(result)
    }

    pub(crate) fn cancel(
        &self,
        order_id: i64,
        input: CancelOrderInput,
        actor_user_id: i64,
    ) -> Result<OrderDetail, OrderError> {
        let reason = input.reason.trim();
        if reason.is_empty() || reason.chars().count() > 500 || !input.acknowledged {
            return Err(validation(
                "กรุณาระบุเหตุผลไม่เกิน 500 ตัวอักษร และยืนยันการตรวจสอบเอกสาร/stock",
            ));
        }
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = preview(&transaction, order_id, input.item_id)?;
        if current.revision != input.revision {
            return Err(validation("ข้อมูลเปลี่ยนหลังเปิดหน้าตรวจสอบ กรุณาโหลดใหม่ก่อนยกเลิก"));
        }
        let unique = input
            .tasks
            .iter()
            .map(|t| t.task_id)
            .collect::<std::collections::HashSet<_>>();
        if unique.len() != input.tasks.len()
            || input.tasks.len() != current.tasks.len()
            || current.tasks.iter().any(|t| !unique.contains(&t.id))
        {
            return Err(validation("กรุณาระบุว่าเตรียมยาจริงแล้วหรือไม่ให้ครบทุกงานเตรียม"));
        }
        for task in &current.tasks {
            if task.state == "pending"
                && input
                    .tasks
                    .iter()
                    .any(|d| d.task_id == task.id && d.actually_prepared)
            {
                return Err(validation(
                    "งานที่ยัง pending ต้องบันทึกการเตรียมให้ครบก่อน จึงจะระบุว่าเตรียมจริงแล้วได้",
                ));
            }
        }
        transaction.execute("INSERT INTO order_cancellations(order_id,item_id,reason,actor_user_id) VALUES(?1,?2,?3,?4)",params![order_id,input.item_id,reason,actor_user_id])?;
        let event_id = transaction.last_insert_rowid();
        for decision in &input.tasks {
            transaction.execute("UPDATE preparation_tasks SET cancellation_id=?1,cancelled_after_preparation=?2 WHERE id=?3 AND cancellation_id IS NULL",params![event_id,decision.actually_prepared,decision.task_id])?;
        }
        transaction.execute("UPDATE order_items SET cancellation_id=?1 WHERE order_id=?2 AND (?3 IS NULL OR id=?3) AND cancellation_id IS NULL",params![event_id,order_id,input.item_id])?;
        if input.item_id.is_none() {
            transaction.execute(
                "UPDATE orders SET cancellation_id=?1 WHERE id=?2",
                params![event_id, order_id],
            )?;
        }
        audit::append_event(
            &transaction,
            Some(actor_user_id),
            "order_cancelled",
            "order",
            order_id,
            &json!({"cancellation_id":event_id,"item_id":input.item_id,"task_count":input.tasks.len(),"physically_prepared_count":input.tasks.iter().filter(|d|d.actually_prepared).count()}),
        )?;
        let result =
            repository::get_order(&transaction, order_id)?.ok_or(OrderError::OrderNotFound)?;
        transaction.commit()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::{AuthService, AuthSession, BootstrapUserInput},
        db::Database,
        preparation::{PreparationQueueRequest, PreparationService},
        report::{
            PreparationCountReportRequest, ReportInterval, ReportService, WardDeliveryReportRequest,
        },
    };

    struct Fixture {
        _dir: tempfile::TempDir,
        db: Database,
        session: AuthSession,
        actor: i64,
    }
    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let db = Database::initialize(dir.path().join("oncoflow.db")).unwrap();
            let session = AuthSession::default();
            let actor = AuthService::new(&db, &session)
                .bootstrap(BootstrapUserInput {
                    username: "cancel.test".into(),
                    display_name: "Synthetic pharmacist".into(),
                    password: "Synthetic-password-42!".into(),
                })
                .unwrap()
                .current_user
                .unwrap()
                .id;
            db.open().unwrap().execute_batch(&format!(
                "INSERT INTO patients(id,legacy_hn,first_name) VALUES(1,'SYN-CANCEL','Synthetic');
                 INSERT INTO drugs(id,legacy_dcode,drug_name,marker) VALUES(1,'SYN-D','Synthetic drug',1);
                 INSERT INTO orders(id,legacy_orderid,patient_id,oncoflow_created,order_time) VALUES(1,'SYN-O',1,1,'2026-09-29T09:00');
                 INSERT INTO order_items(id,order_id,drug_id,start_date,stop_date,dose) VALUES(1,1,1,'2026-09-29','2026-09-30',10),(2,1,1,'2026-09-29','2026-09-30',20);
                 INSERT INTO preparation_tasks(id,source_order_id,source_order_item_id,drug_id,preparation_date,state,prepared_at,verified_at,prepared_by_user_id,verified_by_user_id,final_container_count)
                 VALUES(1,1,1,1,'2026-09-29','verified',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,{actor},{actor},2),
                       (2,1,1,1,'2026-09-30','prepared',CURRENT_TIMESTAMP,NULL,{actor},NULL,1),
                       (3,1,2,1,'2026-09-29','verified',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,{actor},{actor},1);"
            )).unwrap();
            Self {
                _dir: dir,
                db,
                session,
                actor,
            }
        }
        fn service(&self) -> OrderService<'_> {
            OrderService::new(&self.db)
        }
        fn input(&self, item_id: Option<i64>) -> CancelOrderInput {
            let p = self.service().cancellation_preview(1, item_id).unwrap();
            CancelOrderInput {
                item_id,
                revision: p.revision,
                reason: "คีย์เกิน — synthetic".into(),
                acknowledged: true,
                tasks: p
                    .tasks
                    .iter()
                    .map(|t| CancellationDecision {
                        task_id: t.id,
                        actually_prepared: t.id == 1,
                    })
                    .collect(),
            }
        }
        fn counts(&self) -> crate::report::PreparationCountReport {
            ReportService::new(&self.db, &self.session)
                .preparation_counts(PreparationCountReportRequest {
                    interval: ReportInterval::Daily,
                    date_from: "2026-09-29".into(),
                    date_to: "2026-09-30".into(),
                })
                .unwrap()
        }
    }

    #[test]
    fn cancellation_preserves_physical_work_excludes_unprepared_and_leaves_other_line_usable() {
        let f = Fixture::new();
        let result = f.service().cancel(1, f.input(Some(1)), f.actor).unwrap();
        assert!(result.editable);
        assert!(result.items.iter().find(|i| i.id == 1).unwrap().cancelled);
        assert!(!result.items.iter().find(|i| i.id == 2).unwrap().cancelled);
        assert_eq!(result.cancellations.len(), 1);
        assert_eq!(result.cancellations[0].tasks.len(), 2);
        let counts = f.counts();
        assert_eq!(counts.total_prescriptions, 2);
        assert_eq!(counts.total_bottles, 3);
        assert_eq!(counts.rows.len(), 2); // same drug/day/preparer, separate cancelled row
        assert_eq!(
            counts
                .rows
                .iter()
                .find(|r| r.cancelled)
                .unwrap()
                .bottle_count,
            2
        );
        let delivery = ReportService::new(&f.db, &f.session)
            .ward_delivery(WardDeliveryReportRequest {
                preparation_date: "2026-09-29".into(),
            })
            .unwrap();
        assert_eq!(delivery.rows.len(), 1);
        assert_eq!(delivery.rows[0].task_id, 3);
        let prep = PreparationService::new(&f.db, &f.session);
        assert!(prep.check(1).is_err());
        assert!(prep.verify(1).is_err());
        let queue = prep
            .list_queue(PreparationQueueRequest {
                preparation_date: Some("2026-09-29".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(queue.items[0].eligible_item_count, 1);
        let workspace = prep.initialize_for_date(1, "2026-09-30".into()).unwrap();
        assert!(!workspace
            .items
            .iter()
            .any(|i| i.task.as_ref().is_some_and(|t| t.source_order_item_id == 1)));
    }

    #[test]
    fn whole_order_after_partial_cancellation_retains_both_events_and_blocks_future_work() {
        let f = Fixture::new();
        f.service().cancel(1, f.input(Some(1)), f.actor).unwrap();
        let result = f.service().cancel(1, f.input(None), f.actor).unwrap();
        assert!(!result.editable);
        assert_eq!(
            result.workflow_status,
            super::super::OrderWorkflowStatus::Cancelled
        );
        assert_eq!(result.cancellations.len(), 2);
        assert!(result.items.iter().all(|i| i.cancelled));
        assert!(f.service().cancellation_preview(1, None).is_err());
        assert!(PreparationService::new(&f.db, &f.session)
            .initialize_for_date(1, "2026-09-30".into())
            .is_err());
        assert_eq!(f.counts().total_prescriptions, 1);
    }

    #[test]
    fn stale_task_changes_incomplete_decisions_and_audit_failure_are_atomic() {
        let f = Fixture::new();
        let stale = f.input(None);
        f.db.open()
            .unwrap()
            .execute(
                "UPDATE preparation_tasks SET final_container_count=3 WHERE id=1",
                [],
            )
            .unwrap();
        assert!(f.service().cancel(1, stale, f.actor).is_err());
        let mut incomplete = f.input(None);
        incomplete.tasks.pop();
        assert!(f.service().cancel(1, incomplete, f.actor).is_err());
        let mut duplicate = f.input(None);
        duplicate.tasks[1].task_id = duplicate.tasks[0].task_id;
        assert!(f.service().cancel(1, duplicate, f.actor).is_err());
        let mut no_reason = f.input(None);
        no_reason.reason = " ".into();
        assert!(f.service().cancel(1, no_reason, f.actor).is_err());
        let mut unacknowledged = f.input(None);
        unacknowledged.acknowledged = false;
        assert!(f.service().cancel(1, unacknowledged, f.actor).is_err());
        f.db.open().unwrap().execute_batch("CREATE TRIGGER reject_cancel_audit BEFORE INSERT ON audit_events WHEN NEW.event_type='order_cancelled' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
        assert!(f.service().cancel(1, f.input(None), f.actor).is_err());
        let current = f.service().get(1).unwrap();
        assert!(current.editable);
        assert!(current.cancellations.is_empty());
        assert!(current.items.iter().all(|i| !i.cancelled));
        assert_eq!(
            f.db.open()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM preparation_tasks WHERE cancellation_id IS NOT NULL",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn cancelled_records_reject_edits_and_audit_does_not_copy_reason() {
        let f = Fixture::new();
        f.service().cancel(1, f.input(Some(1)), f.actor).unwrap();
        let c = f.db.open().unwrap();
        for sql in [
            "UPDATE order_items SET dose=999 WHERE id=1",
            "DELETE FROM order_items WHERE id=1",
            "UPDATE preparation_tasks SET final_container_count=9 WHERE id=1",
            "DELETE FROM preparation_tasks WHERE id=1",
            "UPDATE order_cancellations SET reason='changed'",
            "DELETE FROM order_cancellations",
        ] {
            assert!(c.execute(sql, []).is_err(), "{sql}");
        }
        assert!(f.service().remove_item(1, 1).is_err());
        assert_eq!(c.query_row("SELECT COUNT(*) FROM audit_events WHERE event_type='order_cancelled' AND metadata_json LIKE '%synthetic%'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
}
