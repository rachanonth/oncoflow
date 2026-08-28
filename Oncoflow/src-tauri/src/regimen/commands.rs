use serde::Serialize;
use tauri::State;

use crate::db::Database;

use super::{
    RegimenDetail, RegimenError, RegimenGroupInput, RegimenInput, RegimenItemInput,
    RegimenListRequest, RegimenListResponse, RegimenLookups, RegimenReorderInput, RegimenService,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<RegimenError> for CommandError {
    fn from(error: RegimenError) -> Self {
        match error {
            RegimenError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            RegimenError::DuplicateCode => Self {
                code: "duplicate_code",
                message: "A regimen with this code already exists.".into(),
                field: Some("code"),
            },
            RegimenError::RegimenNotFound => Self {
                code: "not_found",
                message: "Regimen record was not found.".into(),
                field: None,
            },
            RegimenError::GroupNotFound => Self {
                code: "not_found",
                message: "Regimen treatment group was not found.".into(),
                field: None,
            },
            RegimenError::ItemNotFound => Self {
                code: "not_found",
                message: "Regimen item was not found.".into(),
                field: None,
            },
            RegimenError::GroupNotEmpty => Self {
                code: "group_not_empty",
                message: "Remove the group's drug steps before deleting it.".into(),
                field: None,
            },
            RegimenError::Database(_) | RegimenError::Sqlite(_) => Self {
                code: "database_error",
                message: "The local regimen database operation failed.".into(),
                field: None,
            },
        }
    }
}

#[tauri::command]
pub(crate) fn list_regimens(
    database: State<'_, Database>,
    request: RegimenListRequest,
) -> Result<RegimenListResponse, CommandError> {
    RegimenService::new(&database)
        .list(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_regimen(
    database: State<'_, Database>,
    regimen_id: i64,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .get(regimen_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_regimen(
    database: State<'_, Database>,
    input: RegimenInput,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .create(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_regimen(
    database: State<'_, Database>,
    regimen_id: i64,
    input: RegimenInput,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .update(regimen_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn add_regimen_group(
    database: State<'_, Database>,
    regimen_id: i64,
    input: RegimenGroupInput,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .add_group(regimen_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_regimen_group(
    database: State<'_, Database>,
    regimen_id: i64,
    group_id: i64,
    input: RegimenGroupInput,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .update_group(regimen_id, group_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn delete_regimen_group(
    database: State<'_, Database>,
    regimen_id: i64,
    group_id: i64,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .delete_group(regimen_id, group_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn add_regimen_item(
    database: State<'_, Database>,
    regimen_id: i64,
    input: RegimenItemInput,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .add_item(regimen_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_regimen_item(
    database: State<'_, Database>,
    regimen_id: i64,
    item_id: i64,
    input: RegimenItemInput,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .update_item(regimen_id, item_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn delete_regimen_item(
    database: State<'_, Database>,
    regimen_id: i64,
    item_id: i64,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .delete_item(regimen_id, item_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn reorder_regimen_items(
    database: State<'_, Database>,
    regimen_id: i64,
    input: RegimenReorderInput,
) -> Result<RegimenDetail, CommandError> {
    RegimenService::new(&database)
        .reorder_items(regimen_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_regimen_lookups(
    database: State<'_, Database>,
) -> Result<RegimenLookups, CommandError> {
    RegimenService::new(&database).lookups().map_err(Into::into)
}
