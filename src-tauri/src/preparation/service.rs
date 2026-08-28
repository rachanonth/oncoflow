use std::collections::HashMap;

use rusqlite::{Connection, TransactionBehavior};
use serde_json::json;
use thiserror::Error;

use crate::{
    auth::{audit, AuthError, AuthSession},
    clinical::decimal::{DecimalParse, LegacyDecimal},
    db::{Database, DatabaseError},
    inventory::{self, InventoryMovementType, NewMovement, StockState},
    preparation_calc::{calculate_preparation, PreparationCalculationInput},
    safety::{self, SafetyError},
};

use super::{
    evaluate_eligibility, reference_quantity, repository, EligibilityStatus,
    PreparationInventoryPostingStatus, PreparationIssueStockState, PreparationQueueRequest,
    PreparationQueueResponse, PreparationState, PreparationTask, PreparationTaskInput,
    PreparationWorkspace, PreparationWorkspaceItem, PREPARATION_ELIGIBILITY_RULE,
};

const PREPARATION_INVENTORY_WORKFLOW_RULE: &str = "oncoflow-preparation-inventory-v1";
const MAX_EXACT_CONTAINER_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Debug, Error)]
pub(crate) enum PreparationError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error("order record was not found")]
    OrderNotFound,
    #[error("preparation task was not found")]
    TaskNotFound,
    #[error("historical migrated orders cannot create preparation tasks")]
    HistoricalReadOnly,
    #[error("the order is currently on hold")]
    OrderOnHold,
    #[error("the order is not available for preparation on this date")]
    DateUnavailable,
    #[error("verified preparation tasks are immutable")]
    VerifiedReadOnly,
    #[error("the preparation must be marked prepared before verification")]
    NotPrepared,
    #[error("the source order item changed after preparation was initialized")]
    StaleSource,
    #[error("{count} current safety finding(s) require explicit acknowledgement")]
    SafetyReviewRequired { count: usize },
    #[error("the current safety finding was not found")]
    FindingNotFound,
    #[error("the safety finding does not require acknowledgement")]
    FindingNotAcknowledgable,
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Safety(#[from] SafetyError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct PreparationService<'a> {
    database: &'a Database,
    session: &'a AuthSession,
}

impl<'a> PreparationService<'a> {
    pub(crate) fn new(database: &'a Database, session: &'a AuthSession) -> Self {
        Self { database, session }
    }

    pub(crate) fn list_queue(
        &self,
        mut request: PreparationQueueRequest,
    ) -> Result<PreparationQueueResponse, PreparationError> {
        self.session.require_user()?;
        request.search = clean_optional(request.search);
        request.date_from = clean_optional(request.date_from);
        request.date_to = clean_optional(request.date_to);
        request.preparation_date = clean_optional(request.preparation_date);
        validate_date_range(request.date_from.as_deref(), request.date_to.as_deref())?;
        if let Some(value) = request.preparation_date.as_deref() {
            validate_preparation_date(value)?;
        }
        let connection = self.database.open()?;
        Ok(repository::list_queue(&connection, &request)?)
    }

    #[cfg(test)]
    pub(crate) fn get_workspace(
        &self,
        order_id: i64,
    ) -> Result<PreparationWorkspace, PreparationError> {
        let connection = self.database.open()?;
        let preparation_date = default_preparation_date(&connection, order_id)?;
        drop(connection);
        self.get_workspace_for_date(order_id, preparation_date)
    }

    pub(crate) fn get_workspace_for_date(
        &self,
        order_id: i64,
        preparation_date: String,
    ) -> Result<PreparationWorkspace, PreparationError> {
        self.session.require_user()?;
        let preparation_date = preparation_date.trim();
        validate_preparation_date(preparation_date)?;
        let connection = self.database.open()?;
        build_workspace(&connection, order_id, preparation_date)
    }

    #[cfg(test)]
    pub(crate) fn initialize(
        &self,
        order_id: i64,
    ) -> Result<PreparationWorkspace, PreparationError> {
        let connection = self.database.open()?;
        let preparation_date = default_preparation_date(&connection, order_id)?;
        drop(connection);
        self.initialize_for_date(order_id, preparation_date)
    }

    pub(crate) fn initialize_for_date(
        &self,
        order_id: i64,
        preparation_date: String,
    ) -> Result<PreparationWorkspace, PreparationError> {
        let actor = self.session.require_user()?;
        let preparation_date = preparation_date.trim();
        validate_preparation_date(preparation_date)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let header = repository::load_header(&transaction, order_id)?
            .ok_or(PreparationError::OrderNotFound)?;
        if !header.editable {
            return Err(PreparationError::HistoricalReadOnly);
        }
        let source_date = source_date_for_preparation(&transaction, &header, preparation_date)?;
        let sources = repository::load_source_items(&transaction, order_id)?;
        let eligible = sources
            .iter()
            .filter(|source| {
                source_due_on_date(source, header.treatment_time.as_deref(), &source_date)
                    && evaluate_eligibility(true, true, source.legacy_marker).status
                        == EligibilityStatus::Eligible
            })
            .collect::<Vec<_>>();
        for source in eligible {
            let inserted_task_id =
                repository::insert_task(&transaction, &source.snapshot, preparation_date)?;
            if let Some(task_id) = inserted_task_id {
                audit::append_event(
                    &transaction,
                    Some(actor.id),
                    "preparation_created",
                    "preparation_task",
                    task_id,
                    &json!({
                        "source_order_id": order_id,
                        "source_order_item_id": source.snapshot.source_order_item_id,
                        "preparation_date": preparation_date,
                        "eligibility_rule": PREPARATION_ELIGIBILITY_RULE,
                        "source_stale": false
                    }),
                )?;
            } else if let Some(task) = repository::load_task_for_item_on_date(
                &transaction,
                source.snapshot.source_order_item_id,
                preparation_date,
            )? {
                if task.state == PreparationState::Pending
                    && task.snapshot() != source.snapshot
                    && repository::refresh_pending_task_snapshot(
                        &transaction,
                        task.id,
                        &source.snapshot,
                    )? == 1
                {
                    audit::append_event(
                        &transaction,
                        Some(actor.id),
                        "preparation_source_refreshed",
                        "preparation_task",
                        task.id,
                        &json!({
                            "source_order_id": order_id,
                            "source_order_item_id": source.snapshot.source_order_item_id,
                            "preparation_date": preparation_date,
                            "state": "pending",
                            "preparation_details_cleared": true
                        }),
                    )?;
                }
            }
        }
        transaction.commit()?;
        build_workspace(&connection, order_id, preparation_date)
    }

