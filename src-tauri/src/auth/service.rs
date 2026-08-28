use std::sync::Mutex;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use rusqlite::TransactionBehavior;
use serde_json::json;
use thiserror::Error;

use crate::db::{Database, DatabaseError};

use super::{
    audit, repository, AuthState, BootstrapUserInput, ChangePasswordInput, CreateUserInput,
    CurrentUser, LoginInput, ManagedUser, UpdateUserInput,
};

const PASSWORD_MIN_CHARS: usize = 12;
const PASSWORD_MAX_CHARS: usize = 128;
const ARGON2_MEMORY_KIB: u32 = 19 * 1024;
const ARGON2_ITERATIONS: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;

#[derive(Debug, Error)]
pub(crate) enum AuthError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error("initial local account setup has already been completed")]
    AlreadyBootstrapped,
    #[error("invalid username or password")]
    InvalidCredentials,
    #[error("authentication is required")]
    AuthenticationRequired,
    #[error("local account is inactive")]
    InactiveUser,
    #[error("local administrator access is required")]
    AdminRequired,
    #[error("local user was not found")]
    UserNotFound,
    #[error("the current administrator cannot deactivate their own account")]
    CannotDeactivateCurrentUser,
    #[error("the current administrator cannot change their own access level")]
    CannotChangeCurrentRole,
    #[error("the authenticated session could not be accessed")]
    SessionUnavailable,
    #[error("password hashing failed")]
    PasswordHashing,
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Debug, Default)]
pub(crate) struct AuthSession {
    current: Mutex<Option<CurrentUser>>,
}

impl AuthSession {
    pub(crate) fn current_user(&self) -> Result<Option<CurrentUser>, AuthError> {
        self.current
            .lock()
            .map(|value| value.clone())
            .map_err(|_| AuthError::SessionUnavailable)
    }

    pub(crate) fn require_user(&self) -> Result<CurrentUser, AuthError> {
        self.current_user()?
            .ok_or(AuthError::AuthenticationRequired)
    }

    fn set(&self, user: CurrentUser) -> Result<(), AuthError> {
        *self
            .current
            .lock()
            .map_err(|_| AuthError::SessionUnavailable)? = Some(user);
        Ok(())
    }

    pub(crate) fn invalidate(&self) -> Result<(), AuthError> {
        *self
            .current
            .lock()
            .map_err(|_| AuthError::SessionUnavailable)? = None;
        Ok(())
    }
}

pub(crate) struct AuthService<'a> {
    database: &'a Database,
    session: &'a AuthSession,
}

impl<'a> AuthService<'a> {
    pub(crate) fn new(database: &'a Database, session: &'a AuthSession) -> Self {
        Self { database, session }
    }

    pub(crate) fn state(&self) -> Result<AuthState, AuthError> {
        let connection = self.database.open()?;
        let needs_bootstrap = !repository::active_modern_user_exists(&connection)?;
        let current_user = match self.session.current_user()? {
            Some(current) => match repository::load_credential_by_id(&connection, current.id)? {
                Some(record) if record.active && record.credential_kind == "argon2id" => {
                    Some(record.user)
                }
                _ => {
                    self.session.invalidate()?;
                    None
                }
            },
            None => None,
        };
        Ok(AuthState {
            needs_bootstrap,
            authenticated: current_user.is_some(),
            current_user,
        })
    }

    pub(crate) fn bootstrap(&self, input: BootstrapUserInput) -> Result<AuthState, AuthError> {
        let (username, display_name) = validate_identity(input.username, input.display_name)?;
        validate_password(&input.password, &username)?;
        let password_hash = hash_password(&input.password)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::active_modern_user_exists(&transaction)? {
            return Err(AuthError::AlreadyBootstrapped);
        }
        let user_id =
            if let Some(user_id) = repository::claimable_legacy_user_id(&transaction, &username)? {
                repository::claim_legacy_user(
                    &transaction,
                    user_id,
                    &username,
                    &display_name,
                    &password_hash,
                )?;
                user_id
            } else {
                if repository::username_exists(&transaction, &username)? {
                    return Err(validation("username", "Choose a different local username"));
                }
                repository::insert_user(&transaction, &username, &display_name, &password_hash)?
            };
        audit::append_event(
            &transaction,
            Some(user_id),
            "user_bootstrapped",
            "user",
            user_id,
            &json!({"credential_kind":"argon2id","source":"first_run"}),
        )?;
        transaction.commit()?;
        let record = repository::load_credential_by_id(&connection, user_id)?
            .ok_or(AuthError::InvalidCredentials)?;
        self.session.set(record.user)?;
        self.state()
    }

