use serde::Serialize;
use tauri::State;

use crate::{
    auth::{AuthError, AuthSession},
    db::Database,
};

use super::{
    InventoryUsageReport, InventoryUsageReportRequest, PreparationCountReport,
    PreparationCountReportRequest, ReportError, ReportService,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<ReportError> for CommandError {
    fn from(error: ReportError) -> Self {
        match error {
            ReportError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            ReportError::Auth(AuthError::AuthenticationRequired) => Self {
                code: "authentication_required",
                message: "Sign in to view reports.".into(),
                field: None,
            },
            ReportError::Auth(_) => Self {
                code: "authentication_error",
                message: "The local authenticated session is unavailable.".into(),
                field: None,
            },
            ReportError::Database(_) | ReportError::Sqlite(_) => Self {
                code: "database_error",
                message: "The local report database query failed.".into(),
                field: None,
            },
        }
    }
}

#[tauri::command]
pub(crate) fn get_preparation_count_report(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: PreparationCountReportRequest,
) -> Result<PreparationCountReport, CommandError> {
    ReportService::new(&database, &session)
        .preparation_counts(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_inventory_usage_report(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    request: InventoryUsageReportRequest,
) -> Result<InventoryUsageReport, CommandError> {
    ReportService::new(&database, &session)
        .inventory_usage(request)
        .map_err(Into::into)
}
