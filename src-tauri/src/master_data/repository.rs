use rusqlite::{params, Connection, OptionalExtension, Transaction};

use super::{DiagnosisRecord, DiluentRecord, DoctorRecord, RouteRecord, WardRecord};

pub(super) fn list_doctors(
    connection: &Connection,
    search: Option<&str>,
) -> rusqlite::Result<Vec<DoctorRecord>> {
    let pattern = search.map(|value| format!("%{}%", escape_like(value)));
    let mut statement = connection.prepare(
        "SELECT id,legacy_doccode,doctor_name
         FROM doctors
         WHERE ?1 IS NULL
            OR legacy_doccode LIKE ?1 ESCAPE '\\' COLLATE NOCASE
            OR doctor_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
         ORDER BY doctor_name COLLATE NOCASE,id",
    )?;
    let rows = statement
        .query_map([pattern], |row| {
            Ok(DoctorRecord {
                id: row.get(0)?,
                legacy_code: row.get(1)?,
                name: row.get(2)?,
            })
        })?
        .collect();
    rows
}

pub(super) fn load_doctor(
    connection: &Connection,
    doctor_id: i64,
) -> rusqlite::Result<Option<DoctorRecord>> {
    connection
        .query_row(
            "SELECT id,legacy_doccode,doctor_name FROM doctors WHERE id=?1",
            [doctor_id],
            |row| {
                Ok(DoctorRecord {
                    id: row.get(0)?,
                    legacy_code: row.get(1)?,
                    name: row.get(2)?,
                })
            },
        )
        .optional()
}

pub(super) fn doctor_code_exists(
    connection: &Connection,
    code: &str,
    excluding_id: Option<i64>,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM doctors
            WHERE legacy_doccode=?1 COLLATE NOCASE AND (?2 IS NULL OR id<>?2)
         )",
        params![code, excluding_id],
        |row| row.get(0),
    )
}

pub(super) fn insert_doctor(
    transaction: &Transaction<'_>,
    legacy_code: Option<&str>,
    name: &str,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO doctors(legacy_doccode,doctor_name) VALUES(?1,?2)",
        params![legacy_code, name],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_doctor(
    transaction: &Transaction<'_>,
    doctor_id: i64,
    legacy_code: Option<&str>,
    name: &str,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE doctors SET legacy_doccode=?1,doctor_name=?2 WHERE id=?3",
        params![legacy_code, name, doctor_id],
    )
}

pub(super) fn list_wards(
    connection: &Connection,
    search: Option<&str>,
) -> rusqlite::Result<Vec<WardRecord>> {
    let pattern = search.map(|value| format!("%{}%", escape_like(value)));
    let mut statement = connection.prepare(
        "SELECT id,legacy_wcode,ward_name,telephone
         FROM wards
         WHERE ?1 IS NULL
            OR legacy_wcode LIKE ?1 ESCAPE '\\' COLLATE NOCASE
            OR ward_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
            OR telephone LIKE ?1 ESCAPE '\\' COLLATE NOCASE
         ORDER BY ward_name COLLATE NOCASE,id",
    )?;
    let rows = statement
        .query_map([pattern], |row| {
            Ok(WardRecord {
                id: row.get(0)?,
                legacy_code: row.get(1)?,
                name: row.get(2)?,
                telephone: row.get(3)?,
            })
        })?
        .collect();
    rows
}

pub(super) fn load_ward(
    connection: &Connection,
    ward_id: i64,
) -> rusqlite::Result<Option<WardRecord>> {
    connection
        .query_row(
            "SELECT id,legacy_wcode,ward_name,telephone FROM wards WHERE id=?1",
            [ward_id],
            |row| {
                Ok(WardRecord {
                    id: row.get(0)?,
                    legacy_code: row.get(1)?,
                    name: row.get(2)?,
                    telephone: row.get(3)?,
                })
            },
        )
        .optional()
}

pub(super) fn ward_code_exists(
    connection: &Connection,
    code: &str,
    excluding_id: Option<i64>,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM wards
            WHERE legacy_wcode=?1 COLLATE NOCASE AND (?2 IS NULL OR id<>?2)
         )",
        params![code, excluding_id],
        |row| row.get(0),
    )
}

pub(super) fn insert_ward(
    transaction: &Transaction<'_>,
    legacy_code: Option<&str>,
    name: &str,
    telephone: Option<&str>,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO wards(legacy_wcode,ward_name,telephone) VALUES(?1,?2,?3)",
        params![legacy_code, name, telephone],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_ward(
    transaction: &Transaction<'_>,
    ward_id: i64,
    legacy_code: Option<&str>,
    name: &str,
    telephone: Option<&str>,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE wards SET legacy_wcode=?1,ward_name=?2,telephone=?3 WHERE id=?4",
        params![legacy_code, name, telephone, ward_id],
    )
}

pub(super) fn list_routes(
    connection: &Connection,
    search: Option<&str>,
) -> rusqlite::Result<Vec<RouteRecord>> {
    let pattern = search.map(|value| format!("%{}%", escape_like(value)));
    let mut statement = connection.prepare(
        "SELECT id,legacy_rcode,route_name
         FROM routes
         WHERE ?1 IS NULL
            OR legacy_rcode LIKE ?1 ESCAPE '\\' COLLATE NOCASE
            OR route_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
         ORDER BY route_name COLLATE NOCASE,id",
    )?;
    let rows = statement
        .query_map([pattern], |row| {
            Ok(RouteRecord {
                id: row.get(0)?,
                legacy_code: row.get(1)?,
                name: row.get(2)?,
            })
        })?
        .collect();
    rows
}

