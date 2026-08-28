use rusqlite::TransactionBehavior;
use serde_json::json;
use thiserror::Error;

use crate::{
    auth::{audit, AuthError, AuthService, AuthSession},
    db::{Database, DatabaseError},
};

use super::{repository, ApplicationSettings, UpdateApplicationSettingsInput};

const HOSPITAL_NAME_MAX_CHARS: usize = 160;

#[derive(Debug, Error)]
pub(crate) enum OrganizationError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct OrganizationService<'a> {
    database: &'a Database,
    session: &'a AuthSession,
}

impl<'a> OrganizationService<'a> {
    pub(crate) fn new(database: &'a Database, session: &'a AuthSession) -> Self {
        Self { database, session }
    }

    pub(crate) fn get(&self) -> Result<ApplicationSettings, OrganizationError> {
        AuthService::new(self.database, self.session).current_user()?;
        Ok(repository::load(&self.database.open()?)?)
    }

    pub(crate) fn update(
        &self,
        input: UpdateApplicationSettingsInput,
    ) -> Result<ApplicationSettings, OrganizationError> {
        let actor = AuthService::new(self.database, self.session).require_admin()?;
        let hospital_name = input
            .hospital_name
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if hospital_name
            .as_ref()
            .is_some_and(|value| value.chars().count() > HOSPITAL_NAME_MAX_CHARS)
        {
            return Err(validation(
                "hospitalName",
                "Hospital name is limited to 160 characters",
            ));
        }
        if hospital_name
            .as_ref()
            .is_some_and(|value| value.chars().any(char::is_control))
        {
            return Err(validation(
                "hospitalName",
                "Hospital name must be a single line",
            ));
        }

        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        repository::update(&transaction, hospital_name.as_deref(), actor.id)?;
        audit::append_event(
            &transaction,
            Some(actor.id),
            "application_settings_updated",
            "application_settings",
            1,
            &json!({"hospital_name_configured":hospital_name.is_some()}),
        )?;
        transaction.commit()?;
        Ok(repository::load(&connection)?)
    }
}

fn validation(field: &'static str, message: impl Into<String>) -> OrganizationError {
    OrganizationError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthService, BootstrapUserInput};

    #[test]
    fn administrator_updates_and_resets_trimmed_unicode_hospital_name() {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::initialize(directory.path().join("settings.db")).unwrap();
        let session = AuthSession::default();
        AuthService::new(&database, &session)
            .bootstrap(BootstrapUserInput {
                username: "settings.admin".into(),
                display_name: "ผู้ดูแลระบบ".into(),
                password: "synthetic settings password 42!".into(),
            })
            .unwrap();
        let service = OrganizationService::new(&database, &session);

        assert_eq!(service.get().unwrap().hospital_name, None);
        assert_eq!(
            service
                .update(UpdateApplicationSettingsInput {
                    hospital_name: Some("  โรงพยาบาลทดสอบ  ".into()),
                })
                .unwrap()
                .hospital_name
                .as_deref(),
            Some("โรงพยาบาลทดสอบ")
        );
        assert_eq!(
            service
                .update(UpdateApplicationSettingsInput {
                    hospital_name: Some("  ".into()),
                })
                .unwrap()
                .hospital_name,
            None
        );
        assert_eq!(
            database
                .open()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM audit_events WHERE event_type='application_settings_updated'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            2
        );
    }
}
