use serde::Serialize;
use tauri::State;

use crate::{
    auth::{AuthError, AuthSession},
    db::Database,
};

use super::{
    PreparationError, PreparationQueueRequest, PreparationQueueResponse, PreparationService,
    PreparationTask, PreparationTaskInput, PreparationWorkspace,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<PreparationError> for CommandError {
    fn from(error: PreparationError) -> Self {
        match error {
            PreparationError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            PreparationError::OrderNotFound => {
                Self::plain("not_found", "Order record was not found.")
            }
            PreparationError::TaskNotFound => {
                Self::plain("not_found", "Preparation task was not found.")
            }
            PreparationError::HistoricalReadOnly => Self::plain(
                "historical_read_only",
                "Historical migrated orders cannot create or change preparation tasks.",
            ),
            PreparationError::OrderOnHold => Self::plain(
                "order_on_hold",
                "This order is on hold and is not available in the preparation workflow.",
            ),
            PreparationError::DateUnavailable => Self::plain(
                "preparation_date_unavailable",
                "This order is not available for preparation on the selected date.",
            ),
            PreparationError::VerifiedReadOnly => Self::plain(
                "verified_read_only",
                "Verified preparation tasks are read-only.",
            ),
            PreparationError::NotPrepared => Self::plain(
                "not_prepared",
                "Mark the preparation ready before verification.",
            ),
            PreparationError::StaleSource => Self::plain(
                "stale_source",
                "The source order item changed. Review the order before continuing.",
            ),
            PreparationError::SafetyReviewRequired { count } => Self::plain(
                "safety_review_required",
                format!(
                    "Review and acknowledge {count} current safety finding(s) before verification."
                ),
            ),
            PreparationError::FindingNotFound => Self::plain(
                "finding_not_found",
                "The current safety finding was not found. Refresh and review again.",
            ),
            PreparationError::FindingNotAcknowledgable => Self::plain(
                "finding_not_acknowledgable",
                "This safety information does not require acknowledgement.",
            ),
            PreparationError::Auth(AuthError::AuthenticationRequired) => Self::plain(
                "authentication_required",
                "Sign in with a local OncoFlow account to continue.",
            ),
            PreparationError::Auth(_) => Self::plain(
                "authentication_error",
                "The authenticated local session could not be confirmed.",
            ),
            PreparationError::Safety(_)
            | PreparationError::Database(_)
            | PreparationError::Sqlite(_) => Self::plain(
                "database_error",
                "The local preparation database operation failed.",
            ),
        }
    }
}

impl CommandError {
    fn plain(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
        }
    }
}

#[tauri::command]
pub(crate) fn list_preparation_queue(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: PreparationQueueRequest,
) -> Result<PreparationQueueResponse, CommandError> {
    PreparationService::new(&database, &session)
        .list_queue(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_preparation_workspace(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    order_id: i64,
    preparation_date: String,
) -> Result<PreparationWorkspace, CommandError> {
    PreparationService::new(&database, &session)
        .get_workspace_for_date(order_id, preparation_date)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn initialize_preparation(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    order_id: i64,
    preparation_date: String,
) -> Result<PreparationWorkspace, CommandError> {
    PreparationService::new(&database, &session)
        .initialize_for_date(order_id, preparation_date)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_preparation_task(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    task_id: i64,
    input: PreparationTaskInput,
) -> Result<PreparationTask, CommandError> {
    PreparationService::new(&database, &session)
        .update_task(task_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn mark_preparation_prepared(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    task_id: i64,
    prepared_by_user_id: i64,
) -> Result<PreparationTask, CommandError> {
    PreparationService::new(&database, &session)
        .mark_prepared_for(task_id, prepared_by_user_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn verify_preparation_task(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    task_id: i64,
) -> Result<PreparationTask, CommandError> {
    PreparationService::new(&database, &session)
        .verify(task_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn check_preparation_task(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    task_id: i64,
) -> Result<PreparationTask, CommandError> {
    PreparationService::new(&database, &session)
        .check(task_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn check_preparation_tasks(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    task_ids: Vec<i64>,
) -> Result<Vec<PreparationTask>, CommandError> {
    PreparationService::new(&database, &session)
        .check_batch(task_ids)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn acknowledge_preparation_safety_finding(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    order_id: i64,
    preparation_date: String,
    finding_id: String,
) -> Result<PreparationWorkspace, CommandError> {
    PreparationService::new(&database, &session)
        .acknowledge_safety_finding_for_date(order_id, preparation_date, finding_id)
        .map_err(Into::into)
}
