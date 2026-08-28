use serde::Serialize;
use tauri::State;

use crate::{auth::AuthSession, db::Database};

use super::{GuidanceError, GuidanceService, PageGuidanceRecord, UpdatePageGuidanceInput};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<GuidanceError> for CommandError {
    fn from(error: GuidanceError) -> Self {
        match error {
            GuidanceError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            GuidanceError::Auth(crate::auth::AuthError::AdminRequired) => Self::plain(
                "admin_required",
                "Local administrator access is required to manage Guidance.",
            ),
            GuidanceError::Auth(crate::auth::AuthError::AuthenticationRequired) => Self::plain(
                "authentication_required",
                "Sign in with a local OncoFlow account to continue.",
            ),
            GuidanceError::Auth(_) | GuidanceError::Database(_) | GuidanceError::Sqlite(_) => {
                Self::plain(
                    "guidance_error",
                    "The local Guidance operation could not be completed.",
                )
            }
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
pub(crate) fn list_page_guidance(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
) -> Result<Vec<PageGuidanceRecord>, CommandError> {
    GuidanceService::new(&database, &session)
        .list()
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_page_guidance(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: UpdatePageGuidanceInput,
) -> Result<PageGuidanceRecord, CommandError> {
    GuidanceService::new(&database, &session)
        .update(input)
        .map_err(Into::into)
}