pub(super) fn load_route(
    connection: &Connection,
    route_id: i64,
) -> rusqlite::Result<Option<RouteRecord>> {
    connection
        .query_row(
            "SELECT id,legacy_rcode,route_name FROM routes WHERE id=?1",
            [route_id],
            |row| {
                Ok(RouteRecord {
                    id: row.get(0)?,
                    legacy_code: row.get(1)?,
                    name: row.get(2)?,
                })
            },
        )
        .optional()
}

pub(super) fn route_code_exists(
    connection: &Connection,
    code: &str,
    excluding_id: Option<i64>,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM routes
            WHERE legacy_rcode=?1 COLLATE NOCASE AND (?2 IS NULL OR id<>?2)
         )",
        params![code, excluding_id],
        |row| row.get(0),
    )
}

pub(super) fn insert_route(
    transaction: &Transaction<'_>,
    legacy_code: Option<&str>,
    name: &str,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO routes(legacy_rcode,route_name) VALUES(?1,?2)",
        params![legacy_code, name],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_route(
    transaction: &Transaction<'_>,
    route_id: i64,
    legacy_code: Option<&str>,
    name: &str,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE routes SET legacy_rcode=?1,route_name=?2 WHERE id=?3",
        params![legacy_code, name, route_id],
    )
}

pub(super) fn list_diluents(
    connection: &Connection,
    search: Option<&str>,
) -> rusqlite::Result<Vec<DiluentRecord>> {
    let pattern = search.map(|value| format!("%{}%", escape_like(value)));
    let mut statement = connection.prepare(
        "SELECT id,legacy_dilcode,diluent_name,volume_ml
         FROM diluents
         WHERE ?1 IS NULL
            OR legacy_dilcode LIKE ?1 ESCAPE '\\' COLLATE NOCASE
            OR diluent_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
         ORDER BY diluent_name COLLATE NOCASE,volume_ml,id",
    )?;
    let rows = statement
        .query_map([pattern], |row| {
            Ok(DiluentRecord {
                id: row.get(0)?,
                legacy_code: row.get(1)?,
                name: row.get(2)?,
                volume_ml: row.get(3)?,
            })
        })?
        .collect();
    rows
}

pub(super) fn load_diluent(
    connection: &Connection,
    diluent_id: i64,
) -> rusqlite::Result<Option<DiluentRecord>> {
    connection
        .query_row(
            "SELECT id,legacy_dilcode,diluent_name,volume_ml FROM diluents WHERE id=?1",
            [diluent_id],
            |row| {
                Ok(DiluentRecord {
                    id: row.get(0)?,
                    legacy_code: row.get(1)?,
                    name: row.get(2)?,
                    volume_ml: row.get(3)?,
                })
            },
        )
        .optional()
}

pub(super) fn diluent_code_exists(
    connection: &Connection,
    code: &str,
    excluding_id: Option<i64>,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM diluents
            WHERE legacy_dilcode=?1 COLLATE NOCASE AND (?2 IS NULL OR id<>?2)
         )",
        params![code, excluding_id],
        |row| row.get(0),
    )
}

pub(super) fn insert_diluent(
    transaction: &Transaction<'_>,
    legacy_code: Option<&str>,
    name: &str,
    volume_ml: Option<f64>,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO diluents(legacy_dilcode,diluent_name,volume_ml) VALUES(?1,?2,?3)",
        params![legacy_code, name, volume_ml],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_diluent(
    transaction: &Transaction<'_>,
    diluent_id: i64,
    legacy_code: Option<&str>,
    name: &str,
    volume_ml: Option<f64>,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE diluents SET legacy_dilcode=?1,diluent_name=?2,volume_ml=?3 WHERE id=?4",
        params![legacy_code, name, volume_ml, diluent_id],
    )
}

pub(super) fn list_diagnoses(
    connection: &Connection,
    search: Option<&str>,
) -> rusqlite::Result<Vec<DiagnosisRecord>> {
    let pattern = search.map(|value| format!("%{}%", escape_like(value)));
    let mut statement = connection.prepare(
        "SELECT id,diagnosis
         FROM diagnoses
         WHERE ?1 IS NULL
            OR diagnosis LIKE ?1 ESCAPE '\\' COLLATE NOCASE
         ORDER BY diagnosis COLLATE NOCASE,id",
    )?;
    let rows = statement
        .query_map([pattern], |row| {
            Ok(DiagnosisRecord {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect();
    rows
}

pub(super) fn load_diagnosis(
    connection: &Connection,
    diagnosis_id: i64,
) -> rusqlite::Result<Option<DiagnosisRecord>> {
    connection
        .query_row(
            "SELECT id,diagnosis FROM diagnoses WHERE id=?1",
            [diagnosis_id],
            |row| {
                Ok(DiagnosisRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            },
        )
        .optional()
}

pub(super) fn insert_diagnosis(transaction: &Transaction<'_>, name: &str) -> rusqlite::Result<i64> {
    transaction.execute("INSERT INTO diagnoses(diagnosis) VALUES(?1)", [name])?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_diagnosis(
    transaction: &Transaction<'_>,
    diagnosis_id: i64,
    name: &str,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE diagnoses SET diagnosis=?1 WHERE id=?2",
        params![name, diagnosis_id],
    )
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
