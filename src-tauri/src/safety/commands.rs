use serde::Serialize;
use tauri::State;

use crate::db::Database;

use super::{SafetyError, SafetyEvaluation, SafetyService};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: &'static str,
}

impl From<SafetyError> for CommandError {
    fn from(error: SafetyError) -> Self {
        match error {
            SafetyError::OrderNotFound => Self {
                code: "not_found",
                message: "Order record was not found.",
            },
            SafetyError::Database(_) | SafetyError::Sqlite(_) => Self {
                code: "database_error",
                message: "The local safety evaluation could not be completed.",
            },
        }
    }
}

#[tauri::command]
pub(crate) fn evaluate_order_safety(
    database: State<'_, Database>,
    order_id: i64,
) -> Result<SafetyEvaluation, CommandError> {
    SafetyService::new(&database)
        .evaluate_order(order_id)
        .map_err(Into::into)
}
