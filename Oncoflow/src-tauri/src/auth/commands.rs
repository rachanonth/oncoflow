use serde::Serialize;
use tauri::State;

use crate::db::Database;

use super::{
    AuthError, AuthService, AuthSession, AuthState, BootstrapUserInput, ChangePasswordInput,
    CreateUserInput, CurrentUser, LoginInput, ManagedUser, UpdateUserInput,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<AuthError> for CommandError {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            AuthError::AlreadyBootstrapped => Self::plain(
                "already_bootstrapped",
                "Initial local account setup has already been completed.",
            ),
            AuthError::InvalidCredentials | AuthError::InactiveUser => Self::plain(
                "invalid_credentials",
                "The username or password was not accepted.",
            ),
            AuthError::AuthenticationRequired => Self::plain(
                "authentication_required",
                "Sign in with a local OncoFlow account to continue.",
            ),
            AuthError::AdminRequired => Self::plain(
                "admin_required",
                "Local administrator access is required to manage users.",
            ),
            AuthError::UserNotFound => {
                Self::plain("user_not_found", "The local user could not be found.")
            }
            AuthError::CannotDeactivateCurrentUser => Self::plain(
                "self_deactivation",
                "You cannot deactivate the account currently signed in.",
            ),
            AuthError::CannotChangeCurrentRole => Self::plain(
                "self_role_change",
                "You cannot change the access level of the account currently signed in.",
            ),
            AuthError::SessionUnavailable
            | AuthError::PasswordHashing
            | AuthError::Database(_)
            | AuthError::Sqlite(_) => Self::plain(
                "authentication_error",
                "The local authentication operation could not be completed.",
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
pub(crate) fn get_auth_state(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
) -> Result<AuthState, CommandError> {
    AuthService::new(&database, &session)
        .state()
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn bootstrap_user(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: BootstrapUserInput,
) -> Result<AuthState, CommandError> {
    AuthService::new(&database, &session)
        .bootstrap(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn login(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: LoginInput,
) -> Result<AuthState, CommandError> {
    AuthService::new(&database, &session)
        .login(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn logout(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
) -> Result<AuthState, CommandError> {
    AuthService::new(&database, &session)
        .logout()
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_current_user(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
) -> Result<CurrentUser, CommandError> {
    AuthService::new(&database, &session)
        .current_user()
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn change_password(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: ChangePasswordInput,
) -> Result<(), CommandError> {
    AuthService::new(&database, &session)
        .change_password(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_users(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
) -> Result<Vec<ManagedUser>, CommandError> {
    AuthService::new(&database, &session)
        .list_users()
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_user(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    input: CreateUserInput,
) -> Result<ManagedUser, CommandError> {
    AuthService::new(&database, &session)
        .create_user(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_user(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    user_id: i64,
    input: UpdateUserInput,
) -> Result<ManagedUser, CommandError> {
    AuthService::new(&database, &session)
        .update_user(user_id, input)
        .map_err(Into::into)
}