    pub(crate) fn login(&self, input: LoginInput) -> Result<AuthState, AuthError> {
        let username = input.username.trim();
        if username.is_empty() || input.password.is_empty() {
            return Err(AuthError::InvalidCredentials);
        }
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let record = repository::load_credential(&transaction, username)?
            .ok_or(AuthError::InvalidCredentials)?;
        if !record.active {
            return Err(AuthError::InactiveUser);
        }
        if record.credential_kind != "argon2id"
            || !verify_password(&input.password, &record.password_hash)
        {
            return Err(AuthError::InvalidCredentials);
        }
        audit::append_event(
            &transaction,
            Some(record.user.id),
            "user_login",
            "user",
            record.user.id,
            &json!({"result":"success"}),
        )?;
        transaction.commit()?;
        self.session.set(record.user)?;
        self.state()
    }

    pub(crate) fn logout(&self) -> Result<AuthState, AuthError> {
        if let Some(user) = self.session.current_user()? {
            let mut connection = self.database.open()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            audit::append_event(
                &transaction,
                Some(user.id),
                "user_logout",
                "user",
                user.id,
                &json!({}),
            )?;
            transaction.commit()?;
        }
        self.session.invalidate()?;
        self.state()
    }

    pub(crate) fn current_user(&self) -> Result<CurrentUser, AuthError> {
        let state = self.state()?;
        state.current_user.ok_or(AuthError::AuthenticationRequired)
    }

