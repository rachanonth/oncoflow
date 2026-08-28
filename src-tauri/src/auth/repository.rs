use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use super::{CredentialRecord, CurrentUser, ManagedUser, UserRole, UserType};

pub(super) fn active_modern_user_exists(connection: &Connection) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM users
            WHERE credential_kind='argon2id' AND active=1
         )",
        [],
        |row| row.get(0),
    )
}

pub(super) fn username_exists(connection: &Connection, username: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username=?1 COLLATE NOCASE)",
        [username],
        |row| row.get(0),
    )
}

pub(super) fn username_exists_for_other_user(
    connection: &Connection,
    username: &str,
    user_id: i64,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM users WHERE username=?1 COLLATE NOCASE AND id<>?2
         )",
        params![username, user_id],
        |row| row.get(0),
    )
}

pub(super) fn claimable_legacy_user_id(
    transaction: &Transaction<'_>,
    username: &str,
) -> rusqlite::Result<Option<i64>> {
    transaction
        .query_row(
            "SELECT id FROM users
             WHERE username=?1 COLLATE NOCASE
               AND credential_kind='legacy_disabled'
               AND active=0
             LIMIT 1",
            [username],
            |row| row.get(0),
        )
        .optional()
}

pub(super) fn claim_legacy_user(
    transaction: &Transaction<'_>,
    user_id: i64,
    username: &str,
    display_name: &str,
    password_hash: &str,
) -> rusqlite::Result<()> {
    transaction.execute(
        "UPDATE users
         SET username=?1,display_name=?2,password_hash=?3,credential_kind='argon2id',
             role='admin',user_type='pharmacist',active=1,updated_at=CURRENT_TIMESTAMP,
             password_changed_at=CURRENT_TIMESTAMP
         WHERE id=?4 AND credential_kind='legacy_disabled' AND active=0",
        params![username, display_name, password_hash, user_id],
    )?;
    Ok(())
}

pub(super) fn insert_user(
    transaction: &Transaction<'_>,
    username: &str,
    display_name: &str,
    password_hash: &str,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO users(
            username,display_name,password_hash,role,user_type,active,
            credential_kind,updated_at,password_changed_at
         ) VALUES(?1,?2,?3,'admin','pharmacist',1,'argon2id',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
        params![username, display_name, password_hash],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn load_credential(
    connection: &Connection,
    username: &str,
) -> rusqlite::Result<Option<CredentialRecord>> {
    connection
        .query_row(
            "SELECT id,username,COALESCE(display_name,username),role,user_type,
                    password_hash,active,credential_kind
             FROM users WHERE username=?1 COLLATE NOCASE LIMIT 1",
            [username],
            map_credential,
        )
        .optional()
}

pub(super) fn load_credential_by_id(
    connection: &Connection,
    user_id: i64,
) -> rusqlite::Result<Option<CredentialRecord>> {
    connection
        .query_row(
            "SELECT id,username,COALESCE(display_name,username),role,user_type,
                    password_hash,active,credential_kind
             FROM users WHERE id=?1",
            [user_id],
            map_credential,
        )
        .optional()
}

fn map_credential(row: &Row<'_>) -> rusqlite::Result<CredentialRecord> {
    let role = row.get::<_, String>(3)?;
    let user_type = row.get::<_, String>(4)?;
    Ok(CredentialRecord {
        user: CurrentUser {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
            role: UserRole::from_database(&role)?,
            user_type: UserType::from_database(&user_type)?,
        },
        password_hash: row.get(5)?,
        active: row.get::<_, i64>(6)? != 0,
        credential_kind: row.get(7)?,
    })
}

pub(super) fn list_managed_users(connection: &Connection) -> rusqlite::Result<Vec<ManagedUser>> {
    let mut statement = connection.prepare(
        "SELECT id,username,COALESCE(display_name,username),role,user_type,active,
                created_at,updated_at
         FROM users
         WHERE credential_kind='argon2id'
         ORDER BY active DESC,display_name COLLATE NOCASE,username COLLATE NOCASE,id",
    )?;
    let users = statement.query_map([], map_managed_user)?.collect();
    users
}

pub(super) fn load_managed_user(
    connection: &Connection,
    user_id: i64,
) -> rusqlite::Result<Option<ManagedUser>> {
    connection
        .query_row(
            "SELECT id,username,COALESCE(display_name,username),role,user_type,active,
                    created_at,updated_at
             FROM users
             WHERE id=?1 AND credential_kind='argon2id'",
            [user_id],
            map_managed_user,
        )
        .optional()
}

pub(super) fn insert_managed_user(
    transaction: &Transaction<'_>,
    username: &str,
    display_name: &str,
    password_hash: &str,
    user_type: UserType,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO users(
            username,display_name,password_hash,role,user_type,active,
            credential_kind,updated_at,password_changed_at
         ) VALUES(?1,?2,?3,'user',?4,1,'argon2id',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
        params![
            username,
            display_name,
            password_hash,
            user_type.as_database()
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_managed_user(
    transaction: &Transaction<'_>,
    user_id: i64,
    username: &str,
    display_name: &str,
    user_type: UserType,
    role: UserRole,
    active: bool,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE users
         SET username=?1,display_name=?2,user_type=?3,role=?4,active=?5,
             updated_at=CURRENT_TIMESTAMP
         WHERE id=?6 AND credential_kind='argon2id'",
        params![
            username,
            display_name,
            user_type.as_database(),
            role.as_database(),
            i64::from(active),
            user_id
        ],
    )
}

fn map_managed_user(row: &Row<'_>) -> rusqlite::Result<ManagedUser> {
    let role = row.get::<_, String>(3)?;
    let user_type = row.get::<_, String>(4)?;
    Ok(ManagedUser {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        role: UserRole::from_database(&role)?,
        user_type: UserType::from_database(&user_type)?,
        active: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

pub(super) fn update_password(
    transaction: &Transaction<'_>,
    user_id: i64,
    password_hash: &str,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE users
         SET password_hash=?1,credential_kind='argon2id',updated_at=CURRENT_TIMESTAMP,
             password_changed_at=CURRENT_TIMESTAMP
         WHERE id=?2 AND active=1 AND credential_kind='argon2id'",
        params![password_hash, user_id],
    )
}
