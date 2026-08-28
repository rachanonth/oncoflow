use serde::Serialize;
use tauri::State;

use crate::{
    auth::{AuthError, AuthSession},
    db::Database,
};

use super::{
    InventoryAdjustmentInput, InventoryDetail, InventoryError, InventoryListRequest,
    InventoryListResponse, InventoryManualIssueInput, InventoryMovementListRequest,
    InventoryMovementListResponse, InventoryMovementResult, InventoryReceiptInput,
    InventoryService,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<InventoryError> for CommandError {
    fn from(error: InventoryError) -> Self {
        match error {
            InventoryError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            InventoryError::NotFound => Self {
                code: "not_found",
                message: "Inventory drug was not found.".into(),
                field: None,
            },
            InventoryError::Auth(AuthError::AuthenticationRequired) => Self {
                code: "authentication_required",
                message: "Sign in to use inventory.".into(),
                field: None,
            },
            InventoryError::Auth(_) => Self {
                code: "authentication_error",
                message: "The local authenticated session is unavailable.".into(),
                field: None,
            },
            InventoryError::Database(_) | InventoryError::Sqlite(_) => Self {
                code: "database_error",
                message: "The local inventory database operation failed.".into(),
                field: None,
            },
        }
    }
}

#[tauri::command]
pub(crate) fn list_inventory(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: InventoryListRequest,
) -> Result<InventoryListResponse, CommandError> {
    InventoryService::new(&database, &session)
        .list(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_low_stock_items(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: InventoryListRequest,
) -> Result<InventoryListResponse, CommandError> {
    InventoryService::new(&database, &session)
        .low_stock(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_inventory_item(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    drug_id: i64,
) -> Result<InventoryDetail, CommandError> {
    InventoryService::new(&database, &session)
        .get(drug_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_inventory_movements(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: InventoryMovementListRequest,
) -> Result<InventoryMovementListResponse, CommandError> {
    InventoryService::new(&database, &session)
        .movements(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn record_inventory_receipt(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: InventoryReceiptInput,
) -> Result<InventoryMovementResult, CommandError> {
    InventoryService::new(&database, &session)
        .record_receipt(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn record_inventory_adjustment(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: InventoryAdjustmentInput,
) -> Result<InventoryMovementResult, CommandError> {
    InventoryService::new(&database, &session)
        .record_adjustment(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn record_inventory_manual_issue(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: InventoryManualIssueInput,
) -> Result<InventoryMovementResult, CommandError> {
    InventoryService::new(&database, &session)
        .record_manual_issue(input)
        .map_err(Into::into)
}
