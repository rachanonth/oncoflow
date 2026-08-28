use serde::Serialize;
use tauri::State;

use crate::db::Database;

use super::{
    DrugDetail, DrugError, DrugFormOptions, DrugInput, DrugListRequest, DrugListResponse,
    DrugService,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<DrugError> for CommandError {
    fn from(error: DrugError) -> Self {
        match error {
            DrugError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            DrugError::DuplicateCode => Self {
                code: "duplicate_code",
                message: "A drug with this code already exists.".to_owned(),
                field: Some("code"),
            },
            DrugError::NotFound => Self {
                code: "not_found",
                message: "Drug record was not found.".to_owned(),
                field: None,
            },
            DrugError::Database(_) | DrugError::Sqlite(_) => Self {
                code: "database_error",
                message: "The local drug database operation failed.".to_owned(),
                field: None,
            },
        }
    }
}

#[tauri::command]
pub(crate) fn list_drugs(
    database: State<'_, Database>,
    request: DrugListRequest,
) -> Result<DrugListResponse, CommandError> {
    DrugService::new(&database)
        .list(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_drug(
    database: State<'_, Database>,
    drug_id: i64,
) -> Result<DrugDetail, CommandError> {
    DrugService::new(&database).get(drug_id).map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_drug(
    database: State<'_, Database>,
    input: DrugInput,
) -> Result<DrugDetail, CommandError> {
    DrugService::new(&database)
        .create(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_drug(
    database: State<'_, Database>,
    drug_id: i64,
    input: DrugInput,
) -> Result<DrugDetail, CommandError> {
    DrugService::new(&database)
        .update(drug_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn drug_form_options(
    database: State<'_, Database>,
) -> Result<DrugFormOptions, CommandError> {
    DrugService::new(&database)
        .form_options()
        .map_err(Into::into)
}