    pub(crate) fn change_password(&self, input: ChangePasswordInput) -> Result<(), AuthError> {
        let user = self.current_user()?;
        validate_password(&input.new_password, &user.username)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let record = repository::load_credential_by_id(&transaction, user.id)?
            .ok_or(AuthError::AuthenticationRequired)?;
        if !record.active {
            return Err(AuthError::InactiveUser);
        }
        if record.credential_kind != "argon2id"
            || !verify_password(&input.current_password, &record.password_hash)
        {
            return Err(AuthError::InvalidCredentials);
        }
        let password_hash = hash_password(&input.new_password)?;
        if repository::update_password(&transaction, user.id, &password_hash)? != 1 {
            return Err(AuthError::AuthenticationRequired);
        }
        audit::append_event(
            &transaction,
            Some(user.id),
            "password_changed",
            "user",
            user.id,
            &json!({"credential_kind":"argon2id"}),
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn list_users(&self) -> Result<Vec<ManagedUser>, AuthError> {
        self.require_admin()?;
        let connection = self.database.open()?;
        Ok(repository::list_managed_users(&connection)?)
    }

    pub(crate) fn create_user(&self, input: CreateUserInput) -> Result<ManagedUser, AuthError> {
        let actor = self.require_admin()?;
        let (username, display_name) = validate_identity(input.username, input.display_name)?;
        validate_password(&input.password, &username)?;
        let password_hash = hash_password(&input.password)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::username_exists(&transaction, &username)? {
            return Err(validation("username", "Choose a different local username"));
        }
        let user_id = repository::insert_managed_user(
            &transaction,
            &username,
            &display_name,
            &password_hash,
            input.user_type,
        )?;
        audit::append_event(
            &transaction,
            Some(actor.id),
            "user_created",
            "user",
            user_id,
            &json!({"user_type":input.user_type.as_database(),"active":true}),
        )?;
        transaction.commit()?;
        repository::load_managed_user(&connection, user_id)?.ok_or(AuthError::UserNotFound)
    }

    pub(crate) fn update_user(
        &self,
        user_id: i64,
        input: UpdateUserInput,
    ) -> Result<ManagedUser, AuthError> {
        let actor = self.require_admin()?;
        if actor.id == user_id && !input.active {
            return Err(AuthError::CannotDeactivateCurrentUser);
        }
        if actor.id == user_id && input.role != actor.role {
            return Err(AuthError::CannotChangeCurrentRole);
        }
        let (username, display_name) = validate_identity(input.username, input.display_name)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::load_managed_user(&transaction, user_id)?.is_none() {
            return Err(AuthError::UserNotFound);
        }
        if repository::username_exists_for_other_user(&transaction, &username, user_id)? {
            return Err(validation("username", "Choose a different local username"));
        }
        if repository::update_managed_user(
            &transaction,
            user_id,
            &username,
            &display_name,
            input.user_type,
            input.role,
            input.active,
        )? != 1
        {
            return Err(AuthError::UserNotFound);
        }
        audit::append_event(
            &transaction,
            Some(actor.id),
            "user_updated",
            "user",
            user_id,
            &json!({"user_type":input.user_type.as_database(),"role":input.role.as_database(),"active":input.active}),
        )?;
        transaction.commit()?;
        let managed =
            repository::load_managed_user(&connection, user_id)?.ok_or(AuthError::UserNotFound)?;
        if actor.id == user_id {
            let credential = repository::load_credential_by_id(&connection, user_id)?
                .ok_or(AuthError::UserNotFound)?;
            self.session.set(credential.user)?;
        }
        Ok(managed)
    }

    pub(crate) fn require_admin(&self) -> Result<CurrentUser, AuthError> {
        let user = self.current_user()?;
        if !user.role.is_admin() {
            return Err(AuthError::AdminRequired);
        }
        Ok(user)
    }
}

fn validate_identity(
    username: String,
    display_name: String,
) -> Result<(String, String), AuthError> {
    let username = username.trim().to_owned();
    let display_name = display_name.trim().to_owned();
    if !(3..=64).contains(&username.chars().count())
        || username
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(validation(
            "username",
            "Username must be 3–64 characters without spaces",
        ));
    }
    if display_name.is_empty() || display_name.chars().count() > 100 {
        return Err(validation(
            "displayName",
            "Display name is required and limited to 100 characters",
        ));
    }
    Ok((username, display_name))
}

fn validate_password(password: &str, username: &str) -> Result<(), AuthError> {
    let length = password.chars().count();
    if !(PASSWORD_MIN_CHARS..=PASSWORD_MAX_CHARS).contains(&length) {
        return Err(validation(
            "password",
            format!("Password must be {PASSWORD_MIN_CHARS}–{PASSWORD_MAX_CHARS} characters"),
        ));
    }
    if password.eq_ignore_ascii_case(username) {
        return Err(validation(
            "password",
            "Password must be different from the username",
        ));
    }
    Ok(())
}

fn argon2() -> Result<Argon2<'static>, AuthError> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        None,
    )
    .map_err(|_| AuthError::PasswordHashing)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    argon2()?
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError::PasswordHashing)
}

fn verify_password(password: &str, encoded_hash: &str) -> bool {
    let Ok(hash) = PasswordHash::new(encoded_hash) else {
        return false;
    };
    argon2()
        .and_then(|argon2| {
            argon2
                .verify_password(password.as_bytes(), &hash)
                .map_err(|_| AuthError::InvalidCredentials)
        })
        .is_ok()
}

