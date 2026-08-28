use rusqlite::TransactionBehavior;
use serde_json::json;
use thiserror::Error;

use crate::{
    auth::{audit, AuthError, AuthSession},
    db::{Database, DatabaseError},
};

use super::{
    repository, AdjustmentDirection, InventoryAdjustmentInput, InventoryDetail,
    InventoryListRequest, InventoryListResponse, InventoryManualIssueInput,
    InventoryMovementListRequest, InventoryMovementListResponse, InventoryMovementResult,
    InventoryMovementType, InventoryReceiptInput,
};

const NOTE_MAX_CHARS: usize = 1_000;
const REFERENCE_MAX_CHARS: usize = 120;

#[derive(Debug, Error)]
pub(crate) enum InventoryError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error("inventory drug was not found")]
    NotFound,
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct InventoryService<'a> {
    database: &'a Database,
    session: &'a AuthSession,
}

impl<'a> InventoryService<'a> {
    pub(crate) fn new(database: &'a Database, session: &'a AuthSession) -> Self {
        Self { database, session }
    }

    pub(crate) fn list(
        &self,
        mut request: InventoryListRequest,
    ) -> Result<InventoryListResponse, InventoryError> {
        self.session.require_user()?;
        request.search = clean_optional(request.search, 200, "search")?;
        let connection = self.database.open()?;
        Ok(repository::list_inventory(&connection, &request)?)
    }

    pub(crate) fn low_stock(
        &self,
        mut request: InventoryListRequest,
    ) -> Result<InventoryListResponse, InventoryError> {
        request.tracked_only = true;
        request.low_stock_only = true;
        self.list(request)
    }

    pub(crate) fn get(&self, drug_id: i64) -> Result<InventoryDetail, InventoryError> {
        self.session.require_user()?;
        validate_drug_id(drug_id)?;
        let connection = self.database.open()?;
        repository::get_inventory_item(&connection, drug_id)?.ok_or(InventoryError::NotFound)
    }

    pub(crate) fn movements(
        &self,
        request: InventoryMovementListRequest,
    ) -> Result<InventoryMovementListResponse, InventoryError> {
        self.session.require_user()?;
        validate_drug_id(request.drug_id)?;
        let connection = self.database.open()?;
        if repository::get_inventory_item(&connection, request.drug_id)?.is_none() {
            return Err(InventoryError::NotFound);
        }
        Ok(repository::list_movements(&connection, &request)?)
    }

    pub(crate) fn record_receipt(
        &self,
        input: InventoryReceiptInput,
    ) -> Result<InventoryMovementResult, InventoryError> {
        let reference = clean_optional(input.reference, REFERENCE_MAX_CHARS, "reference")?;
        let note = clean_optional(input.note, NOTE_MAX_CHARS, "note")?;
        self.record_movement(MovementRequest {
            drug_id: input.drug_id,
            movement_type: InventoryMovementType::Receipt,
            quantity_delta: validate_positive_quantity(input.quantity)?,
            occurred_at: clean_timestamp(input.occurred_at)?,
            reference,
            note,
            note_required: false,
            event_type: "inventory_receipt",
        })
    }

    pub(crate) fn record_adjustment(
        &self,
        input: InventoryAdjustmentInput,
    ) -> Result<InventoryMovementResult, InventoryError> {
        let quantity = validate_positive_quantity(input.quantity)?;
        let (movement_type, quantity_delta) = match input.direction {
            AdjustmentDirection::Increase => (InventoryMovementType::AdjustmentIncrease, quantity),
            AdjustmentDirection::Decrease => (InventoryMovementType::AdjustmentDecrease, -quantity),
        };
        self.record_movement(MovementRequest {
            drug_id: input.drug_id,
            movement_type,
            quantity_delta,
            occurred_at: clean_timestamp(input.occurred_at)?,
            reference: clean_optional(input.reference, REFERENCE_MAX_CHARS, "reference")?,
            note: clean_optional(Some(input.note), NOTE_MAX_CHARS, "note")?,
            note_required: true,
            event_type: "inventory_adjustment",
        })
    }