    pub(crate) fn update_task(
        &self,
        task_id: i64,
        mut input: PreparationTaskInput,
    ) -> Result<PreparationTask, PreparationError> {
        self.session.require_user()?;
        if input
            .preparation_volume_ml
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(validation(
                "preparationVolumeMl",
                "Preparation volume must be zero or greater",
            ));
        }
        input.preparation_notes = clean_optional(input.preparation_notes);
        if input
            .preparation_notes
            .as_ref()
            .is_some_and(|value| value.chars().count() > 4000)
        {
            return Err(validation(
                "preparationNotes",
                "Preparation notes are limited to 4,000 characters",
            ));
        }
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let task = load_mutable_task(&transaction, task_id)?;
        ensure_current_source(&transaction, &task)?;
        let container_count = input
            .final_container_count
            .unwrap_or(task.final_container_count);
        validate_final_container_count(container_count)?;
        if repository::update_task(
            &transaction,
            task_id,
            input.preparation_volume_ml,
            input.preparation_notes.as_deref(),
            None,
            container_count,
        )? == 0
        {
            return Err(PreparationError::TaskNotFound);
        }
        transaction.commit()?;
        repository::load_task(&connection, task_id)?.ok_or(PreparationError::TaskNotFound)
    }

    #[cfg(test)]
    pub(crate) fn mark_prepared(&self, task_id: i64) -> Result<PreparationTask, PreparationError> {
        let actor = self.session.require_user()?;
        self.mark_prepared_for(task_id, actor.id)
    }

    pub(crate) fn mark_prepared_for(
        &self,
        task_id: i64,
        prepared_by_user_id: i64,
    ) -> Result<PreparationTask, PreparationError> {
        let actor = self.session.require_user()?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prepared_by =
            repository::load_preparation_pharmacist(&transaction, prepared_by_user_id)?
                .ok_or_else(|| {
                    validation(
                        "preparedByUserId",
                        "Select an active pharmacist as the preparation pharmacist",
                    )
                })?;
        let task = load_mutable_task(&transaction, task_id)?;
        ensure_current_source(&transaction, &task)?;
        if repository::mark_prepared(&transaction, task_id, prepared_by.id)? == 0 {
            return Err(PreparationError::NotPrepared);
        }
        audit::append_event(
            &transaction,
            Some(actor.id),
            "preparation_marked_prepared",
            "preparation_task",
            task_id,
            &json!({"state":"prepared","prepared_by_user_id":prepared_by.id,"source_stale":false}),
        )?;
        transaction.commit()?;
        repository::load_task(&connection, task_id)?.ok_or(PreparationError::TaskNotFound)
    }

    pub(crate) fn verify(&self, task_id: i64) -> Result<PreparationTask, PreparationError> {
        let actor = self.session.require_user()?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let task =
            repository::load_task(&transaction, task_id)?.ok_or(PreparationError::TaskNotFound)?;
        if task.state == PreparationState::Verified {
            return Ok(task);
        }
        if task.state != PreparationState::Prepared {
            return Err(PreparationError::NotPrepared);
        }
        ensure_editable_order(&transaction, task.source_order_id)?;
        ensure_current_source(&transaction, &task)?;

        let safety = safety::evaluate_order(&transaction, task.source_order_id)?;
        let acknowledgements =
            repository::load_acknowledgements(&transaction, task.source_order_id)?;
        let missing = safety
            .findings
            .iter()
            .filter(|finding| {
                finding.acknowledgement_required
                    && finding
                        .order_item_id
                        .is_none_or(|item_id| item_id == task.source_order_item_id)
                    && !acknowledgements.iter().any(|acknowledgement| {
                        acknowledgement.finding_id == finding.id
                            && acknowledgement.finding_fingerprint == finding.fingerprint
                            && if finding.order_item_id.is_some() {
                                acknowledgement.preparation_task_id == Some(task.id)
                            } else {
                                acknowledgement.preparation_task_id.is_none()
                            }
                    })
            })
            .count();
        if missing > 0 {
            return Err(PreparationError::SafetyReviewRequired { count: missing });
        }
        let source = repository::load_source_items(&transaction, task.source_order_id)?
            .into_iter()
            .find(|source| source.snapshot.source_order_item_id == task.source_order_item_id)
            .ok_or(PreparationError::StaleSource)?;
        let calculation = calculate_source(&source);
        validate_final_container_count(task.final_container_count)?;
        repository::update_task(
            &transaction,
            task.id,
            task.preparation_volume_ml,
            task.preparation_notes.as_deref(),
            calculation.withdrawal_volume_ml.as_deref(),
            task.final_container_count,
        )?;
        if repository::mark_verified(&transaction, task_id, actor.id)? == 0 {
            return Err(PreparationError::NotPrepared);
        }
        let posting =
            post_inventory_decision(&transaction, &task, &source, &calculation, actor.id)?;
        if let Some(movement_id) = posting.inventory_movement_id {
            audit::append_event(
                &transaction,
                Some(actor.id),
                "preparation_inventory_issued",
                "inventory_movement",
                movement_id,
                &json!({
                    "preparation_task_id": task.id,
                    "inventory_movement_id": movement_id,
                    "drug_id": task.drug_id,
                    "quantity_delta": posting.quantity_delta,
                    "containers_required": posting.containers_required,
                    "balance_before": posting.balance_before,
                    "balance_after": posting.balance_after,
                    "calculation_rule_id": calculation.rule_id,
                    "calculation_ruleset_version": calculation.ruleset_version,
                    "workflow_rule_id": PREPARATION_INVENTORY_WORKFLOW_RULE
                }),
            )?;
        } else if posting.status == PreparationInventoryPostingStatus::ManualReconciliationRequired
        {
            audit::append_event(
                &transaction,
                Some(actor.id),
                "preparation_inventory_reconciliation_required",
                "preparation_inventory_posting",
                posting.posting_id,
                &json!({
                    "preparation_task_id": task.id,
                    "drug_id": task.drug_id,
                    "calculation_status": calculation.status.as_database(),
                    "calculation_rule_id": calculation.rule_id,
                    "calculation_ruleset_version": calculation.ruleset_version,
                    "workflow_rule_id": PREPARATION_INVENTORY_WORKFLOW_RULE,
                    "reason_code": posting.reason_code
                }),
            )?;
        }
        audit::append_event(
            &transaction,
            Some(actor.id),
            "preparation_verified",
            "preparation_task",
            task_id,
            &json!({
                "state":"verified",
                "source_stale":false,
                "inventory_posting_id":posting.posting_id,
                "inventory_posting_status":posting.status.as_database(),
                "workflow_rule_id":PREPARATION_INVENTORY_WORKFLOW_RULE
            }),
        )?;
        transaction.commit()?;
        repository::load_task(&connection, task_id)?.ok_or(PreparationError::TaskNotFound)
    }

    pub(crate) fn check(&self, task_id: i64) -> Result<PreparationTask, PreparationError> {
        let actor = self.session.require_user()?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_task_in_transaction(&transaction, task_id, actor.id)?;
        transaction.commit()?;
        repository::load_task(&connection, task_id)?.ok_or(PreparationError::TaskNotFound)
    }

    pub(crate) fn check_batch(
        &self,
        mut task_ids: Vec<i64>,
    ) -> Result<Vec<PreparationTask>, PreparationError> {
        let actor = self.session.require_user()?;
        task_ids.sort_unstable();
        task_ids.dedup();
        if task_ids.is_empty() {
            return Err(validation(
                "taskIds",
                "Select at least one preparation task to check",
            ));
        }
        if task_ids.len() > 2_000 {
            return Err(validation(
                "taskIds",
                "A maximum of 2,000 preparation tasks can be checked at once",
            ));
        }

        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        for task_id in &task_ids {
            check_task_in_transaction(&transaction, *task_id, actor.id)?;
        }
        transaction.commit()?;

        task_ids
            .into_iter()
            .map(|task_id| {
                repository::load_task(&connection, task_id)?.ok_or(PreparationError::TaskNotFound)
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn acknowledge_safety_finding(
        &self,
        order_id: i64,
        finding_id: String,
    ) -> Result<PreparationWorkspace, PreparationError> {
        let connection = self.database.open()?;
        let preparation_date = default_preparation_date(&connection, order_id)?;
        drop(connection);
        self.acknowledge_safety_finding_for_date(order_id, preparation_date, finding_id)
    }

    pub(crate) fn acknowledge_safety_finding_for_date(
        &self,
        order_id: i64,
        preparation_date: String,
        finding_id: String,
    ) -> Result<PreparationWorkspace, PreparationError> {
        let actor = self.session.require_user()?;
        let preparation_date = preparation_date.trim();
        validate_preparation_date(preparation_date)?;
        let finding_id = finding_id.trim();
        if finding_id.is_empty() {
            return Err(PreparationError::FindingNotFound);
        }
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_editable_order(&transaction, order_id)?;
        let evaluation = safety::evaluate_order(&transaction, order_id)?;
        let finding = evaluation
            .findings
            .iter()
            .find(|finding| finding.id == finding_id)
            .ok_or(PreparationError::FindingNotFound)?;
        if !finding.acknowledgement_required {
            return Err(PreparationError::FindingNotAcknowledgable);
        }
        let task = if let Some(order_item_id) = finding.order_item_id {
            let task = repository::load_task_for_item_on_date(
                &transaction,
                order_item_id,
                preparation_date,
            )?
            .ok_or(PreparationError::TaskNotFound)?;
            if task.source_order_id != order_id {
                return Err(PreparationError::TaskNotFound);
            }
            ensure_current_source(&transaction, &task)?;
            Some(task)
        } else {
            let tasks =
                repository::load_tasks_for_order_on_date(&transaction, order_id, preparation_date)?;
            if tasks.is_empty() {
                return Err(PreparationError::TaskNotFound);
            }
            for task in &tasks {
                ensure_current_source(&transaction, task)?;
            }
            None
        };
        if let Some(acknowledgement_id) = repository::insert_acknowledgement(
            &transaction,
            order_id,
            task.as_ref(),
            finding,
            actor.id,
        )? {
            audit::append_event(
                &transaction,
                Some(actor.id),
                "safety_finding_acknowledged",
                "safety_acknowledgement",
                acknowledgement_id,
                &json!({
                    "order_id": order_id,
                    "preparation_task_id": task.as_ref().map(|value| value.id),
                    "finding_id": finding.id,
                    "finding_fingerprint": finding.fingerprint,
                    "rule_id": finding.rule_id,
                    "ruleset_version": finding.ruleset_version,
                    "source_stale": false
                }),
            )?;
        }
        transaction.commit()?;
        build_workspace(&connection, order_id, preparation_date)
    }
}

fn build_workspace(
    connection: &Connection,
    order_id: i64,
    preparation_date: &str,
) -> Result<PreparationWorkspace, PreparationError> {
    let header =
        repository::load_header(connection, order_id)?.ok_or(PreparationError::OrderNotFound)?;
    let source_date = source_date_for_preparation(connection, &header, preparation_date)?;
    let sources = repository::load_source_items(connection, order_id)?;
    let mut excluded_item_count = 0_u64;
    let mut items = Vec::new();
    for source in sources {
        if !source_due_on_date(&source, header.treatment_time.as_deref(), &source_date) {
            continue;
        }
        let eligibility = evaluate_eligibility(header.editable, true, source.legacy_marker);
        if eligibility.status == EligibilityStatus::Excluded {
            excluded_item_count += 1;
            continue;
        }
        let task = repository::load_task_for_item_on_date(
            connection,
            source.snapshot.source_order_item_id,
            preparation_date,
        )?;
        let calculation = calculate_source(&source);
        let reference_quantity = reference_quantity(&calculation);
        let default_preparation_volume_ml = default_preparation_volume_ml(
            source.snapshot.diluent_volume_ml,
            calculation.withdrawal_volume_ml.as_deref(),
        );
        items.push(PreparationWorkspaceItem {
            order_item_id: source.snapshot.source_order_item_id,
            drug_id: source.snapshot.drug_id,
            drug_code: source.drug_code,
            drug_name: source.drug_name,
            ordered_dose_text: source.snapshot.ordered_dose_text,
            dose_unit_text: source.snapshot.dose_unit_text,
            diluent_name: source.snapshot.diluent_name,
            diluent_volume_ml: source.snapshot.diluent_volume_ml,
            route_name: source.snapshot.route_name,
            rate_text: source.snapshot.rate_text,
            treatment_day: source.snapshot.treatment_day,
            sequence_no: source.snapshot.sequence_no,
            regimen_details: source.snapshot.regimen_details,
            drug_detail: source.snapshot.drug_detail,
            drug_storage: source.snapshot.drug_storage,
            eligibility,
            reference_quantity,
            calculation,
            default_preparation_volume_ml,
            task,
        });
    }
    let safety = safety::evaluate_order(connection, order_id)?;
    let tasks_by_item = items
        .iter()
        .filter_map(|item| item.task.as_ref().map(|task| (item.order_item_id, task.id)))
        .collect::<HashMap<_, _>>();
    let safety_acknowledgements = repository::load_acknowledgements(connection, order_id)?
        .into_iter()
        .filter(|acknowledgement| {
            safety.findings.iter().any(|finding| {
                finding.id == acknowledgement.finding_id
                    && finding.fingerprint == acknowledgement.finding_fingerprint
                    && match finding.order_item_id {
                        Some(item_id) => {
                            acknowledgement.preparation_task_id
                                == tasks_by_item.get(&item_id).copied()
                        }
                        None => acknowledgement.preparation_task_id.is_none(),
                    }
            })
        })
        .collect();
    Ok(PreparationWorkspace {
        order_id: header.order_id,
        order_code: header.order_code,
        patient_hn: header.patient_hn,
        patient_name: header.patient_name,
        ward_name: header.ward_name,
        regimen_name: header.regimen_name,
        treatment_time: header.treatment_time,
        preparation_date: preparation_date.to_owned(),
        assigned_preparer: header.assigned_preparer,
        editable: header.editable,
        eligibility_rule_id: PREPARATION_ELIGIBILITY_RULE,
        excluded_item_count,
        pharmacists: repository::list_preparation_pharmacists(connection)?,
        items,
        safety,
        safety_acknowledgements,
    })
}

fn source_date_for_preparation(
    connection: &Connection,
    header: &repository::WorkspaceHeader,
    preparation_date: &str,
) -> Result<String, PreparationError> {
    if header.workflow_status != "active" {
        return Err(PreparationError::OrderOnHold);
    }
    if let Some(source_date) =
        repository::rescheduled_source_date(connection, header.order_id, preparation_date)?
    {
        return Ok(source_date);
    }
    if repository::is_no_show_date(connection, header.order_id, preparation_date)? {
        return Err(PreparationError::DateUnavailable);
    }
    if repository::is_suspended_date(connection, header.order_id, preparation_date)? {
        return Err(PreparationError::DateUnavailable);
    }
    Ok(preparation_date.to_owned())
}

fn calculate_source(
    source: &repository::WorkspaceSourceItem,
) -> crate::preparation_calc::PreparationCalculation {
    calculate_preparation(PreparationCalculationInput {
        ordered_dose_text: source.snapshot.ordered_dose_text.as_deref(),
        ordered_dose_unit: source.snapshot.dose_unit_text.as_deref(),
        amount_per_container: source.amount_per_container.as_deref(),
        presentation_unit: source.presentation_unit.as_deref(),
        volume_per_container_ml: source.volume_per_container_ml.as_deref(),
        package_label: source.package_label.as_deref(),
        legacy_stored_quantity: source.legacy_stored_quantity.as_deref(),
        inventory_tracking_enabled: source.inventory_tracking_enabled,
        current_inventory: source.current_inventory.as_deref(),
        minimum_inventory: source.minimum_inventory.as_deref(),
    })
}

fn default_preparation_volume_ml(
    diluent_volume_ml: Option<f64>,
    withdrawal_volume_ml: Option<&str>,
) -> Option<String> {
    let diluent_volume_ml = diluent_volume_ml.filter(|value| value.is_finite() && *value >= 0.0)?;
    let diluent = match LegacyDecimal::parse_access_subset(&diluent_volume_ml.to_string()) {
        DecimalParse::Parsed(value) => value,
        DecimalParse::NotNumeric | DecimalParse::Unsupported => return None,
    };
    let withdrawal = match LegacyDecimal::parse_access_subset(withdrawal_volume_ml?) {
        DecimalParse::Parsed(value)
            if value.compare_integer(0).is_some_and(|order| !order.is_lt()) =>
        {
            value
        }
        DecimalParse::Parsed(_) | DecimalParse::NotNumeric | DecimalParse::Unsupported => {
            return None
        }
    };
    diluent.checked_add(withdrawal)?.invariant_string()
}

fn validate_final_container_count(count: u32) -> Result<(), PreparationError> {
    if !(1..=20).contains(&count) {
        return Err(validation(
            "finalContainerCount",
            "Final container count must be between 1 and 20",
        ));
    }
    Ok(())
}

fn check_task_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    task_id: i64,
    actor_user_id: i64,
) -> Result<(), PreparationError> {
    let task =
        repository::load_task(transaction, task_id)?.ok_or(PreparationError::TaskNotFound)?;
    if task.state == PreparationState::Verified {
        return Ok(());
    }
    ensure_editable_order(transaction, task.source_order_id)?;
    ensure_current_source(transaction, &task)?;

    let prepared_by_user_id = if let Some(prepared_by) = task.prepared_by.as_ref() {
        prepared_by.id
    } else {
        let header = repository::load_header(transaction, task.source_order_id)?
            .ok_or(PreparationError::OrderNotFound)?;
        let assigned = header.assigned_preparer.ok_or_else(|| {
            validation(
                "assignedPreparerUserId",
                "Assign an active preparation pharmacist on the order before checking preparation",
            )
        })?;
        repository::load_preparation_pharmacist(transaction, assigned.id)?
            .ok_or_else(|| {
                validation(
                    "assignedPreparerUserId",
                    "The assigned preparation pharmacist is no longer active",
                )
            })?
            .id
    };
    let source = repository::load_source_items(transaction, task.source_order_id)?
        .into_iter()
        .find(|source| source.snapshot.source_order_item_id == task.source_order_item_id)
        .ok_or(PreparationError::StaleSource)?;
    let calculation = calculate_source(&source);
    let effective_volume = if task.preparation_volume_ml.is_none() {
        default_preparation_volume_ml(
            task.diluent_volume_ml,
            calculation.withdrawal_volume_ml.as_deref(),
        )
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
    } else {
        task.preparation_volume_ml
    };
    validate_final_container_count(task.final_container_count)?;
    repository::update_task(
        transaction,
        task.id,
        effective_volume,
        task.preparation_notes.as_deref(),
        calculation.withdrawal_volume_ml.as_deref(),
        task.final_container_count,
    )?;
    if repository::mark_checked(transaction, task_id, prepared_by_user_id, actor_user_id)? == 0 {
        return Err(PreparationError::NotPrepared);
    }
    let posting =
        post_inventory_decision(transaction, &task, &source, &calculation, actor_user_id)?;
    if let Some(movement_id) = posting.inventory_movement_id {
        audit::append_event(
            transaction,
            Some(actor_user_id),
            "preparation_inventory_issued",
            "inventory_movement",
            movement_id,
            &json!({
                "preparation_task_id": task.id,
                "inventory_movement_id": movement_id,
                "drug_id": task.drug_id,
                "quantity_delta": posting.quantity_delta,
                "containers_required": posting.containers_required,
                "balance_before": posting.balance_before,
                "balance_after": posting.balance_after,
                "calculation_rule_id": calculation.rule_id,
                "calculation_ruleset_version": calculation.ruleset_version,
                "workflow_rule_id": PREPARATION_INVENTORY_WORKFLOW_RULE
            }),
        )?;
    } else if posting.status == PreparationInventoryPostingStatus::ManualReconciliationRequired {
        audit::append_event(
            transaction,
            Some(actor_user_id),
            "preparation_inventory_reconciliation_required",
            "preparation_inventory_posting",
            posting.posting_id,
            &json!({
                "preparation_task_id": task.id,
                "drug_id": task.drug_id,
                "calculation_status": calculation.status.as_database(),
                "calculation_rule_id": calculation.rule_id,
                "calculation_ruleset_version": calculation.ruleset_version,
                "workflow_rule_id": PREPARATION_INVENTORY_WORKFLOW_RULE,
                "reason_code": posting.reason_code
            }),
        )?;
    }
    audit::append_event(
        transaction,
        Some(actor_user_id),
        "preparation_checked",
        "preparation_task",
        task_id,
        &json!({
            "state":"checked",
            "prepared_by_user_id":prepared_by_user_id,
            "source_stale":false,
            "inventory_posting_id":posting.posting_id,
            "inventory_posting_status":posting.status.as_database(),
            "workflow_rule_id":PREPARATION_INVENTORY_WORKFLOW_RULE
        }),
    )?;
    Ok(())
}

struct InventoryPostingOutcome {
    posting_id: i64,
    status: PreparationInventoryPostingStatus,
    inventory_movement_id: Option<i64>,
    containers_required: Option<i64>,
    quantity_delta: Option<f64>,
    balance_before: Option<f64>,
    balance_after: Option<f64>,
    reason_code: &'static str,
}

fn post_inventory_decision(
    transaction: &rusqlite::Transaction<'_>,
    task: &PreparationTask,
    source: &repository::WorkspaceSourceItem,
    calculation: &crate::preparation_calc::PreparationCalculation,
    actor_user_id: i64,
) -> Result<InventoryPostingOutcome, PreparationError> {
    let calculation_status = calculation.status.as_database();
    let parsed_containers = calculation
        .containers_required
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| (0..=MAX_EXACT_CONTAINER_INTEGER).contains(value));

    let decision = if calculation.status
        != crate::preparation_calc::PreparationCalculationStatus::Calculated
    {
        PostingDecision::without_movement(
            PreparationInventoryPostingStatus::ManualReconciliationRequired,
            parsed_containers,
            "calculation_not_fully_supported",
        )
    } else if parsed_containers.is_none() {
        PostingDecision::without_movement(
            PreparationInventoryPostingStatus::ManualReconciliationRequired,
            None,
            "container_requirement_invalid",
        )
    } else if parsed_containers == Some(0) {
        PostingDecision::without_movement(
            PreparationInventoryPostingStatus::NotRequired,
            Some(0),
            "zero_containers_required",
        )
    } else if !source.inventory_tracking_enabled {
        PostingDecision::without_movement(
            PreparationInventoryPostingStatus::TrackingDisabled,
            parsed_containers,
            "inventory_tracking_disabled",
        )
    } else if inventory::current_balance(transaction, task.drug_id)?.is_none() {
        PostingDecision::without_movement(
            PreparationInventoryPostingStatus::ManualReconciliationRequired,
            parsed_containers,
            "inventory_balance_unavailable",
        )
    } else {
        let containers = parsed_containers.expect("positive accepted count was checked");
        let balance_before = inventory::current_balance(transaction, task.drug_id)?
            .expect("the authoritative balance was checked");
        let quantity_delta = -(containers as f64);
        let occurred_at: String =
            transaction.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now')", [], |row| {
                row.get(0)
            })?;
        let task_reference = task.id.to_string();
        let movement_id = inventory::insert_movement(
            transaction,
            &NewMovement {
                drug_id: task.drug_id,
                movement_type: InventoryMovementType::PreparationIssue,
                quantity_delta,
                occurred_at: &occurred_at,
                actor_user_id,
                reference_type: Some("preparation_task"),
                reference_id: Some(&task_reference),
                note: None,
                preparation_task_id: Some(task.id),
            },
        )?;
        let balance_after = inventory::current_balance(transaction, task.drug_id)?
            .expect("a newly inserted movement always produces a balance");
        let minimum = source
            .minimum_inventory
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok());
        let resulting_stock_state = match inventory::stock_state(true, Some(balance_after), minimum)
        {
            StockState::Normal => PreparationIssueStockState::Normal,
            StockState::Low => PreparationIssueStockState::Low,
            StockState::Out => PreparationIssueStockState::Out,
            StockState::Shortage => PreparationIssueStockState::Shortage,
            StockState::Unknown | StockState::Untracked => {
                unreachable!("a tracked movement always has a known balance")
            }
        };
        PostingDecision {
            status: PreparationInventoryPostingStatus::Posted,
            movement_id: Some(movement_id),
            containers_required: Some(containers),
            balance_before: Some(balance_before),
            balance_after: Some(balance_after),
            resulting_stock_state: Some(resulting_stock_state),
            reason_code: "supported_container_issue",
        }
    };

    let posting_id = repository::insert_inventory_posting(
        transaction,
        &repository::NewPreparationInventoryPosting {
            preparation_task_id: task.id,
            status: decision.status,
            inventory_movement_id: decision.movement_id,
            containers_required: decision.containers_required,
            balance_before: decision.balance_before,
            balance_after: decision.balance_after,
            resulting_stock_state: decision.resulting_stock_state,
            calculation_status,
            calculation_ruleset_version: calculation.ruleset_version,
            calculation_rule_id: calculation.rule_id,
            workflow_rule_id: PREPARATION_INVENTORY_WORKFLOW_RULE,
            reason_code: decision.reason_code,
            actor_user_id,
        },
    )?;
    Ok(InventoryPostingOutcome {
        posting_id,
        status: decision.status,
        inventory_movement_id: decision.movement_id,
        containers_required: decision.containers_required,
        quantity_delta: decision.containers_required.map(|value| -(value as f64)),
        balance_before: decision.balance_before,
        balance_after: decision.balance_after,
        reason_code: decision.reason_code,
    })
}

