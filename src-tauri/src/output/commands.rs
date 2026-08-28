use serde::Serialize;
use tauri::State;

use crate::{
    auth::{AuthError, AuthSession},
    db::Database,
};

use super::{OutputError, OutputService, PreparationOutput};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: &'static str,
}

impl From<OutputError> for CommandError {
    fn from(error: OutputError) -> Self {
        match error {
            OutputError::TaskNotFound => Self::new("not_found", "Preparation task was not found."),
            OutputError::VerificationRequired => Self::new(
                "preparation_check_required",
                "Only a checked preparation can produce a final label.",
            ),
            OutputError::IncompleteProvenance => Self::new(
                "incomplete_provenance",
                "The checked preparation does not contain enough provenance for final output.",
            ),
            OutputError::InvalidSelection => Self::new(
                "invalid_selection",
                "Select preparation items that belong to the current order.",
            ),
            OutputError::Auth(AuthError::AuthenticationRequired) => Self::new(
                "authentication_required",
                "Sign in with a local OncoFlow account to preview or print a final label.",
            ),
            OutputError::Auth(_) => Self::new(
                "authentication_error",
                "The authenticated local session could not be confirmed.",
            ),
            OutputError::Database(_) | OutputError::Sqlite(_) => Self::new(
                "database_error",
                "The local preparation output operation failed.",
            ),
        }
    }
}

impl CommandError {
    const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

#[tauri::command]
pub(crate) fn get_preparation_output(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    preparation_id: i64,
) -> Result<PreparationOutput, CommandError> {
    OutputService::new(&database, &session)
        .get_preparation_output(preparation_id)
        .map_err(Into::into)
}