    pub(crate) fn record_manual_issue(
        &self,
        input: InventoryManualIssueInput,
    ) -> Result<InventoryMovementResult, InventoryError> {
        self.record_movement(MovementRequest {
            drug_id: input.drug_id,
            movement_type: InventoryMovementType::ManualIssue,
            quantity_delta: -validate_positive_integer_quantity(input.quantity)?,
            occurred_at: clean_timestamp(input.occurred_at)?,
            reference: clean_optional(input.reference, REFERENCE_MAX_CHARS, "reference")?,
            note: clean_optional(Some(input.note), NOTE_MAX_CHARS, "note")?,
            note_required: true,
            event_type: "inventory_manual_issue",
        })
    }

    fn record_movement(
        &self,
        request: MovementRequest,
    ) -> Result<InventoryMovementResult, InventoryError> {
        let actor = self.session.require_user()?;
        validate_drug_id(request.drug_id)?;
        if request.note_required && request.note.is_none() {
            return Err(validation("note", "A reason is required"));
        }
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !repository::drug_exists(&transaction, request.drug_id)? {
            return Err(InventoryError::NotFound);
        }
        let occurred_at =
            repository::normalize_timestamp(&transaction, request.occurred_at.as_deref())?
                .ok_or_else(|| validation("occurredAt", "Enter a valid date and time"))?;
        let reference_type = request
            .reference
            .as_ref()
            .map(|_| match request.movement_type {
                InventoryMovementType::Receipt => "receipt_reference",
                InventoryMovementType::ManualIssue => "manual_issue_reference",
                InventoryMovementType::AdjustmentIncrease
                | InventoryMovementType::AdjustmentDecrease => "adjustment_reference",
                InventoryMovementType::OpeningBalance => {
                    unreachable!("runtime opening is unsupported")
                }
                InventoryMovementType::PreparationIssue => {
                    unreachable!("preparation issues are recorded by verification")
                }
            });
        let movement_id = repository::insert_movement(
            &transaction,
            &repository::NewMovement {
                drug_id: request.drug_id,
                movement_type: request.movement_type,
                quantity_delta: request.quantity_delta,
                occurred_at: &occurred_at,
                actor_user_id: actor.id,
                reference_type,
                reference_id: request.reference.as_deref(),
                note: request.note.as_deref(),
                preparation_task_id: None,
            },
        )?;
        let resulting_balance = repository::current_balance(&transaction, request.drug_id)?
            .expect("a newly inserted movement always produces a balance");
        audit::append_event(
            &transaction,
            Some(actor.id),
            request.event_type,
            "inventory_movement",
            movement_id,
            &json!({
                "drug_id": request.drug_id,
                "movement_type": request.movement_type.as_database(),
                "quantity_delta": request.quantity_delta,
                "resulting_balance": resulting_balance,
            }),
        )?;
        transaction.commit()?;

        let inventory = repository::get_inventory_item(&connection, request.drug_id)?
            .ok_or(InventoryError::NotFound)?;
        let movement = repository::get_movement(&connection, request.drug_id, movement_id)?
            .ok_or(InventoryError::NotFound)?;
        Ok(InventoryMovementResult {
            inventory,
            movement,
        })
    }
}

struct MovementRequest {
    drug_id: i64,
    movement_type: InventoryMovementType,
    quantity_delta: f64,
    occurred_at: Option<String>,
    reference: Option<String>,
    note: Option<String>,
    note_required: bool,
    event_type: &'static str,
}

fn validate_drug_id(drug_id: i64) -> Result<(), InventoryError> {
    if drug_id <= 0 {
        return Err(validation("drugId", "Select a valid drug"));
    }
    Ok(())
}