struct PostingDecision {
    status: PreparationInventoryPostingStatus,
    movement_id: Option<i64>,
    containers_required: Option<i64>,
    balance_before: Option<f64>,
    balance_after: Option<f64>,
    resulting_stock_state: Option<PreparationIssueStockState>,
    reason_code: &'static str,
}

impl PostingDecision {
    fn without_movement(
        status: PreparationInventoryPostingStatus,
        containers_required: Option<i64>,
        reason_code: &'static str,
    ) -> Self {
        Self {
            status,
            movement_id: None,
            containers_required,
            balance_before: None,
            balance_after: None,
            resulting_stock_state: None,
            reason_code,
        }
    }
}

fn load_mutable_task(
    connection: &Connection,
    task_id: i64,
) -> Result<PreparationTask, PreparationError> {
    let task = repository::load_task(connection, task_id)?.ok_or(PreparationError::TaskNotFound)?;
    if task.state == PreparationState::Verified {
        return Err(PreparationError::VerifiedReadOnly);
    }
    ensure_editable_order(connection, task.source_order_id)?;
    Ok(task)
}

fn ensure_editable_order(connection: &Connection, order_id: i64) -> Result<(), PreparationError> {
    let header =
        repository::load_header(connection, order_id)?.ok_or(PreparationError::OrderNotFound)?;
    if header.editable {
        Ok(())
    } else {
        Err(PreparationError::HistoricalReadOnly)
    }
}

