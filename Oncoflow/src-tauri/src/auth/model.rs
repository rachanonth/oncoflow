use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UserRole {
    Pharmacist,
    Admin,
}

impl UserRole {
    pub(crate) fn from_database(value: &str) -> rusqlite::Result<Self> {
        match value {
            "user" => Ok(Self::Pharmacist),
            "admin" => Ok(Self::Admin),
            value => Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                format!("unsupported local role: {value}").into(),
            )),
        }
    }

    pub(crate) const fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }

    pub(crate) const fn as_database(self) -> &'static str {
        match self {
            Self::Pharmacist => "user",
            Self::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UserType {
    Pharmacist,
    NonPharmacist,
}

impl UserType {
    pub(crate) fn from_database(value: &str) -> rusqlite::Result<Self> {
        match value {
            "pharmacist" => Ok(Self::Pharmacist),
            "non_pharmacist" => Ok(Self::NonPharmacist),
            value => Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                format!("unsupported local user type: {value}").into(),
            )),
        }
    }

    pub(crate) const fn as_database(self) -> &'static str {
        match self {
            Self::Pharmacist => "pharmacist",
            Self::NonPharmacist => "non_pharmacist",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentUser {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
    pub user_type: UserType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManagedUser {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
    pub user_type: UserType,
    pub active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthState {
    pub needs_bootstrap: bool,
    pub authenticated: bool,
    pub current_user: Option<CurrentUser>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct BootstrapUserInput {
    pub username: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ChangePasswordInput {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateUserInput {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub user_type: UserType,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateUserInput {
    pub username: String,
    pub display_name: String,
    pub user_type: UserType,
    pub role: UserRole,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub(super) struct CredentialRecord {
    pub user: CurrentUser,
    pub password_hash: String,
    pub active: bool,
    pub credential_kind: String,
}