fn validate_positive_quantity(quantity: f64) -> Result<f64, InventoryError> {
    if !quantity.is_finite() || quantity <= 0.0 {
        return Err(validation(
            "quantity",
            "Quantity must be a finite number greater than zero",
        ));
    }
    Ok(quantity)
}

fn validate_positive_integer_quantity(quantity: f64) -> Result<f64, InventoryError> {
    let quantity = validate_positive_quantity(quantity)?;
    if quantity.fract() != 0.0 {
        return Err(validation(
            "quantity",
            "Issue quantity must be a whole number",
        ));
    }
    Ok(quantity)
}

fn clean_timestamp(value: Option<String>) -> Result<Option<String>, InventoryError> {
    let value = value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    });
    if value.as_ref().is_some_and(|value| value.len() > 64) {
        return Err(validation("occurredAt", "Enter a valid date and time"));
    }
    Ok(value)
}

fn clean_optional(
    value: Option<String>,
    max_chars: usize,
    field: &'static str,
) -> Result<Option<String>, InventoryError> {
    let value = value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    });
    if value
        .as_ref()
        .is_some_and(|value| value.chars().count() > max_chars)
    {
        return Err(validation(
            field,
            format!("Value must be {max_chars} characters or fewer"),
        ));
    }
    Ok(value)
}