fn ensure_current_source(
    connection: &Connection,
    task: &PreparationTask,
) -> Result<(), PreparationError> {
    let current = repository::load_source_snapshot(connection, task.source_order_item_id)?
        .ok_or(PreparationError::StaleSource)?;
    if current == task.snapshot() {
        Ok(())
    } else {
        Err(PreparationError::StaleSource)
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

#[cfg(test)]
fn default_preparation_date(
    connection: &Connection,
    order_id: i64,
) -> Result<String, PreparationError> {
    let header =
        repository::load_header(connection, order_id)?.ok_or(PreparationError::OrderNotFound)?;
    if let Some(value) = header.treatment_time.as_deref().and_then(date_part) {
        return Ok(value.to_owned());
    }
    Ok(connection.query_row("SELECT date('now','+7 hours')", [], |row| row.get(0))?)
}

fn source_due_on_date(
    source: &repository::WorkspaceSourceItem,
    order_time: Option<&str>,
    preparation_date: &str,
) -> bool {
    let start = source.snapshot.start_date.as_deref().and_then(date_part);
    let stop = source.snapshot.stop_date.as_deref().and_then(date_part);
    match (start, stop) {
        (Some(start), Some(stop)) => start <= preparation_date && preparation_date <= stop,
        (Some(start), None) => start == preparation_date,
        (None, Some(stop)) => stop == preparation_date,
        (None, None) => order_time.and_then(date_part) == Some(preparation_date),
    }
}

fn date_part(value: &str) -> Option<&str> {
    let value = value.get(..10)?;
    valid_iso_date(value).then_some(value)
}

fn validate_preparation_date(value: &str) -> Result<(), PreparationError> {
    if valid_iso_date(value) {
        Ok(())
    } else {
        Err(validation(
            "preparationDate",
            "Use a valid preparation date in YYYY-MM-DD format",
        ))
    }
}

fn validate_date_range(from: Option<&str>, to: Option<&str>) -> Result<(), PreparationError> {
    for (field, value) in [("dateFrom", from), ("dateTo", to)] {
        if value.is_some_and(|value| !valid_iso_date(value)) {
            return Err(validation(field, "Use a valid date in YYYY-MM-DD format"));
        }
    }
    if let (Some(from), Some(to)) = (from, to) {
        if to < from {
            return Err(validation("dateTo", "To date cannot be before from date"));
        }
    }
    Ok(())
}

fn valid_iso_date(value: &str) -> bool {
    let parts = value
        .split('-')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>();
    let Ok(parts) = parts else {
        return false;
    };
    if parts.len() != 3 || value.len() != 10 {
        return false;
    }
    let (year, month, day) = (parts[0], parts[1], parts[2]);
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days).contains(&day)
}

fn validation(field: &'static str, message: impl Into<String>) -> PreparationError {
    PreparationError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    use crate::{
        auth::{
            AuthService, AuthSession, BootstrapUserInput, CreateUserInput, LoginInput, UserType,
        },
        safety::SafetyEvaluationMode,
    };

    #[test]
    fn default_final_volume_adds_diluent_and_withdrawal_exactly() {
        assert_eq!(
            default_preparation_volume_ml(Some(100.0), Some("20")),
            Some("120".into())
        );
        assert_eq!(
            default_preparation_volume_ml(Some(0.1), Some("0.2")),
            Some("0.3".into())
        );
        assert_eq!(default_preparation_volume_ml(Some(100.0), None), None);
        assert_eq!(default_preparation_volume_ml(None, Some("20")), None);
    }

    struct Fixture {
        _directory: tempfile::TempDir,
        database: Database,
        session: AuthSession,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
            let connection = database.open().unwrap();
            connection.execute("INSERT INTO patients(id,legacy_hn,first_name,last_name) VALUES(1,'SYN-HN','ผู้ป่วย','ทดสอบ')", []).unwrap();
            connection
                .execute(
                    "INSERT INTO units(id,legacy_unitcode,unit_name) VALUES(1,'MG','mg')",
                    [],
                )
                .unwrap();
            connection.execute("INSERT INTO diluents(id,legacy_dilcode,diluent_name,volume_ml) VALUES(1,'DIL','สารละลายทดสอบ',1)", []).unwrap();
            connection
                .execute(
                    "INSERT INTO routes(id,legacy_rcode,route_name) VALUES(1,'IV','IV')",
                    [],
                )
                .unwrap();
            connection.execute("INSERT INTO regimens(id,legacy_regcode,regimen_name) VALUES(1,'REG','สูตรสังเคราะห์')", []).unwrap();
            connection.execute("INSERT INTO wards(id,legacy_wcode,ward_name) VALUES(1,'WARD-SYN','หอผู้ป่วยสังเคราะห์')", []).unwrap();
            connection.execute("INSERT INTO drugs(id,legacy_dcode,drug_name,unit_id,dose_per_pack,volume_per_pack_ml,marker,detail,storage,max_dilution_alert,max_dilution_hard) VALUES(1,'PREP','Synthetic preparation agent',1,50,10,1,'เตรียมแบบทดสอบ','เก็บแบบทดสอบ',1,1)", []).unwrap();
            connection.execute("INSERT INTO drugs(id,legacy_dcode,drug_name,unit_id,marker) VALUES(2,'ROUTINE','Synthetic routine supportive medication',1,0)", []).unwrap();
            connection.execute("INSERT INTO drugs(id,legacy_dcode,drug_name,unit_id,dose_per_pack,volume_per_pack_ml,marker) VALUES(3,'ADJUNCT','Synthetic IV mesna-like protocol adjunct',1,100,10,1)", []).unwrap();
            connection.execute("INSERT INTO orders(id,legacy_orderid,patient_id,ward_id,regimen_id,order_time,oncoflow_created) VALUES(10,'OF-SYN-10',1,1,1,'2026-08-23T09:00',1)", []).unwrap();
            connection.execute("INSERT INTO order_items(id,order_id,drug_id,diluent_id,route_id,dose,legacy_dose_text,rate,ordering_no,regimen_unit_text,regimen_details,regimen_start_day) VALUES(11,10,1,1,1,100,'100','60 min',1,'mg','คำแนะนำสังเคราะห์',1)", []).unwrap();
            connection.execute("INSERT INTO order_items(id,order_id,drug_id,dose,legacy_dose_text,ordering_no) VALUES(12,10,2,8,'8',2)", []).unwrap();
            connection.execute("INSERT INTO order_items(id,order_id,drug_id,dose,legacy_dose_text,ordering_no,regimen_unit_text) VALUES(13,10,3,50,'50',3,'mg')", []).unwrap();
            connection.execute("INSERT INTO orders(id,legacy_orderid,patient_id,oncoflow_created) VALUES(50,'LEGACY-SYN',1,0)", []).unwrap();
            connection.execute("INSERT INTO order_items(id,order_id,drug_id,dose,ordering_no) VALUES(50,50,1,75,1)", []).unwrap();
            drop(connection);
            let session = AuthSession::default();
            AuthService::new(&database, &session)
                .bootstrap(BootstrapUserInput {
                    username: "synthetic.pharmacist".into(),
                    display_name: "เภสัชกรสังเคราะห์".into(),
                    password: "synthetic preparation password 42!".into(),
                })
                .unwrap();
            database
                .open()
                .unwrap()
                .execute(
                    "UPDATE orders SET assigned_preparer_user_id=1 WHERE id=10",
                    [],
                )
                .unwrap();
            Self {
                _directory: directory,
                database,
                session,
            }
        }

        fn service(&self) -> PreparationService<'_> {
            PreparationService::new(&self.database, &self.session)
        }

        fn initialize(&self) -> PreparationWorkspace {
            self.service().initialize(10).unwrap()
        }

        fn prepare_tracked_adjunct(&self, opening_balance: f64, ordered_dose: f64) -> i64 {
            let connection = self.database.open().unwrap();
            connection
                .execute(
                    "UPDATE drugs
                     SET inventory_enabled=1,inventory_min=1,package='Amp.'
                     WHERE id=3",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    "UPDATE order_items
                     SET dose=?1,legacy_dose_text=?2,number_of_drug=999
                     WHERE id=13",
                    params![ordered_dose, ordered_dose.to_string()],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO inventory_movements(
                        drug_id,movement_type,quantity_delta,reference_type,reference_id,note
                     ) VALUES(3,'opening_balance',?1,'synthetic_fixture','ADJUNCT',
                              'Synthetic opening only')",
                    [opening_balance],
                )
                .unwrap();
            drop(connection);
            let workspace = self.initialize();
            let task_id = workspace
                .items
                .iter()
                .find(|item| item.order_item_id == 13)
                .unwrap()
                .task
                .as_ref()
                .unwrap()
                .id;
            self.service().mark_prepared(task_id).unwrap();
            task_id
        }
    }

    #[test]
    fn final_container_count_is_user_defined_without_changing_dose_or_volume() {
        let fixture = Fixture::new();
        let task_id = fixture.initialize().items[0].task.as_ref().unwrap().id;
        let changed = fixture
            .service()
            .update_task(
                task_id,
                PreparationTaskInput {
                    preparation_volume_ml: Some(120.0),
                    preparation_notes: None,
                    final_container_count: Some(2),
                },
            )
            .unwrap();
        assert_eq!(changed.final_container_count, 2);
        assert_eq!(changed.ordered_dose_text.as_deref(), Some("100"));
        assert_eq!(changed.preparation_volume_ml, Some(120.0));

        assert!(matches!(
            fixture.service().update_task(
                task_id,
                PreparationTaskInput {
                    preparation_volume_ml: Some(120.0),
                    preparation_notes: None,
                    final_container_count: Some(21),
                }
            ),
            Err(PreparationError::Validation {
                field: "finalContainerCount",
                ..
            })
        ));
    }

    #[test]
    fn assistant_pharmacist_checks_pending_task_with_order_assigned_preparer() {
        let fixture = Fixture::new();
        AuthService::new(&fixture.database, &fixture.session)
            .create_user(CreateUserInput {
                username: "synthetic.assistant".into(),
                display_name: "ผู้ช่วยเภสัชกรสังเคราะห์".into(),
                password: "synthetic assistant password 42!".into(),
                user_type: UserType::NonPharmacist,
            })
            .unwrap();
        let workspace = fixture.initialize();
        let task_id = workspace.items[0].task.as_ref().unwrap().id;
        AuthService::new(&fixture.database, &fixture.session)
            .logout()
            .unwrap();
        let assistant = AuthService::new(&fixture.database, &fixture.session)
            .login(LoginInput {
                username: "synthetic.assistant".into(),
                password: "synthetic assistant password 42!".into(),
            })
            .unwrap()
            .current_user
            .unwrap();

        let checked = fixture.service().check(task_id).unwrap();
        assert_eq!(checked.state, PreparationState::Verified);
        assert_eq!(checked.prepared_by.as_ref().map(|value| value.id), Some(1));
        assert_eq!(
            checked.verified_by.as_ref().map(|value| value.id),
            Some(assistant.id)
        );
        let connection = fixture.database.open().unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM audit_events WHERE event_type='preparation_checked' AND user_id=?1",
                    [assistant.id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
        let movement_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM inventory_movements WHERE preparation_task_id=?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap();
        let checked_again = fixture.service().check(task_id).unwrap();
        assert_eq!(checked_again.id, task_id);
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM inventory_movements WHERE preparation_task_id=?1",
                    [task_id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            movement_count
        );
    }

    #[test]
    fn check_transition_inventory_posting_and_audit_are_atomic() {
        let fixture = Fixture::new();
        let task_id = fixture.initialize().items[0].task.as_ref().unwrap().id;
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_check_audit_failure BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='preparation_checked'
                 BEGIN SELECT RAISE(ABORT,'synthetic check audit failure'); END;",
            )
            .unwrap();

        assert!(matches!(
            fixture.service().check(task_id),
            Err(PreparationError::Sqlite(_))
        ));

        let connection = fixture.database.open().unwrap();
        let row: (String, Option<i64>, Option<i64>, i64, i64, i64) = connection
            .query_row(
                "SELECT t.state,t.prepared_by_user_id,t.verified_by_user_id,
                        (SELECT COUNT(*) FROM inventory_movements WHERE preparation_task_id=t.id),
                        (SELECT COUNT(*) FROM preparation_inventory_postings WHERE preparation_task_id=t.id),
                        (SELECT COUNT(*) FROM audit_events WHERE event_type='preparation_checked')
                 FROM preparation_tasks t WHERE t.id=?1",
                [task_id],
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
        assert_eq!(row, ("pending".into(), None, None, 0, 0, 0));
    }

    #[test]
    fn batch_check_marks_every_task_once_and_is_idempotent() {
        let fixture = Fixture::new();
        let task_ids = fixture
            .initialize()
            .items
            .into_iter()
            .filter_map(|item| item.task.map(|task| task.id))
            .collect::<Vec<_>>();

        let checked = fixture.service().check_batch(task_ids.clone()).unwrap();
        assert_eq!(checked.len(), task_ids.len());
        assert!(checked
            .iter()
            .all(|task| task.state == PreparationState::Verified));

        let connection = fixture.database.open().unwrap();
        let posting_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM preparation_inventory_postings",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM audit_events WHERE event_type='preparation_checked'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);

        fixture.service().check_batch(task_ids).unwrap();
        let connection = fixture.database.open().unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM preparation_inventory_postings",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            posting_count
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM audit_events WHERE event_type='preparation_checked'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            audit_count
        );
    }

    #[test]
    fn batch_check_rolls_back_every_task_when_one_fails() {
        let fixture = Fixture::new();
        let task_ids = fixture
            .initialize()
            .items
            .into_iter()
            .filter_map(|item| item.task.map(|task| task.id))
            .collect::<Vec<_>>();
        let failing_task_id = *task_ids.last().unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(&format!(
                "CREATE TRIGGER synthetic_batch_check_failure BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='preparation_checked'
                  AND NEW.entity_id='{failing_task_id}'
                 BEGIN SELECT RAISE(ABORT,'synthetic batch check failure'); END;"
            ))
            .unwrap();

        assert!(matches!(
            fixture.service().check_batch(task_ids),
            Err(PreparationError::Sqlite(_))
        ));

        let connection = fixture.database.open().unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM preparation_tasks WHERE state!='pending'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM preparation_inventory_postings",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM audit_events WHERE event_type='preparation_checked'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn queue_and_workspace_include_marked_agents_and_exclude_routine_supportive_items() {
        let fixture = Fixture::new();
        let queue = fixture
            .service()
            .list_queue(PreparationQueueRequest {
                search: Some("ผู้ป่วย".into()),
                ..PreparationQueueRequest::default()
            })
            .unwrap();
        assert_eq!(queue.total, 1);
        assert_eq!(queue.items[0].eligible_item_count, 2);
        assert_eq!(queue.items[0].initialized_item_count, 0);
        assert_eq!(queue.items[0].ward_name.as_deref(), Some("หอผู้ป่วยสังเคราะห์"));

        let workspace = fixture.service().get_workspace(10).unwrap();
        assert_eq!(workspace.items.len(), 2);
        assert_eq!(workspace.ward_name.as_deref(), Some("หอผู้ป่วยสังเคราะห์"));
        assert_eq!(workspace.excluded_item_count, 1);
        assert!(workspace
            .items
            .iter()
            .any(|item| item.drug_code == "ADJUNCT"));
        assert!(!workspace
            .items
            .iter()
            .any(|item| item.drug_code == "ROUTINE"));
        assert_eq!(workspace.eligibility_rule_id, PREPARATION_ELIGIBILITY_RULE);
    }

    #[test]
    fn initialization_snapshots_the_order_item_diluent_volume_override() {
        let fixture = Fixture::new();
        fixture
            .database
            .open()
            .unwrap()
            .execute(
                "UPDATE order_items SET diluent_volume_ml=250.5 WHERE id=11",
                [],
            )
            .unwrap();

        let workspace = fixture.initialize();
        let item = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap();

        assert_eq!(item.task.as_ref().unwrap().diluent_volume_ml, Some(250.5));
    }

    #[test]
    fn initializes_edits_and_verifies_preparation_with_explicit_safety_review() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        assert_eq!(workspace.items.len(), 2);
        assert!(workspace.items.iter().all(|item| item.task.is_some()));
        let first = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap();
        assert_eq!(
            first.reference_quantity.drug_solution_volume_ml.as_deref(),
            Some("20")
        );
        assert_eq!(first.default_preparation_volume_ml.as_deref(), Some("21"));
        assert_eq!(workspace.safety.mode, SafetyEvaluationMode::Active);
        let warning = workspace
            .safety
            .findings
            .iter()
            .find(|finding| finding.order_item_id == Some(11) && finding.acknowledgement_required)
            .unwrap();
        let task_id = first.task.as_ref().unwrap().id;

        let changed = fixture
            .service()
            .update_task(
                task_id,
                PreparationTaskInput {
                    preparation_volume_ml: Some(121.5),
                    preparation_notes: Some("  บันทึกการเตรียม  ".into()),
                    ..PreparationTaskInput::default()
                },
            )
            .unwrap();
        assert_eq!(changed.preparation_volume_ml, Some(121.5));
        assert_eq!(changed.preparation_notes.as_deref(), Some("บันทึกการเตรียม"));
        let prepared = fixture.service().mark_prepared(task_id).unwrap();
        assert_eq!(prepared.state, PreparationState::Prepared);
        assert!(matches!(
            fixture.service().verify(task_id),
            Err(PreparationError::SafetyReviewRequired { .. })
        ));
        let acknowledged = fixture
            .service()
            .acknowledge_safety_finding(10, warning.id.clone())
            .unwrap();
        assert!(acknowledged
            .safety_acknowledgements
            .iter()
            .any(|value| value.finding_fingerprint == warning.fingerprint));
        let verified = fixture.service().verify(task_id).unwrap();
        assert_eq!(verified.state, PreparationState::Verified);
        assert!(verified.verified_at.is_some());
        assert!(verified.prepared_by.is_some());
        assert!(verified.verified_by.is_some());
        assert_eq!(verified.prepared_by, verified.verified_by);
        let frozen_withdrawal: Option<String> = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT withdrawal_volume_ml FROM preparation_tasks WHERE id=?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(frozen_withdrawal.as_deref(), Some("20"));
        assert!(matches!(
            fixture
                .service()
                .update_task(task_id, PreparationTaskInput::default()),
            Err(PreparationError::VerifiedReadOnly)
        ));
    }

    #[test]
    fn initialization_is_idempotent_and_supports_null_optional_values() {
        let fixture = Fixture::new();
        fixture.initialize();
        let second = fixture.initialize();
        assert_eq!(second.items.len(), 2);
        let count: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM preparation_tasks", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);
        let adjunct = second
            .items
            .iter()
            .find(|item| item.order_item_id == 13)
            .unwrap()
            .task
            .as_ref()
            .unwrap();
        assert!(adjunct.diluent_id.is_none());
        assert!(adjunct.preparation_volume_ml.is_none());
        assert!(adjunct.preparation_notes.is_none());
    }

    #[test]
    fn continuing_order_creates_independent_tasks_for_each_treatment_date() {
        let fixture = Fixture::new();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "UPDATE order_items
                 SET start_date='2026-08-23',stop_date='2026-08-27'
                 WHERE id=11;
                 UPDATE order_items
                 SET start_date='2026-08-23',stop_date='2026-08-23'
                 WHERE id=13;",
            )
            .unwrap();

        let queue = fixture
            .service()
            .list_queue(PreparationQueueRequest {
                preparation_date: Some("2026-08-25".into()),
                source_filter: super::super::PreparationQueueSourceFilter::Continuing,
                ..PreparationQueueRequest::default()
            })
            .unwrap();
        assert_eq!(queue.total, 1);
        assert_eq!(queue.items[0].eligible_item_count, 1);
        assert_eq!(
            queue.items[0].source_kind,
            super::super::PreparationQueueSourceFilter::Continuing
        );

        let order_day = fixture
            .service()
            .list_queue(PreparationQueueRequest {
                preparation_date: Some("2026-08-23".into()),
                source_filter: super::super::PreparationQueueSourceFilter::SameDay,
                ..PreparationQueueRequest::default()
            })
            .unwrap();
        assert_eq!(order_day.total, 1);
        assert_eq!(order_day.items[0].eligible_item_count, 2);
        assert_eq!(
            order_day.items[0].source_kind,
            super::super::PreparationQueueSourceFilter::SameDay
        );

        let same_day = fixture
            .service()
            .list_queue(PreparationQueueRequest {
                preparation_date: Some("2026-08-25".into()),
                source_filter: super::super::PreparationQueueSourceFilter::SameDay,
                ..PreparationQueueRequest::default()
            })
            .unwrap();
        assert_eq!(same_day.total, 0);

        let day_three = fixture
            .service()
            .initialize_for_date(10, "2026-08-25".into())
            .unwrap();
        assert_eq!(day_three.preparation_date, "2026-08-25");
        assert_eq!(day_three.items.len(), 1);
        let day_three_task = day_three.items[0].task.as_ref().unwrap();
        assert_eq!(day_three_task.preparation_date, "2026-08-25");

        let repeated = fixture
            .service()
            .initialize_for_date(10, "2026-08-25".into())
            .unwrap();
        assert_eq!(
            repeated.items[0].task.as_ref().unwrap().id,
            day_three_task.id
        );

        let day_four = fixture
            .service()
            .initialize_for_date(10, "2026-08-26".into())
            .unwrap();
        let day_four_task = day_four.items[0].task.as_ref().unwrap();
        assert_eq!(day_four_task.preparation_date, "2026-08-26");
        assert_ne!(day_four_task.id, day_three_task.id);
        assert_eq!(day_four_task.state, PreparationState::Pending);

        fixture.service().mark_prepared(day_three_task.id).unwrap();
        let unchanged_day_four =
            repository::load_task(&fixture.database.open().unwrap(), day_four_task.id)
                .unwrap()
                .unwrap();
        assert_eq!(unchanged_day_four.state, PreparationState::Pending);

        fixture.service().check(day_three_task.id).unwrap();
        fixture.service().check(day_four_task.id).unwrap();
        let posting_count: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM preparation_inventory_postings
                 WHERE preparation_task_id IN (?1,?2)",
                params![day_three_task.id, day_four_task.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(posting_count, 2);
    }

    #[test]
    fn no_show_leaves_the_queue_and_reschedule_uses_original_due_items_without_date_mutation() {
        let fixture = Fixture::new();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "UPDATE order_items
                 SET start_date='2026-08-23',stop_date='2026-08-27'
                 WHERE id=11;
                 UPDATE order_items
                 SET start_date='2026-08-23',stop_date='2026-08-23'
                 WHERE id=13;",
            )
            .unwrap();
        let actor = fixture.session.require_user().unwrap();
        let order_service = crate::order::OrderService::new(&fixture.database);
        order_service
            .record_no_show(
                10,
                crate::order::OrderNoShowInput {
                    scheduled_date: "2026-08-25".into(),
                },
                actor.id,
            )
            .unwrap();

        let held_queue = fixture
            .service()
            .list_queue(PreparationQueueRequest {
                preparation_date: Some("2026-08-25".into()),
                ..PreparationQueueRequest::default()
            })
            .unwrap();
        assert_eq!(held_queue.total, 0);
        assert!(matches!(
            fixture
                .service()
                .get_workspace_for_date(10, "2026-08-25".into()),
            Err(PreparationError::OrderOnHold)
        ));

        order_service
            .reschedule(
                10,
                crate::order::OrderRescheduleInput {
                    missed_date: "2026-08-25".into(),
                    new_date: "2026-08-30".into(),
                },
                actor.id,
            )
            .unwrap();
        let queue = fixture
            .service()
            .list_queue(PreparationQueueRequest {
                preparation_date: Some("2026-08-30".into()),
                ..PreparationQueueRequest::default()
            })
            .unwrap();
        assert_eq!(queue.total, 1);
        assert_eq!(queue.items[0].eligible_item_count, 1);
        assert_eq!(
            queue.items[0].source_kind,
            super::super::PreparationQueueSourceFilter::Rescheduled
        );
        let suspended_queue = fixture
            .service()
            .list_queue(PreparationQueueRequest {
                preparation_date: Some("2026-08-26".into()),
                ..PreparationQueueRequest::default()
            })
            .unwrap();
        assert_eq!(suspended_queue.total, 0);
        assert!(matches!(
            fixture
                .service()
                .get_workspace_for_date(10, "2026-08-26".into()),
            Err(PreparationError::DateUnavailable)
        ));
        let workspace = fixture
            .service()
            .initialize_for_date(10, "2026-08-30".into())
            .unwrap();
        assert_eq!(workspace.items.len(), 1);
        let task = workspace.items[0].task.as_ref().unwrap();
        assert_eq!(task.preparation_date, "2026-08-30");
        assert_eq!(task.start_date.as_deref(), Some("2026-08-23"));
        assert_eq!(task.stop_date.as_deref(), Some("2026-08-27"));
        assert!(matches!(
            fixture
                .service()
                .get_workspace_for_date(10, "2026-08-25".into()),
            Err(PreparationError::DateUnavailable)
        ));
    }

    #[test]
    fn selected_preparation_pharmacist_is_distinct_from_the_audit_actor() {
        let fixture = Fixture::new();
        let audit_actor = fixture.session.require_user().unwrap();
        let connection = fixture.database.open().unwrap();
        connection
            .execute(
                "INSERT INTO users(
                    id,username,display_name,password_hash,role,user_type,active,
                    credential_kind,updated_at
                 ) VALUES(
                    99,'actual.preparer','Actual preparation pharmacist',
                    '$argon2id$synthetic','user','pharmacist',1,'argon2id',CURRENT_TIMESTAMP
                 )",
                [],
            )
            .unwrap();
        drop(connection);

        let workspace = fixture.initialize();
        assert!(workspace.pharmacists.iter().any(|user| user.id == 99));
        let task_id = workspace.items[0].task.as_ref().unwrap().id;
        let prepared = fixture.service().mark_prepared_for(task_id, 99).unwrap();
        assert_eq!(prepared.prepared_by.as_ref().map(|user| user.id), Some(99));

        let connection = fixture.database.open().unwrap();
        let (stored_preparer, audit_user, metadata): (i64, i64, String) = connection
            .query_row(
                "SELECT t.prepared_by_user_id,a.user_id,a.metadata_json
                 FROM preparation_tasks t
                 JOIN audit_events a
                   ON a.event_type='preparation_marked_prepared'
                  AND a.entity_id=CAST(t.id AS TEXT)
                 WHERE t.id=?1",
                [task_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(stored_preparer, 99);
        assert_eq!(audit_user, audit_actor.id);
        assert_ne!(audit_user, stored_preparer);
        assert!(metadata.contains("\"prepared_by_user_id\":99"));
    }

    #[test]
    fn inactive_or_unknown_user_cannot_be_recorded_as_preparation_pharmacist() {
        let fixture = Fixture::new();
        let task_id = fixture.initialize().items[0].task.as_ref().unwrap().id;
        assert!(matches!(
            fixture.service().mark_prepared_for(task_id, 99_999),
            Err(PreparationError::Validation {
                field: "preparedByUserId",
                ..
            })
        ));
        let task = repository::load_task(&fixture.database.open().unwrap(), task_id)
            .unwrap()
            .unwrap();
        assert_eq!(task.state, PreparationState::Pending);
        assert!(task.prepared_by.is_none());
    }

    #[test]
    fn invalid_and_historical_orders_are_rejected_without_backfill() {
        let fixture = Fixture::new();
        assert!(matches!(
            fixture.service().initialize(999),
            Err(PreparationError::OrderNotFound)
        ));
        let before: String = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT quote(dose)||'|'||quote(ordering_no) FROM order_items WHERE id=50",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let historical = fixture.service().get_workspace(50).unwrap();
        assert!(!historical.editable);
        assert!(historical.items.is_empty());
        assert_eq!(
            historical.safety.mode,
            SafetyEvaluationMode::HistoricalNotEvaluated
        );
        assert!(matches!(
            fixture.service().initialize(50),
            Err(PreparationError::HistoricalReadOnly)
        ));
        let after: String = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT quote(dose)||'|'||quote(ordering_no) FROM order_items WHERE id=50",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn initialization_returns_an_empty_workspace_when_the_order_has_no_drug_items() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        connection
            .execute("DELETE FROM order_items WHERE order_id=10", [])
            .unwrap();
        drop(connection);

        let workspace = fixture.service().initialize(10).unwrap();

        assert!(workspace.items.is_empty());
        let connection = fixture.database.open().unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM preparation_tasks", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM audit_events WHERE event_type='preparation_created'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn source_changes_are_detected_and_never_silently_copied() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let task_id = workspace.items[0].task.as_ref().unwrap().id;
        fixture
            .database
            .open()
            .unwrap()
            .execute(
                "UPDATE order_items SET legacy_dose_text='changed' WHERE id=11",
                [],
            )
            .unwrap();
        assert!(matches!(
            fixture.service().mark_prepared(task_id),
            Err(PreparationError::StaleSource)
        ));
        let task = repository::load_task(&fixture.database.open().unwrap(), task_id)
            .unwrap()
            .unwrap();
        assert_eq!(task.ordered_dose_text.as_deref(), Some("100"));
        assert_eq!(task.state, PreparationState::Pending);
    }

    #[test]
    fn initialization_refreshes_a_pending_task_after_the_order_changes() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let task_id = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;
        fixture
            .service()
            .update_task(
                task_id,
                PreparationTaskInput {
                    preparation_volume_ml: Some(21.0),
                    preparation_notes: Some("old preparation details".into()),
                    ..PreparationTaskInput::default()
                },
            )
            .unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute("UPDATE order_items SET rate='20 min' WHERE id=11", [])
            .unwrap();

        let refreshed = fixture.initialize();
        let task = refreshed
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap()
            .task
            .as_ref()
            .unwrap();

        assert_eq!(task.id, task_id);
        assert_eq!(task.state, PreparationState::Pending);
        assert_eq!(task.rate_text.as_deref(), Some("20 min"));
        assert_eq!(task.preparation_volume_ml, None);
        assert_eq!(task.preparation_notes, None);
        fixture.service().mark_prepared(task_id).unwrap();
        assert_eq!(
            fixture
                .database
                .open()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM audit_events
                     WHERE event_type='preparation_source_refreshed' AND entity_id=?1",
                    [task_id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn initialization_does_not_refresh_a_prepared_task_after_the_order_changes() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let task_id = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;
        fixture.service().mark_prepared(task_id).unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute("UPDATE order_items SET rate='20 min' WHERE id=11", [])
            .unwrap();

        let reopened = fixture.initialize();
        let task = reopened
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap()
            .task
            .as_ref()
            .unwrap();

        assert_eq!(task.state, PreparationState::Prepared);
        assert_eq!(task.rate_text.as_deref(), Some("60 min"));
        assert!(matches!(
            fixture.service().verify(task_id),
            Err(PreparationError::StaleSource)
        ));
    }

    #[test]
    fn batch_check_records_the_default_final_volume_when_none_was_entered() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let task_id = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;

        let checked = fixture.service().check_batch(vec![task_id]).unwrap();

        assert_eq!(checked[0].preparation_volume_ml, Some(21.0));
        assert_eq!(checked[0].state, PreparationState::Verified);
    }

    #[test]
    fn task_creation_rolls_back_when_any_eligible_item_fails() {
        let fixture = Fixture::new();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_preparation_failure BEFORE INSERT ON preparation_tasks
             WHEN NEW.source_order_item_id=13 BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().initialize(10),
            Err(PreparationError::Sqlite(_))
        ));
        let count: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM preparation_tasks", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn failed_safety_evaluation_rolls_back_verification() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let task_id = workspace.items[0].task.as_ref().unwrap().id;
        fixture.service().mark_prepared(task_id).unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute("DROP TABLE alert_settings", [])
            .unwrap();
        assert!(matches!(
            fixture.service().verify(task_id),
            Err(PreparationError::Safety(_))
        ));
        let task = repository::load_task(&fixture.database.open().unwrap(), task_id)
            .unwrap()
            .unwrap();
        assert_eq!(task.state, PreparationState::Prepared);
        assert!(task.verified_at.is_none());
    }

    #[test]
    fn preparation_lifecycle_does_not_mutate_orders_regimens_patients_or_inventory() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        let before: String = connection
            .query_row(
                "SELECT quote((SELECT dose FROM order_items WHERE id=13))||'|'||
                    quote((SELECT regimen_id FROM orders WHERE id=10))||'|'||
                    quote((SELECT legacy_hn FROM patients WHERE id=1))||'|'||
                    (SELECT COUNT(*) FROM inventory_events)||'|'||
                    (SELECT COUNT(*) FROM inventory_movements)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);
        let workspace = fixture.initialize();
        let task_id = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 13)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;
        fixture.service().mark_prepared(task_id).unwrap();
        fixture.service().verify(task_id).unwrap();
        let after: String = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT quote((SELECT dose FROM order_items WHERE id=13))||'|'||
                    quote((SELECT regimen_id FROM orders WHERE id=10))||'|'||
                    quote((SELECT legacy_hn FROM patients WHERE id=1))||'|'||
                    (SELECT COUNT(*) FROM inventory_events)||'|'||
                    (SELECT COUNT(*) FROM inventory_movements)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn quantity_and_shortage_preview_are_read_only_and_non_blocking() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        connection
            .execute(
                "UPDATE drugs SET inventory_enabled=1,inventory_min=1,package='Amp.' WHERE id=1",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO inventory_movements(
                    drug_id,movement_type,quantity_delta,reference_type,reference_id,note
                 ) VALUES(1,'opening_balance',1,'synthetic_fixture','PREP','Synthetic opening only')",
                [],
            )
            .unwrap();
        let before: (i64, f64, String) = connection
            .query_row(
                "SELECT COUNT(*),SUM(quantity_delta),
                        quote((SELECT dose FROM order_items WHERE id=11))
                 FROM inventory_movements",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        drop(connection);

        let workspace = fixture.service().get_workspace(10).unwrap();
        let item = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap();
        assert_eq!(item.calculation.containers_required.as_deref(), Some("2"));
        assert_eq!(
            item.calculation
                .inventory_projection
                .projected_stock
                .as_deref(),
            Some("-1")
        );
        assert_eq!(
            item.calculation.inventory_projection.state,
            crate::preparation_calc::InventoryProjectionState::Shortage
        );

        let after: (i64, f64, String) = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT COUNT(*),SUM(quantity_delta),
                        quote((SELECT dose FROM order_items WHERE id=11))
                 FROM inventory_movements",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(before, after);
        assert!(workspace.editable);
    }

    #[test]
    fn supported_verification_posts_exact_authenticated_issue_and_audit_once() {
        let fixture = Fixture::new();
        let task_id = fixture.prepare_tracked_adjunct(5.0, 50.0);
        let accepted_containers = fixture
            .service()
            .get_workspace(10)
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.order_item_id == 13)
            .unwrap()
            .calculation
            .containers_required;
        let clinical_before: String = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT quote((SELECT dose FROM order_items WHERE id=13))||'|'||
                        quote((SELECT number_of_drug FROM order_items WHERE id=13))||'|'||
                        quote((SELECT regimen_id FROM orders WHERE id=10))||'|'||
                        quote((SELECT legacy_hn FROM patients WHERE id=1))",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let verified = fixture.service().verify(task_id).unwrap();
        let posting = verified.inventory_posting.as_ref().unwrap();
        assert_eq!(posting.status, PreparationInventoryPostingStatus::Posted);
        assert_eq!(posting.containers_required, accepted_containers);
        assert_eq!(posting.balance_before.as_deref(), Some("5.0"));
        assert_eq!(posting.balance_after.as_deref(), Some("4.0"));
        assert_eq!(
            posting.resulting_stock_state,
            Some(PreparationIssueStockState::Normal)
        );
        assert_eq!(posting.actor, verified.verified_by.clone().unwrap());
        assert_eq!(
            posting.calculation_ruleset_version,
            "legacy-cytotoxic-v8+withdrawal-1dp-v1"
        );
        assert_eq!(
            posting.calculation_rule_id,
            "legacy-cytotoxic-v8:preparation-container-use-withdrawal-1dp"
        );
        assert_eq!(
            posting.workflow_rule_id,
            PREPARATION_INVENTORY_WORKFLOW_RULE
        );

        let connection = fixture.database.open().unwrap();
        let movement: (i64, f64, i64, i64) = connection
            .query_row(
                "SELECT m.id,m.quantity_delta,m.actor_user_id,m.preparation_task_id
                 FROM inventory_movements m
                 WHERE m.movement_type='preparation_issue'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(movement.0, posting.inventory_movement_id.unwrap());
        assert_eq!(movement.1, -1.0);
        assert_eq!(movement.2, posting.actor.id);
        assert_eq!(movement.3, task_id);
        let issue_audit: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM audit_events
                 WHERE event_type='preparation_inventory_issued'
                   AND entity_id=?1",
                [movement.0.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(issue_audit, 1);
        let clinical_after: String = connection
            .query_row(
                "SELECT quote((SELECT dose FROM order_items WHERE id=13))||'|'||
                        quote((SELECT number_of_drug FROM order_items WHERE id=13))||'|'||
                        quote((SELECT regimen_id FROM orders WHERE id=10))||'|'||
                        quote((SELECT legacy_hn FROM patients WHERE id=1))",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(clinical_after, clinical_before);
        drop(connection);

        let retry = fixture.service().verify(task_id).unwrap();
        assert_eq!(retry.inventory_posting, verified.inventory_posting);
        let reloaded = fixture.service().get_workspace(10).unwrap();
        let reloaded_posting = reloaded
            .items
            .iter()
            .find(|item| item.order_item_id == 13)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .inventory_posting
            .as_ref()
            .unwrap();
        assert_eq!(reloaded_posting.id, posting.id);

        let restarted_session = AuthSession::default();
        AuthService::new(&fixture.database, &restarted_session)
            .login(LoginInput {
                username: "synthetic.pharmacist".into(),
                password: "synthetic preparation password 42!".into(),
            })
            .unwrap();
        let after_restart = PreparationService::new(&fixture.database, &restarted_session)
            .verify(task_id)
            .unwrap();
        assert_eq!(
            after_restart
                .inventory_posting
                .as_ref()
                .map(|value| value.id),
            Some(posting.id)
        );
        let issue_count: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM inventory_movements
                 WHERE movement_type='preparation_issue' AND preparation_task_id=?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(issue_count, 1);
    }

    #[test]
    fn zero_and_already_negative_stock_never_block_verification() {
        for (opening, dose, expected_after, expected_containers) in
            [(0.0, 50.0, -1.0, "1"), (-2.0, 250.0, -5.0, "3")]
        {
            let fixture = Fixture::new();
            let task_id = fixture.prepare_tracked_adjunct(opening, dose);
            let verified = fixture.service().verify(task_id).unwrap();
            let posting = verified.inventory_posting.unwrap();
            assert_eq!(posting.status, PreparationInventoryPostingStatus::Posted);
            assert_eq!(
                posting.containers_required.as_deref(),
                Some(expected_containers)
            );
            assert_eq!(
                posting
                    .balance_after
                    .as_deref()
                    .and_then(|value| value.parse::<f64>().ok()),
                Some(expected_after)
            );
            assert_eq!(
                posting.resulting_stock_state,
                Some(PreparationIssueStockState::Shortage)
            );
            assert_eq!(verified.state, PreparationState::Verified);
        }
    }

    #[test]
    fn zero_requirement_disabled_tracking_and_missing_balance_record_explicit_outcomes() {
        let zero_fixture = Fixture::new();
        let zero_task = zero_fixture.prepare_tracked_adjunct(5.0, 0.0);
        let zero = zero_fixture.service().verify(zero_task).unwrap();
        let zero_posting = zero.inventory_posting.unwrap();
        assert_eq!(
            zero_posting.status,
            PreparationInventoryPostingStatus::NotRequired
        );
        assert_eq!(zero_posting.containers_required.as_deref(), Some("0"));
        assert!(zero_posting.inventory_movement_id.is_none());

        let disabled_fixture = Fixture::new();
        let disabled_workspace = disabled_fixture.initialize();
        let disabled_task = disabled_workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 13)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;
        disabled_fixture
            .service()
            .mark_prepared(disabled_task)
            .unwrap();
        let disabled = disabled_fixture.service().verify(disabled_task).unwrap();
        assert_eq!(
            disabled.inventory_posting.unwrap().status,
            PreparationInventoryPostingStatus::TrackingDisabled
        );

        let missing_fixture = Fixture::new();
        missing_fixture
            .database
            .open()
            .unwrap()
            .execute("UPDATE drugs SET inventory_enabled=1 WHERE id=3", [])
            .unwrap();
        let missing_workspace = missing_fixture.initialize();
        let missing_task = missing_workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 13)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;
        missing_fixture
            .service()
            .mark_prepared(missing_task)
            .unwrap();
        let missing = missing_fixture.service().verify(missing_task).unwrap();
        let missing_posting = missing.inventory_posting.unwrap();
        assert_eq!(
            missing_posting.status,
            PreparationInventoryPostingStatus::ManualReconciliationRequired
        );
        assert_eq!(missing_posting.reason_code, "inventory_balance_unavailable");
        assert!(missing_posting.inventory_movement_id.is_none());
    }

    #[test]
    fn unsupported_units_verify_with_manual_reconciliation_and_ignore_noofdrug() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        connection
            .execute(
                "UPDATE drugs SET inventory_enabled=1,inventory_min=1 WHERE id=3",
                [],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE order_items
                 SET regimen_unit_text='mcg',number_of_drug=777
                 WHERE id=13",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO inventory_movements(
                    drug_id,movement_type,quantity_delta,reference_type,reference_id,note
                 ) VALUES(3,'opening_balance',10,'synthetic_fixture','ADJUNCT',
                          'Synthetic opening only')",
                [],
            )
            .unwrap();
        drop(connection);
        let workspace = fixture.initialize();
        let item = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 13)
            .unwrap();
        assert_eq!(
            item.calculation.status,
            crate::preparation_calc::PreparationCalculationStatus::Unsupported
        );
        let task_id = item.task.as_ref().unwrap().id;
        fixture.service().mark_prepared(task_id).unwrap();

        let verified = fixture.service().verify(task_id).unwrap();
        assert_eq!(verified.state, PreparationState::Verified);
        let posting = verified.inventory_posting.unwrap();
        assert_eq!(
            posting.status,
            PreparationInventoryPostingStatus::ManualReconciliationRequired
        );
        assert!(posting.inventory_movement_id.is_none());
        assert!(posting.containers_required.is_none());
        assert_eq!(posting.reason_code, "calculation_not_fully_supported");
        let issue_count: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM inventory_movements
                 WHERE movement_type='preparation_issue'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(issue_count, 0);
    }

    #[test]
    fn movement_failure_rolls_back_verification_and_posting() {
        let fixture = Fixture::new();
        let task_id = fixture.prepare_tracked_adjunct(5.0, 50.0);
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_issue_failure
                 BEFORE INSERT ON inventory_movements
                 WHEN NEW.movement_type='preparation_issue'
                 BEGIN SELECT RAISE(ABORT,'synthetic issue failure'); END;",
            )
            .unwrap();

        assert!(matches!(
            fixture.service().verify(task_id),
            Err(PreparationError::Sqlite(_))
        ));
        let connection = fixture.database.open().unwrap();
        let state: String = connection
            .query_row(
                "SELECT state FROM preparation_tasks WHERE id=?1",
                [task_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, "prepared");
        let counts: (i64, i64) = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM inventory_movements WHERE movement_type='preparation_issue'),
                    (SELECT COUNT(*) FROM preparation_inventory_postings)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(counts, (0, 0));
    }

    #[test]
    fn final_audit_failure_rolls_back_issued_event_movement_posting_and_verification() {
        let fixture = Fixture::new();
        let task_id = fixture.prepare_tracked_adjunct(5.0, 50.0);
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_final_audit_failure
                 BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='preparation_verified'
                 BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;",
            )
            .unwrap();

        assert!(matches!(
            fixture.service().verify(task_id),
            Err(PreparationError::Sqlite(_))
        ));
        let connection = fixture.database.open().unwrap();
        let row: (String, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT t.state,
                        (SELECT COUNT(*) FROM inventory_movements WHERE movement_type='preparation_issue'),
                        (SELECT COUNT(*) FROM preparation_inventory_postings),
                        (SELECT COUNT(*) FROM audit_events WHERE event_type='preparation_inventory_issued'),
                        (SELECT COUNT(*) FROM audit_events WHERE event_type='preparation_verified')
                 FROM preparation_tasks t WHERE t.id=?1",
                [task_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(row, ("prepared".into(), 0, 0, 0, 0));
    }

    #[test]
    fn database_uniqueness_and_append_only_history_protect_automatic_issue() {
        let fixture = Fixture::new();
        let task_id = fixture.prepare_tracked_adjunct(5.0, 50.0);
        let verified = fixture.service().verify(task_id).unwrap();
        let posting = verified.inventory_posting.unwrap();
        let connection = fixture.database.open().unwrap();
        assert!(connection
            .execute(
                "INSERT INTO inventory_movements(
                    drug_id,movement_type,quantity_delta,occurred_at,actor_user_id,
                    reference_type,reference_id,preparation_task_id
                 ) VALUES(3,'preparation_issue',-1,CURRENT_TIMESTAMP,1,
                          'preparation_task','duplicate',?1)",
                [task_id],
            )
            .is_err());
        assert!(connection
            .execute(
                "UPDATE inventory_movements SET quantity_delta=-9 WHERE id=?1",
                [posting.inventory_movement_id.unwrap()],
            )
            .is_err());
        assert!(connection
            .execute(
                "DELETE FROM preparation_inventory_postings WHERE id=?1",
                [posting.id],
            )
            .is_err());
    }

    #[test]
    fn pre_integration_verified_task_is_returned_without_backfill() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let task_id = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 13)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;
        fixture.service().mark_prepared(task_id).unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute(
                "UPDATE preparation_tasks
                 SET state='verified',verified_at=CURRENT_TIMESTAMP,verified_by_user_id=1
                 WHERE id=?1",
                [task_id],
            )
            .unwrap();

        let returned = fixture.service().verify(task_id).unwrap();
        assert_eq!(returned.state, PreparationState::Verified);
        assert!(returned.inventory_posting.is_none());
        let counts: (i64, i64) = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM preparation_inventory_postings),
                    (SELECT COUNT(*) FROM inventory_movements WHERE movement_type='preparation_issue')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(counts, (0, 0));
    }

    #[test]
    fn validates_date_range_and_preparation_volume() {
        let fixture = Fixture::new();
        assert!(matches!(
            fixture.service().list_queue(PreparationQueueRequest {
                date_from: Some("2026-02-30".into()),
                ..PreparationQueueRequest::default()
            }),
            Err(PreparationError::Validation {
                field: "dateFrom",
                ..
            })
        ));
        assert!(matches!(
            fixture.service().list_queue(PreparationQueueRequest {
                preparation_date: Some("2026-02-30".into()),
                ..PreparationQueueRequest::default()
            }),
            Err(PreparationError::Validation {
                field: "preparationDate",
                ..
            })
        ));
        let task_id = fixture.initialize().items[0].task.as_ref().unwrap().id;
        assert!(matches!(
            fixture.service().update_task(
                task_id,
                PreparationTaskInput {
                    preparation_volume_ml: Some(-1.0),
                    preparation_notes: None,
                    ..PreparationTaskInput::default()
                }
            ),
            Err(PreparationError::Validation {
                field: "preparationVolumeMl",
                ..
            })
        ));
    }

    #[test]
    fn anonymous_preparation_actions_are_rejected_by_the_rust_session() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let task_id = workspace.items[0].task.as_ref().unwrap().id;
        AuthService::new(&fixture.database, &fixture.session)
            .logout()
            .unwrap();
        assert!(matches!(
            fixture.service().mark_prepared(task_id),
            Err(PreparationError::Auth(AuthError::AuthenticationRequired))
        ));
        let task = repository::load_task(&fixture.database.open().unwrap(), task_id)
            .unwrap()
            .unwrap();
        assert_eq!(task.state, PreparationState::Pending);
        assert!(task.prepared_by.is_none());
    }

    #[test]
    fn prior_schema_four_task_remains_explicitly_unattributed() {
        let fixture = Fixture::new();
        fixture.database.open().unwrap().execute(
            "INSERT INTO preparation_tasks(
                source_order_id,source_order_item_id,preparation_date,drug_id,state,
                snapshot_ordered_dose_text,snapshot_dose_unit_text,
                snapshot_diluent_id,snapshot_diluent_name,snapshot_diluent_volume_ml,
                snapshot_route_id,snapshot_route_name,snapshot_rate_text,
                snapshot_treatment_day,snapshot_sequence_no,snapshot_regimen_details,
                snapshot_drug_detail,snapshot_drug_storage,prepared_at
             ) VALUES(10,11,'2026-08-23',1,'prepared','100','mg',1,'สารละลายทดสอบ',1,1,'IV','60 min','1',1,'คำแนะนำสังเคราะห์','เตรียมแบบทดสอบ','เก็บแบบทดสอบ',CURRENT_TIMESTAMP)",
            [],
        ).unwrap();
        let workspace = fixture.service().get_workspace(10).unwrap();
        let task = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap()
            .task
            .as_ref()
            .unwrap();
        assert_eq!(task.state, PreparationState::Prepared);
        assert!(task.prepared_by.is_none());
        assert!(task.verified_by.is_none());
    }

    #[test]
    fn prepare_transition_and_audit_event_are_atomic() {
        let fixture = Fixture::new();
        let task_id = fixture.initialize().items[0].task.as_ref().unwrap().id;
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_prepare_audit_failure BEFORE INSERT ON audit_events
             WHEN NEW.event_type='preparation_marked_prepared'
             BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().mark_prepared(task_id),
            Err(PreparationError::Sqlite(_))
        ));
        let task = repository::load_task(&fixture.database.open().unwrap(), task_id)
            .unwrap()
            .unwrap();
        assert_eq!(task.state, PreparationState::Pending);
        assert!(task.prepared_by.is_none());
        let events: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM audit_events WHERE event_type='preparation_marked_prepared'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(events, 0);
    }

    #[test]
    fn verify_transition_and_audit_event_are_atomic() {
        let fixture = Fixture::new();
        let task_id = fixture
            .initialize()
            .items
            .iter()
            .find(|item| item.order_item_id == 13)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;
        fixture.service().mark_prepared(task_id).unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_verify_audit_failure BEFORE INSERT ON audit_events
             WHEN NEW.event_type='preparation_verified'
             BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().verify(task_id),
            Err(PreparationError::Sqlite(_))
        ));
        let task = repository::load_task(&fixture.database.open().unwrap(), task_id)
            .unwrap()
            .unwrap();
        assert_eq!(task.state, PreparationState::Prepared);
        assert!(task.prepared_by.is_some());
        assert!(task.verified_by.is_none());
        assert!(task.verified_at.is_none());
    }

    #[test]
    fn acknowledgement_persists_with_actor_rule_version_and_invalidates_on_change() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let warning = workspace
            .safety
            .findings
            .iter()
            .find(|finding| finding.order_item_id == Some(11) && finding.acknowledgement_required)
            .unwrap()
            .clone();
        let acknowledged = fixture
            .service()
            .acknowledge_safety_finding(10, warning.id.clone())
            .unwrap();
        let record = acknowledged
            .safety_acknowledgements
            .iter()
            .find(|value| value.finding_id == warning.id)
            .unwrap();
        assert_eq!(record.finding_fingerprint, warning.fingerprint);
        assert_eq!(record.ruleset_version, warning.ruleset_version);
        assert_eq!(record.rule_id, warning.rule_id);
        assert_eq!(record.user.display_name, "เภสัชกรสังเคราะห์");
        assert!(!record.source_snapshot_stale);
        assert_eq!(
            fixture
                .service()
                .get_workspace(10)
                .unwrap()
                .safety_acknowledgements
                .len(),
            1
        );

        fixture
            .database
            .open()
            .unwrap()
            .execute("UPDATE drugs SET max_dilution_hard=2 WHERE id=1", [])
            .unwrap();
        let changed = fixture.service().get_workspace(10).unwrap();
        let changed_warning = changed
            .safety
            .findings
            .iter()
            .find(|finding| finding.id == warning.id)
            .unwrap();
        assert_ne!(changed_warning.fingerprint, warning.fingerprint);
        assert!(changed.safety_acknowledgements.is_empty());
        let task_id = changed
            .items
            .iter()
            .find(|item| item.order_item_id == 11)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;
        fixture.service().mark_prepared(task_id).unwrap();
        assert!(matches!(
            fixture.service().verify(task_id),
            Err(PreparationError::SafetyReviewRequired { .. })
        ));
    }

    #[test]
    fn acknowledgement_and_audit_event_are_atomic() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let finding_id = workspace
            .safety
            .findings
            .iter()
            .find(|finding| finding.acknowledgement_required)
            .unwrap()
            .id
            .clone();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_ack_audit_failure BEFORE INSERT ON audit_events
             WHEN NEW.event_type='safety_finding_acknowledged'
             BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().acknowledge_safety_finding(10, finding_id),
            Err(PreparationError::Sqlite(_))
        ));
        let connection = fixture.database.open().unwrap();
        let counts: (i64, i64) = connection.query_row(
            "SELECT (SELECT COUNT(*) FROM safety_acknowledgements),
                    (SELECT COUNT(*) FROM audit_events WHERE event_type='safety_finding_acknowledged')",
            [], |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
        assert_eq!(counts, (0, 0));
    }

    #[test]
    fn preparation_audit_metadata_contains_only_minimal_non_sensitive_context() {
        let fixture = Fixture::new();
        let workspace = fixture.initialize();
        let task_id = workspace
            .items
            .iter()
            .find(|item| item.order_item_id == 13)
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .id;
        fixture.service().mark_prepared(task_id).unwrap();
        fixture.service().verify(task_id).unwrap();
        let metadata = fixture.database.open().unwrap().prepare(
            "SELECT metadata_json FROM audit_events WHERE event_type LIKE 'preparation_%' ORDER BY id",
        ).unwrap().query_map([], |row| row.get::<_, String>(0)).unwrap()
            .collect::<Result<Vec<_>, _>>().unwrap().join("|");
        for forbidden in ["ผู้ป่วย", "ทดสอบ", "Synthetic preparation agent", "คำแนะนำ"]
        {
            assert!(!metadata.contains(forbidden));
        }
        assert!(metadata.contains("source_stale"));
    }
}