fn validation(field: &'static str, message: impl Into<String>) -> AuthError {
    AuthError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "synthetic local password 42!";
    const NEW_PASSWORD: &str = "different synthetic password 84!";

    struct Fixture {
        _directory: tempfile::TempDir,
        database: Database,
        session: AuthSession,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
            Self {
                _directory: directory,
                database,
                session: AuthSession::default(),
            }
        }

        fn service(&self) -> AuthService<'_> {
            AuthService::new(&self.database, &self.session)
        }

        fn bootstrap(&self) -> AuthState {
            self.service()
                .bootstrap(BootstrapUserInput {
                    username: "local.pharmacist".into(),
                    display_name: "เภสัชกรทดสอบ".into(),
                    password: PASSWORD.into(),
                })
                .unwrap()
        }
    }

    #[test]
    fn fresh_database_has_no_default_credentials_and_requires_bootstrap() {
        let fixture = Fixture::new();
        let state = fixture.service().state().unwrap();
        assert!(state.needs_bootstrap);
        assert!(!state.authenticated);
        assert!(state.current_user.is_none());
        let users: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .unwrap();
        assert_eq!(users, 0);
        assert!(matches!(
            fixture.service().login(LoginInput {
                username: "admin".into(),
                password: "admin".into()
            }),
            Err(AuthError::InvalidCredentials)
        ));
    }

    #[test]
    fn bootstraps_first_user_with_argon2id_and_cannot_bootstrap_a_second() {
        let fixture = Fixture::new();
        let state = fixture.bootstrap();
        assert!(!state.needs_bootstrap);
        assert!(state.authenticated);
        assert_eq!(
            state.current_user.as_ref().unwrap().display_name,
            "เภสัชกรทดสอบ"
        );
        let connection = fixture.database.open().unwrap();
        let stored: (String, String, i64) = connection
            .query_row(
                "SELECT password_hash,credential_kind,active FROM users",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert!(stored.0.starts_with("$argon2id$v=19$"));
        assert_ne!(stored.0, PASSWORD);
        assert_eq!((stored.1.as_str(), stored.2), ("argon2id", 1));
        let serialized = serde_json::to_string(&state).unwrap();
        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("argon2"));
        assert!(matches!(
            fixture.service().bootstrap(BootstrapUserInput {
                username: "second".into(),
                display_name: "Second".into(),
                password: NEW_PASSWORD.into()
            }),
            Err(AuthError::AlreadyBootstrapped)
        ));
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn bootstrap_can_claim_disabled_legacy_identity_without_using_legacy_password() {
        let fixture = Fixture::new();
        fixture.database.open().unwrap().execute(
            "INSERT INTO users(id,legacy_user,username,display_name,password_hash,role,active,credential_kind)
             VALUES(7,'LEG','legacy.user','Legacy display','LEGACY_CREDENTIAL_DISABLED','user',0,'legacy_disabled')",
            [],
        ).unwrap();
        let state = fixture
            .service()
            .bootstrap(BootstrapUserInput {
                username: "legacy.user".into(),
                display_name: "New local display".into(),
                password: PASSWORD.into(),
            })
            .unwrap();
        assert_eq!(state.current_user.as_ref().unwrap().id, 7);
        let row: (Option<String>, String, String, i64) = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT legacy_user,credential_kind,role,active FROM users WHERE id=7",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            row,
            (Some("LEG".into()), "argon2id".into(), "admin".into(), 1)
        );
    }

    #[test]
    fn login_failure_success_logout_and_authenticated_state_are_process_local() {
        let fixture = Fixture::new();
        fixture.bootstrap();
        fixture.service().logout().unwrap();
        assert!(matches!(
            fixture.service().login(LoginInput {
                username: "local.pharmacist".into(),
                password: "incorrect synthetic password".into()
            }),
            Err(AuthError::InvalidCredentials)
        ));
        assert!(!fixture.service().state().unwrap().authenticated);
        let logged_in = fixture
            .service()
            .login(LoginInput {
                username: "LOCAL.PHARMACIST".into(),
                password: PASSWORD.into(),
            })
            .unwrap();
        assert!(logged_in.authenticated);
        assert_eq!(
            fixture.service().current_user().unwrap().username,
            "local.pharmacist"
        );
        let logged_out = fixture.service().logout().unwrap();
        assert!(!logged_out.authenticated);
        assert!(matches!(
            fixture.service().current_user(),
            Err(AuthError::AuthenticationRequired)
        ));
    }

    #[test]
    fn inactive_user_is_rejected_and_an_existing_session_is_cleared() {
        let fixture = Fixture::new();
        let state = fixture.bootstrap();
        let user_id = state.current_user.unwrap().id;
        fixture
            .database
            .open()
            .unwrap()
            .execute("UPDATE users SET active=0 WHERE id=?1", [user_id])
            .unwrap();
        let state = fixture.service().state().unwrap();
        assert!(!state.authenticated);
        assert!(state.needs_bootstrap);
        assert!(matches!(
            fixture.service().login(LoginInput {
                username: "local.pharmacist".into(),
                password: PASSWORD.into()
            }),
            Err(AuthError::InactiveUser)
        ));
    }

    #[test]
    fn inactive_only_database_can_establish_a_new_first_active_account() {
        let fixture = Fixture::new();
        let first = fixture.bootstrap().current_user.unwrap();
        fixture.service().logout().unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute("UPDATE users SET active=0 WHERE id=?1", [first.id])
            .unwrap();
        let recovered = fixture
            .service()
            .bootstrap(BootstrapUserInput {
                username: "recovery.admin".into(),
                display_name: "Synthetic Recovery Admin".into(),
                password: NEW_PASSWORD.into(),
            })
            .unwrap();
        assert!(recovered.authenticated);
        assert!(!recovered.needs_bootstrap);
        assert_eq!(recovered.current_user.unwrap().username, "recovery.admin");
    }

    #[test]
    fn password_change_invalidates_old_password_and_accepts_new_password() {
        let fixture = Fixture::new();
        fixture.bootstrap();
        fixture
            .service()
            .change_password(ChangePasswordInput {
                current_password: PASSWORD.into(),
                new_password: NEW_PASSWORD.into(),
            })
            .unwrap();
        fixture.service().logout().unwrap();
        assert!(matches!(
            fixture.service().login(LoginInput {
                username: "local.pharmacist".into(),
                password: PASSWORD.into()
            }),
            Err(AuthError::InvalidCredentials)
        ));
        assert!(
            fixture
                .service()
                .login(LoginInput {
                    username: "local.pharmacist".into(),
                    password: NEW_PASSWORD.into()
                })
                .unwrap()
                .authenticated
        );
    }

    #[test]
    fn audit_events_append_without_sensitive_payload_and_reject_update_or_delete() {
        let fixture = Fixture::new();
        fixture.bootstrap();
        fixture.service().logout().unwrap();
        fixture
            .service()
            .login(LoginInput {
                username: "local.pharmacist".into(),
                password: PASSWORD.into(),
            })
            .unwrap();
        let connection = fixture.database.open().unwrap();
        let events: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(events, 3);
        let metadata = connection
            .prepare("SELECT metadata_json FROM audit_events ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("|");
        assert!(!metadata.contains(PASSWORD));
        assert!(!metadata.contains("$argon2"));
        assert!(!metadata.contains("local.pharmacist"));
        assert!(connection
            .execute(
                "UPDATE audit_events SET event_type='changed' WHERE id=1",
                []
            )
            .is_err());
        assert!(connection
            .execute("DELETE FROM audit_events WHERE id=1", [])
            .is_err());
    }

    #[test]
    fn bootstrap_user_and_audit_event_roll_back_together() {
        let fixture = Fixture::new();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_auth_audit_failure BEFORE INSERT ON audit_events
             WHEN NEW.event_type='user_bootstrapped'
             BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().bootstrap(BootstrapUserInput {
                username: "local.pharmacist".into(),
                display_name: "Synthetic".into(),
                password: PASSWORD.into()
            }),
            Err(AuthError::Sqlite(_))
        ));
        let connection = fixture.database.open().unwrap();
        let counts: (i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM users),(SELECT COUNT(*) FROM audit_events)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(counts, (0, 0));
        assert!(fixture.service().state().unwrap().needs_bootstrap);
    }

    #[test]
    fn independent_bootstraps_use_unique_random_salts() {
        let first = Fixture::new();
        let second = Fixture::new();
        first.bootstrap();
        second.bootstrap();
        let hash = |fixture: &Fixture| {
            fixture
                .database
                .open()
                .unwrap()
                .query_row("SELECT password_hash FROM users", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap()
        };
        assert_ne!(hash(&first), hash(&second));
    }

    #[test]
    fn administrator_creates_and_lists_both_user_types_without_exposing_hashes() {
        let fixture = Fixture::new();
        let admin = fixture.bootstrap().current_user.unwrap();
        assert_eq!(admin.user_type, super::super::UserType::Pharmacist);

        let pharmacist = fixture
            .service()
            .create_user(CreateUserInput {
                username: "second.pharmacist".into(),
                display_name: "เภสัชกรสอง".into(),
                password: NEW_PASSWORD.into(),
                user_type: super::super::UserType::Pharmacist,
            })
            .unwrap();
        let support = fixture
            .service()
            .create_user(CreateUserInput {
                username: "local.support".into(),
                display_name: "เจ้าหน้าที่ทดสอบ".into(),
                password: "synthetic support password 77!".into(),
                user_type: super::super::UserType::NonPharmacist,
            })
            .unwrap();

        assert_eq!(pharmacist.user_type, super::super::UserType::Pharmacist);
        assert_eq!(support.user_type, super::super::UserType::NonPharmacist);
        let users = fixture.service().list_users().unwrap();
        assert_eq!(users.len(), 3);
        let serialized = serde_json::to_string(&users).unwrap();
        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("argon2"));
        let stored_hash: String = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT password_hash FROM users WHERE id=?1",
                [support.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(stored_hash.starts_with("$argon2id$v=19$"));
    }

    #[test]
    fn non_administrator_cannot_manage_users() {
        let fixture = Fixture::new();
        fixture.bootstrap();
        fixture
            .service()
            .create_user(CreateUserInput {
                username: "local.support".into(),
                display_name: "Synthetic Support".into(),
                password: NEW_PASSWORD.into(),
                user_type: super::super::UserType::NonPharmacist,
            })
            .unwrap();
        fixture.service().logout().unwrap();
        fixture
            .service()
            .login(LoginInput {
                username: "local.support".into(),
                password: NEW_PASSWORD.into(),
            })
            .unwrap();
        assert!(matches!(
            fixture.service().list_users(),
            Err(AuthError::AdminRequired)
        ));
        assert!(matches!(
            fixture.service().create_user(CreateUserInput {
                username: "not.allowed".into(),
                display_name: "Not Allowed".into(),
                password: "another synthetic password 90!".into(),
                user_type: super::super::UserType::Pharmacist,
            }),
            Err(AuthError::AdminRequired)
        ));
    }

    #[test]
    fn administrator_updates_type_and_activation_with_append_only_audit() {
        let fixture = Fixture::new();
        let admin = fixture.bootstrap().current_user.unwrap();
        let user = fixture
            .service()
            .create_user(CreateUserInput {
                username: "local.staff".into(),
                display_name: "Local Staff".into(),
                password: NEW_PASSWORD.into(),
                user_type: super::super::UserType::NonPharmacist,
            })
            .unwrap();
        let updated = fixture
            .service()
            .update_user(
                user.id,
                UpdateUserInput {
                    username: "local.staff".into(),
                    display_name: "เภสัชกรสาม".into(),
                    user_type: super::super::UserType::Pharmacist,
                    role: super::super::UserRole::Pharmacist,
                    active: false,
                },
            )
            .unwrap();
        assert_eq!(updated.display_name, "เภสัชกรสาม");
        assert_eq!(updated.user_type, super::super::UserType::Pharmacist);
        assert!(!updated.active);
        assert!(matches!(
            fixture.service().update_user(
                admin.id,
                UpdateUserInput {
                    username: admin.username,
                    display_name: admin.display_name,
                    user_type: admin.user_type,
                    role: super::super::UserRole::Admin,
                    active: false,
                }
            ),
            Err(AuthError::CannotDeactivateCurrentUser)
        ));
        let connection = fixture.database.open().unwrap();
        let events: Vec<(String, String)> = connection
            .prepare(
                "SELECT event_type,metadata_json FROM audit_events
                 WHERE event_type IN ('user_created','user_updated') ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(events.len(), 2);
        assert!(events[1].1.contains("pharmacist"));
        assert!(!events.iter().any(|event| event.1.contains("local.staff")));
    }

    #[test]
    fn administrator_can_promote_and_demote_another_modern_user() {
        let fixture = Fixture::new();
        let admin = fixture.bootstrap().current_user.unwrap();
        let user = fixture
            .service()
            .create_user(CreateUserInput {
                username: "local.standard".into(),
                display_name: "Standard User".into(),
                password: NEW_PASSWORD.into(),
                user_type: super::super::UserType::NonPharmacist,
            })
            .unwrap();
        assert_eq!(user.role, super::super::UserRole::Pharmacist);

        let promoted = fixture
            .service()
            .update_user(
                user.id,
                UpdateUserInput {
                    username: user.username.clone(),
                    display_name: user.display_name.clone(),
                    user_type: user.user_type,
                    role: super::super::UserRole::Admin,
                    active: true,
                },
            )
            .unwrap();
        assert_eq!(promoted.role, super::super::UserRole::Admin);

        fixture.service().logout().unwrap();
        fixture
            .service()
            .login(LoginInput {
                username: "local.standard".into(),
                password: NEW_PASSWORD.into(),
            })
            .unwrap();
        assert_eq!(fixture.service().list_users().unwrap().len(), 2);
        fixture.service().logout().unwrap();
        fixture
            .service()
            .login(LoginInput {
                username: "local.pharmacist".into(),
                password: PASSWORD.into(),
            })
            .unwrap();

        let demoted = fixture
            .service()
            .update_user(
                user.id,
                UpdateUserInput {
                    username: promoted.username,
                    display_name: promoted.display_name,
                    user_type: promoted.user_type,
                    role: super::super::UserRole::Pharmacist,
                    active: true,
                },
            )
            .unwrap();
        assert_eq!(demoted.role, super::super::UserRole::Pharmacist);
        assert!(matches!(
            fixture.service().update_user(
                admin.id,
                UpdateUserInput {
                    username: admin.username,
                    display_name: admin.display_name,
                    user_type: admin.user_type,
                    role: super::super::UserRole::Pharmacist,
                    active: true,
                }
            ),
            Err(AuthError::CannotChangeCurrentRole)
        ));

        let role_events: Vec<String> = fixture
            .database
            .open()
            .unwrap()
            .prepare("SELECT metadata_json FROM audit_events WHERE event_type='user_updated' ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(role_events
            .iter()
            .any(|metadata| metadata.contains("\"role\":\"admin\"")));
        assert!(role_events
            .iter()
            .any(|metadata| metadata.contains("\"role\":\"user\"")));
        assert!(!role_events
            .iter()
            .any(|metadata| metadata.contains("local.standard")));
    }

    #[test]
    fn role_change_and_audit_are_atomic() {
        let fixture = Fixture::new();
        fixture.bootstrap();
        let user = fixture
            .service()
            .create_user(CreateUserInput {
                username: "atomic.user".into(),
                display_name: "Atomic User".into(),
                password: NEW_PASSWORD.into(),
                user_type: super::super::UserType::Pharmacist,
            })
            .unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_user_update_audit_failure BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='user_updated'
                 BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().update_user(
                user.id,
                UpdateUserInput {
                    username: user.username,
                    display_name: user.display_name,
                    user_type: user.user_type,
                    role: super::super::UserRole::Admin,
                    active: true,
                }
            ),
            Err(AuthError::Sqlite(_))
        ));
        assert_eq!(
            repository::load_managed_user(&fixture.database.open().unwrap(), user.id)
                .unwrap()
                .unwrap()
                .role,
            super::super::UserRole::Pharmacist
        );
    }

    #[test]
    fn user_creation_and_audit_are_atomic() {
        let fixture = Fixture::new();
        fixture.bootstrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_user_create_audit_failure BEFORE INSERT ON audit_events
             WHEN NEW.event_type='user_created'
             BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().create_user(CreateUserInput {
                username: "rolled.back".into(),
                display_name: "Rolled Back".into(),
                password: NEW_PASSWORD.into(),
                user_type: super::super::UserType::NonPharmacist,
            }),
            Err(AuthError::Sqlite(_))
        ));
        assert!(
            !repository::username_exists(&fixture.database.open().unwrap(), "rolled.back").unwrap()
        );
    }
}
