use rusqlite::TransactionBehavior;
use serde_json::json;
use thiserror::Error;

use crate::{
    auth::{AuthError, AuthService, AuthSession},
    db::{Database, DatabaseError},
};

use super::{repository, PageGuidanceRecord, UpdatePageGuidanceInput};

const GUIDANCE_MAX_CHARS: usize = 500;
const SUPPORTED_PAGE_KEYS: &[&str] = &[
    "account",
    "backup_restore",
    "diagnoses",
    "diagnostics",
    "diluents",
    "doctors",
    "drug_form",
    "drugs",
    "guidance",
    "general",
    "hardware",
    "inventory",
    "order_form",
    "orders",
    "patient_form",
    "patients",
    "preparation",
    "regimen_form",
    "regimens",
    "routes",
    "users",
    "wards",
];

#[derive(Debug, Error)]
pub(crate) enum GuidanceError {
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

pub(crate) struct GuidanceService<'a> {
    database: &'a Database,
    session: &'a AuthSession,
}

impl<'a> GuidanceService<'a> {
    pub(crate) fn new(database: &'a Database, session: &'a AuthSession) -> Self {
        Self { database, session }
    }

    pub(crate) fn list(&self) -> Result<Vec<PageGuidanceRecord>, GuidanceError> {
        AuthService::new(self.database, self.session).current_user()?;
        Ok(repository::list(&self.database.open()?)?)
    }

    pub(crate) fn update(
        &self,
        input: UpdatePageGuidanceInput,
    ) -> Result<PageGuidanceRecord, GuidanceError> {
        let actor = AuthService::new(self.database, self.session).require_admin()?;
        let page_key = input.page_key.trim();
        if !SUPPORTED_PAGE_KEYS.contains(&page_key) {
            return Err(validation("pageKey", "Choose a supported OncoFlow page"));
        }
        let guidance = input
            .guidance
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if guidance
            .as_ref()
            .is_some_and(|value| value.chars().count() > GUIDANCE_MAX_CHARS)
        {
            return Err(validation(
                "guidance",
                "Guidance is limited to 500 characters",
            ));
        }

        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let event_type = if let Some(value) = guidance.as_deref() {
            repository::upsert(&transaction, page_key, value, actor.id)?;
            "page_guidance_updated"
        } else {
            repository::remove(&transaction, page_key)?;
            "page_guidance_reset"
        };
        crate::auth::audit::append_event(
            &transaction,
            Some(actor.id),
            event_type,
            "page_guidance",
            page_key,
            &json!({"page_key":page_key}),
        )?;
        transaction.commit()?;

        Ok(
            repository::load(&connection, page_key)?.unwrap_or(PageGuidanceRecord {
                page_key: page_key.to_owned(),
                guidance: None,
            }),
        )
    }
}

fn validation(field: &'static str, message: impl Into<String>) -> GuidanceError {
    GuidanceError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthService, BootstrapUserInput, CreateUserInput, LoginInput, UserType};

    const ADMIN_PASSWORD: &str = "synthetic administrator password 42!";

    struct Fixture {
        _directory: tempfile::TempDir,
        database: Database,
        session: AuthSession,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
            let session = AuthSession::default();
            AuthService::new(&database, &session)
                .bootstrap(BootstrapUserInput {
                    username: "local.admin".into(),
                    display_name: "ผู้ดูแลทดสอบ".into(),
                    password: ADMIN_PASSWORD.into(),
                })
                .unwrap();
            Self {
                _directory: directory,
                database,
                session,
            }
        }

        fn service(&self) -> GuidanceService<'_> {
            GuidanceService::new(&self.database, &self.session)
        }
    }

    #[test]
    fn stores_lists_and_resets_trimmed_thai_guidance() {
        let fixture = Fixture::new();
        let saved = fixture
            .service()
            .update(UpdatePageGuidanceInput {
                page_key: "patients".into(),
                guidance: Some("  ตรวจสอบ HN ก่อนสร้างผู้ป่วยใหม่  ".into()),
            })
            .unwrap();
        assert_eq!(saved.guidance.as_deref(), Some("ตรวจสอบ HN ก่อนสร้างผู้ป่วยใหม่"));
        assert_eq!(fixture.service().list().unwrap(), vec![saved]);
        let audit_metadata: String = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT metadata_json FROM audit_events
                 WHERE event_type='page_guidance_updated' ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_metadata, r#"{"page_key":"patients"}"#);
        assert!(!audit_metadata.contains("ตรวจสอบ"));

        let reset = fixture
            .service()
            .update(UpdatePageGuidanceInput {
                page_key: "patients".into(),
                guidance: Some("   ".into()),
            })
            .unwrap();
        assert_eq!(reset.guidance, None);
        assert!(fixture.service().list().unwrap().is_empty());
    }

    #[test]
    fn rejects_unknown_pages_and_overlong_guidance() {
        let fixture = Fixture::new();
        assert!(matches!(
            fixture.service().update(UpdatePageGuidanceInput {
                page_key: "unknown".into(),
                guidance: Some("text".into()),
            }),
            Err(GuidanceError::Validation {
                field: "pageKey",
                ..
            })
        ));
        assert!(matches!(
            fixture.service().update(UpdatePageGuidanceInput {
                page_key: "patients".into(),
                guidance: Some("ก".repeat(501)),
            }),
            Err(GuidanceError::Validation {
                field: "guidance",
                ..
            })
        ));
    }

    #[test]
    fn guidance_and_audit_are_atomic_and_metadata_omits_content() {
        let fixture = Fixture::new();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_guidance_audit_failure BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='page_guidance_updated'
                 BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().update(UpdatePageGuidanceInput {
                page_key: "patients".into(),
                guidance: Some("Synthetic private guidance".into()),
            }),
            Err(GuidanceError::Sqlite(_))
        ));
        let connection = fixture.database.open().unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM page_guidance", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn reading_requires_authentication_and_updates_require_an_administrator() {
        let fixture = Fixture::new();
        AuthService::new(&fixture.database, &fixture.session)
            .create_user(CreateUserInput {
                username: "standard.user".into(),
                display_name: "Standard User".into(),
                password: "synthetic standard user password 84!".into(),
                user_type: UserType::NonPharmacist,
            })
            .unwrap();
        AuthService::new(&fixture.database, &fixture.session)
            .logout()
            .unwrap();
        assert!(matches!(
            fixture.service().list(),
            Err(GuidanceError::Auth(AuthError::AuthenticationRequired))
        ));
        AuthService::new(&fixture.database, &fixture.session)
            .login(LoginInput {
                username: "standard.user".into(),
                password: "synthetic standard user password 84!".into(),
            })
            .unwrap();
        assert!(fixture.service().list().unwrap().is_empty());
        assert!(matches!(
            fixture.service().update(UpdatePageGuidanceInput {
                page_key: "patients".into(),
                guidance: Some("Not allowed".into()),
            }),
            Err(GuidanceError::Auth(AuthError::AdminRequired))
        ));
    }
}
