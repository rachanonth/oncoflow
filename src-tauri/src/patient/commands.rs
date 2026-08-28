use serde::Serialize;
use tauri::State;

use crate::db::Database;

use super::{
    PatientDetail, PatientError, PatientFormOptions, PatientInput, PatientListRequest,
    PatientListResponse, PatientService,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<PatientError> for CommandError {
    fn from(error: PatientError) -> Self {
        match error {
            PatientError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            PatientError::DuplicateHn => Self {
                code: "duplicate_hn",
                message: "A patient with this HN already exists.".to_owned(),
                field: Some("hn"),
            },
            PatientError::NotFound => Self {
                code: "not_found",
                message: "Patient record was not found.".to_owned(),
                field: None,
            },
            PatientError::Database(_) | PatientError::Sqlite(_) => Self {
                code: "database_error",
                message: "The local patient database operation failed.".to_owned(),
                field: None,
            },
        }
    }
}

#[tauri::command]
pub(crate) fn list_patients(
    database: State<'_, Database>,
    request: PatientListRequest,
) -> Result<PatientListResponse, CommandError> {
    PatientService::new(&database)
        .list(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_patient(
    database: State<'_, Database>,
    patient_id: i64,
) -> Result<PatientDetail, CommandError> {
    PatientService::new(&database)
        .get(patient_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_patient(
    database: State<'_, Database>,
    input: PatientInput,
) -> Result<PatientDetail, CommandError> {
    PatientService::new(&database)
        .create(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_patient(
    database: State<'_, Database>,
    patient_id: i64,
    input: PatientInput,
) -> Result<PatientDetail, CommandError> {
    PatientService::new(&database)
        .update(patient_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn patient_form_options(
    database: State<'_, Database>,
) -> Result<PatientFormOptions, CommandError> {
    PatientService::new(&database)
        .form_options()
        .map_err(Into::into)
}
