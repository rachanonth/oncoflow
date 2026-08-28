use serde::Serialize;
use tauri::State;

use crate::{auth::AuthSession, db::Database};

use super::{
    DiagnosisInput, DiagnosisRecord, DiluentInput, DiluentRecord, DoctorInput, DoctorRecord,
    MasterDataError, MasterDataListRequest, MasterDataService, RouteInput, RouteRecord, WardInput,
    WardRecord,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<MasterDataError> for CommandError {
    fn from(error: MasterDataError) -> Self {
        match error {
            MasterDataError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            MasterDataError::DoctorNotFound => {
                Self::plain("doctor_not_found", "The doctor could not be found.")
            }
            MasterDataError::WardNotFound => {
                Self::plain("ward_not_found", "The ward could not be found.")
            }
            MasterDataError::RouteNotFound => {
                Self::plain("route_not_found", "The route could not be found.")
            }
            MasterDataError::DiluentNotFound => {
                Self::plain("diluent_not_found", "The diluent could not be found.")
            }
            MasterDataError::DiagnosisNotFound => {
                Self::plain("diagnosis_not_found", "The diagnosis could not be found.")
            }
            MasterDataError::Auth(crate::auth::AuthError::AdminRequired) => Self::plain(
                "admin_required",
                "Local administrator access is required to manage master data.",
            ),
            MasterDataError::Auth(crate::auth::AuthError::AuthenticationRequired) => Self::plain(
                "authentication_required",
                "Sign in with a local OncoFlow account to continue.",
            ),
            MasterDataError::Auth(_)
            | MasterDataError::Database(_)
            | MasterDataError::Sqlite(_) => Self::plain(
                "master_data_error",
                "The local master-data operation could not be completed.",
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
pub(crate) fn list_doctors(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: MasterDataListRequest,
) -> Result<Vec<DoctorRecord>, CommandError> {
    MasterDataService::new(&database, &session)
        .list_doctors(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_doctor(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: DoctorInput,
) -> Result<DoctorRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .create_doctor(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_doctor(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    doctor_id: i64,
    input: DoctorInput,
) -> Result<DoctorRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .update_doctor(doctor_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_wards(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: MasterDataListRequest,
) -> Result<Vec<WardRecord>, CommandError> {
    MasterDataService::new(&database, &session)
        .list_wards(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_ward(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: WardInput,
) -> Result<WardRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .create_ward(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_ward(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    ward_id: i64,
    input: WardInput,
) -> Result<WardRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .update_ward(ward_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_routes(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: MasterDataListRequest,
) -> Result<Vec<RouteRecord>, CommandError> {
    MasterDataService::new(&database, &session)
        .list_routes(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_route(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: RouteInput,
) -> Result<RouteRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .create_route(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_route(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    route_id: i64,
    input: RouteInput,
) -> Result<RouteRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .update_route(route_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_diluents(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: MasterDataListRequest,
) -> Result<Vec<DiluentRecord>, CommandError> {
    MasterDataService::new(&database, &session)
        .list_diluents(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_diluent(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: DiluentInput,
) -> Result<DiluentRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .create_diluent(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_diluent(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    diluent_id: i64,
    input: DiluentInput,
) -> Result<DiluentRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .update_diluent(diluent_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_diagnoses(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: MasterDataListRequest,
) -> Result<Vec<DiagnosisRecord>, CommandError> {
    MasterDataService::new(&database, &session)
        .list_diagnoses(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_diagnosis(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: DiagnosisInput,
) -> Result<DiagnosisRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .create_diagnosis(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_diagnosis(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    diagnosis_id: i64,
    input: DiagnosisInput,
) -> Result<DiagnosisRecord, CommandError> {
    MasterDataService::new(&database, &session)
        .update_diagnosis(diagnosis_id, input)
        .map_err(Into::into)
}