fn validation(field: &'static str, message: impl Into<String>) -> InventoryError {
    InventoryError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::{AuthService, AuthSession, BootstrapUserInput},
        inventory::{InventorySortField, SortDirection, StockState},
    };

    struct Fixture {
        _directory: tempfile::TempDir,
        database: Database,
        session: AuthSession,
    }

    impl Fixture {
        fn new(authenticated: bool) -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
            let session = AuthSession::default();
            if authenticated {
                AuthService::new(&database, &session)
                    .bootstrap(BootstrapUserInput {
                        username: "inventory.pharmacist".into(),
                        display_name: "เภสัชกรคลังทดสอบ".into(),
                        password: "synthetic inventory password 42!".into(),
                    })
                    .unwrap();
            }
            let connection = database.open().unwrap();
            connection
                .execute(
                    "INSERT INTO units(id,legacy_unitcode,unit_name)
                     VALUES(1,'SYN-U','legacy unit')",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO drugs(
                       id,legacy_dcode,drug_name,unit_id,package,inventory_qty,
                       inventory_min,inventory_max,inventory_enabled,inventory_cut
                     ) VALUES
                       (1,'TH-LOW','ยาคลังทดสอบ',1,'Synthetic pack',5,5,20,1,1),
                       (2,'NEG','Negative synthetic',1,NULL,-2,NULL,10,1,0),
                       (3,'OFF','Untracked synthetic',NULL,NULL,NULL,NULL,NULL,0,NULL),
                       (4,'EMPTY','Unknown tracked',NULL,NULL,NULL,NULL,NULL,1,NULL)",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO inventory_movements(
                       drug_id,movement_type,quantity_delta,reference_type,reference_id,note
                     ) VALUES
                       (1,'opening_balance',5,'legacy_drug_inventory','TH-LOW','Synthetic opening'),
                       (2,'opening_balance',-2,'legacy_drug_inventory','NEG','Synthetic opening')",
                    [],
                )
                .unwrap();
            Self {
                _directory: directory,
                database,
                session,
            }
        }

        fn service(&self) -> InventoryService<'_> {
            InventoryService::new(&self.database, &self.session)
        }
    }

    #[test]
    fn lists_searches_and_classifies_exact_low_shortage_untracked_and_unknown() {
        let fixture = Fixture::new(true);
        let all = fixture
            .service()
            .list(InventoryListRequest {
                sort_by: InventorySortField::State,
                sort_direction: SortDirection::Asc,
                ..InventoryListRequest::default()
            })
            .unwrap();
        assert_eq!(all.total, 4);
        assert_eq!(
            all.items
                .iter()
                .find(|item| item.drug_code == "TH-LOW")
                .unwrap()
                .stock_state,
            StockState::Low
        );
        assert_eq!(
            all.items
                .iter()
                .find(|item| item.drug_code == "NEG")
                .unwrap()
                .stock_state,
            StockState::Shortage
        );
        assert_eq!(
            all.items
                .iter()
                .find(|item| item.drug_code == "OFF")
                .unwrap()
                .stock_state,
            StockState::Untracked
        );
        assert_eq!(
            all.items
                .iter()
                .find(|item| item.drug_code == "EMPTY")
                .unwrap()
                .stock_state,
            StockState::Unknown
        );

        let thai = fixture
            .service()
            .list(InventoryListRequest {
                search: Some("คลัง".into()),
                ..InventoryListRequest::default()
            })
            .unwrap();
        assert_eq!(thai.total, 1);
        assert_eq!(thai.items[0].drug_code, "TH-LOW");
        let low = fixture
            .service()
            .low_stock(InventoryListRequest::default())
            .unwrap();
        assert_eq!(low.total, 2);
        assert!(low.items.iter().any(|item| item.drug_code == "NEG"));
    }

    #[test]
    fn records_receipt_adjustments_and_manual_issue_with_actor_and_negative_balance() {
        let fixture = Fixture::new(true);
        let receipt = fixture
            .service()
            .record_receipt(InventoryReceiptInput {
                drug_id: 1,
                quantity: 3.5,
                reference: Some(" SYN-REF ".into()),
                note: Some(" Synthetic receipt ".into()),
                ..InventoryReceiptInput::default()
            })
            .unwrap();
        assert_eq!(receipt.inventory.summary.current_stock, Some(8.5));
        assert_eq!(
            receipt.movement.actor_display_name.as_deref(),
            Some("เภสัชกรคลังทดสอบ")
        );
        fixture
            .service()
            .record_adjustment(InventoryAdjustmentInput {
                drug_id: 1,
                direction: AdjustmentDirection::Increase,
                quantity: 1.5,
                note: "Synthetic count correction".into(),
                ..InventoryAdjustmentInput::default()
            })
            .unwrap();
        fixture
            .service()
            .record_adjustment(InventoryAdjustmentInput {
                drug_id: 1,
                direction: AdjustmentDirection::Decrease,
                quantity: 2.0,
                note: "Synthetic count correction".into(),
                ..InventoryAdjustmentInput::default()
            })
            .unwrap();
        let issue = fixture
            .service()
            .record_manual_issue(InventoryManualIssueInput {
                drug_id: 1,
                quantity: 11.0,
                note: "Synthetic manual issue".into(),
                ..InventoryManualIssueInput::default()
            })
            .unwrap();
        assert_eq!(issue.inventory.summary.current_stock, Some(-3.0));
        assert_eq!(issue.inventory.summary.stock_state, StockState::Shortage);
        assert_eq!(issue.movement.quantity_delta, -11.0);

        let history = fixture
            .service()
            .movements(InventoryMovementListRequest {
                drug_id: 1,
                ..InventoryMovementListRequest::default()
            })
            .unwrap();
        assert_eq!(history.total, 5);
        assert_eq!(history.items[0].resulting_balance, -3.0);
        let event_types = fixture
            .database
            .open()
            .unwrap()
            .prepare(
                "SELECT event_type FROM audit_events
                 WHERE event_type LIKE 'inventory_%' ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            event_types,
            vec![
                "inventory_receipt",
                "inventory_adjustment",
                "inventory_adjustment",
                "inventory_manual_issue"
            ]
        );
    }

    #[test]
    fn rejects_zero_non_finite_missing_reason_invalid_date_and_unknown_drug() {
        let fixture = Fixture::new(true);
        for quantity in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                fixture.service().record_receipt(InventoryReceiptInput {
                    drug_id: 1,
                    quantity,
                    ..InventoryReceiptInput::default()
                }),
                Err(InventoryError::Validation {
                    field: "quantity",
                    ..
                })
            ));
        }
        assert!(matches!(
            fixture
                .service()
                .record_manual_issue(InventoryManualIssueInput {
                    drug_id: 1,
                    quantity: 1.5,
                    note: "Synthetic fractional issue".into(),
                    ..InventoryManualIssueInput::default()
                }),
            Err(InventoryError::Validation {
                field: "quantity",
                ..
            })
        ));
        assert!(matches!(
            fixture
                .service()
                .record_adjustment(InventoryAdjustmentInput {
                    drug_id: 1,
                    quantity: 1.0,
                    note: "  ".into(),
                    ..InventoryAdjustmentInput::default()
                }),
            Err(InventoryError::Validation { field: "note", .. })
        ));
        assert!(matches!(
            fixture.service().record_receipt(InventoryReceiptInput {
                drug_id: 1,
                quantity: 1.0,
                occurred_at: Some("not-a-date".into()),
                ..InventoryReceiptInput::default()
            }),
            Err(InventoryError::Validation {
                field: "occurredAt",
                ..
            })
        ));
        assert!(matches!(
            fixture.service().record_receipt(InventoryReceiptInput {
                drug_id: 99_999,
                quantity: 1.0,
                ..InventoryReceiptInput::default()
            }),
            Err(InventoryError::NotFound)
        ));
    }

    #[test]
    fn mutation_requires_authenticated_rust_session() {
        let fixture = Fixture::new(false);
        let error = fixture
            .service()
            .record_receipt(InventoryReceiptInput {
                drug_id: 1,
                quantity: 1.0,
                ..InventoryReceiptInput::default()
            })
            .unwrap_err();
        assert!(matches!(
            error,
            InventoryError::Auth(AuthError::AuthenticationRequired)
        ));
        let movements: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM inventory_movements WHERE drug_id=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(movements, 1);
    }

    #[test]
    fn movement_and_audit_roll_back_together_without_changing_legacy_snapshot() {
        let fixture = Fixture::new(true);
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_inventory_audit_failure
                 BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='inventory_receipt'
                 BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().record_receipt(InventoryReceiptInput {
                drug_id: 1,
                quantity: 9.0,
                ..InventoryReceiptInput::default()
            }),
            Err(InventoryError::Sqlite(_))
        ));
        let connection = fixture.database.open().unwrap();
        let values: (i64, f64, f64) = connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM inventory_movements WHERE drug_id=1),
                   (SELECT SUM(quantity_delta) FROM inventory_movements WHERE drug_id=1),
                   (SELECT inventory_qty FROM drugs WHERE id=1)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(values, (1, 5.0, 5.0));
    }

    #[test]
    fn movement_history_is_append_only_and_legacy_inventory_events_are_not_double_counted() {
        let fixture = Fixture::new(true);
        let connection = fixture.database.open().unwrap();
        connection
            .execute(
                "INSERT INTO inventory_events(
                   legacy_incode,drug_id,quantity,inventory_ok,send_order
                 ) VALUES('SYN-EVENT',1,4,1,1)",
                [],
            )
            .unwrap();
        assert_eq!(
            fixture.service().get(1).unwrap().summary.current_stock,
            Some(5.0)
        );
        assert_eq!(
            fixture
                .service()
                .get(1)
                .unwrap()
                .legacy_inventory_event_count,
            1
        );
        assert!(connection
            .execute(
                "UPDATE inventory_movements SET quantity_delta=99 WHERE drug_id=1",
                [],
            )
            .is_err());
        assert!(connection
            .execute("DELETE FROM inventory_movements WHERE drug_id=1", [])
            .is_err());
        assert_eq!(
            fixture.service().get(1).unwrap().summary.current_stock,
            Some(5.0)
        );
    }
}
