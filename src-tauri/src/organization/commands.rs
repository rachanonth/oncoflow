use serde::Serialize;
use tauri::State;

use crate::{auth::AuthSession, db::Database};

use super::{
    ApplicationSettings, OrganizationError, OrganizationService, UpdateApplicationSettingsInput,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<OrganizationError> for CommandError {
    fn from(error: OrganizationError) -> Self {
        match error {
            OrganizationError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            OrganizationError::Auth(crate::auth::AuthError::AdminRequired) => Self::plain(
                "admin_required",
                "Local administrator access is required to manage application settings.",
            ),
            OrganizationError::Auth(crate::auth::AuthError::AuthenticationRequired) => Self::plain(
                "authentication_required",
                "Sign in with a local OncoFlow account to continue.",
            ),
            OrganizationError::Auth(_)
            | OrganizationError::Database(_)
            | OrganizationError::Sqlite(_) => Self::plain(
                "application_settings_error",
                "The local application settings operation could not be completed.",
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
pub(crate) fn get_application_settings(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
) -> Result<ApplicationSettings, CommandError> {
    OrganizationService::new(&database, &session)
        .get()
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_application_settings(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: UpdateApplicationSettingsInput,
) -> Result<ApplicationSettings, CommandError> {
    OrganizationService::new(&database, &session)
        .update(input)
        .map_err(Into::into)
}
