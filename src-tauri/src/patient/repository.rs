use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use super::{
    LookupOption, PatientDetail, PatientInput, PatientListRequest, PatientListResponse,
    PatientSortField, PatientSummary, SortDirection,
};

pub(super) fn list_patients(
    connection: &Connection,
    request: &PatientListRequest,
) -> rusqlite::Result<PatientListResponse> {
    let search = request.search.as_deref().unwrap_or("").trim();
    let pattern = format!("%{}%", escape_like(search));
    let limit = request.limit.unwrap_or(100).clamp(1, 200) as i64;
    let offset = request.offset.unwrap_or(0) as i64;
    let direction = match request.sort_direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    let order = match request.sort_by {
        PatientSortField::Hn => format!("p.legacy_hn {direction}"),
        PatientSortField::Name => format!(
            "COALESCE(p.last_name, '') {direction}, COALESCE(p.first_name, '') {direction}, p.legacy_hn ASC"
        ),
        PatientSortField::LastUpdated => format!(
            "CASE WHEN p.record_time IS NULL THEN 1 ELSE 0 END ASC, p.record_time {direction}, p.legacy_hn ASC"
        ),
    };
    let where_clause = "(?1 = '%%' OR p.legacy_hn LIKE ?1 ESCAPE '\\' COLLATE NOCASE OR p.first_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE OR p.last_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE)";

    let total = connection.query_row(
        &format!("SELECT COUNT(*) FROM patients p WHERE {where_clause}"),
        [&pattern],
        |row| row.get::<_, u64>(0),
    )?;

    let sql = format!(
        "SELECT p.id, p.legacy_hn, p.title, p.first_name, p.last_name,
                d.diagnosis, r.regimen_name, p.record_time
         FROM patients p
         LEFT JOIN diagnoses d ON d.id = p.diagnosis_id
         LEFT JOIN regimens r ON r.id = p.regimen_id
         WHERE {where_clause}
         ORDER BY {order}
         LIMIT ?2 OFFSET ?3"
    );
    let mut statement = connection.prepare(&sql)?;
    let items = statement
        .query_map(params![pattern, limit, offset], map_patient_summary)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PatientListResponse { items, total })
}

pub(super) fn get_patient(
    connection: &Connection,
    patient_id: i64,
) -> rusqlite::Result<Option<PatientDetail>> {
    connection
        .query_row(
            "SELECT p.id, p.legacy_hn, p.cancer_no, p.title, p.first_name, p.last_name,
                    p.sex, p.telephone, p.weight_kg, p.height_cm, p.birth_date, p.legacy_age,
                    p.occupation, p.address, p.diagnosis_id, d.diagnosis,
                    p.regimen_id, r.regimen_name, p.stage, p.her2, p.erpr,
                    p.allergy, p.patient_history, p.counselling, p.appointment_card,
                    p.treatment_ended, p.treatment_end_date, p.record_by, p.record_time
             FROM patients p
             LEFT JOIN diagnoses d ON d.id = p.diagnosis_id
             LEFT JOIN regimens r ON r.id = p.regimen_id
             WHERE p.id = ?1",
            [patient_id],
            map_patient_detail,
        )
        .optional()
}

#[cfg(test)]
pub(super) fn get_patient_by_hn(
    connection: &Connection,
    hn: &str,
) -> rusqlite::Result<Option<PatientDetail>> {
    let patient_id = connection
        .query_row(
            "SELECT id FROM patients WHERE legacy_hn = ?1",
            [hn],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    patient_id
        .map(|patient_id| get_patient(connection, patient_id))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn hn_exists(
    transaction: &Transaction<'_>,
    hn: &str,
    excluding_id: Option<i64>,
) -> rusqlite::Result<bool> {
    transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM patients
            WHERE legacy_hn = ?1 COLLATE NOCASE
              AND (?2 IS NULL OR id <> ?2)
         )",
        params![hn, excluding_id],
        |row| row.get(0),
    )
}

pub(super) fn lookup_exists(
    transaction: &Transaction<'_>,
    table: &str,
    id: i64,
) -> rusqlite::Result<bool> {
    let sql = match table {
        "diagnoses" => "SELECT EXISTS(SELECT 1 FROM diagnoses WHERE id = ?1)",
        "regimens" => "SELECT EXISTS(SELECT 1 FROM regimens WHERE id = ?1)",
        _ => unreachable!("patient lookup table is allow-listed"),
    };
    transaction.query_row(sql, [id], |row| row.get(0))
}

pub(super) fn insert_patient(
    transaction: &Transaction<'_>,
    input: &PatientInput,
) -> rusqlite::Result<i64> {
    transaction.execute(
        "INSERT INTO patients (
            legacy_hn, cancer_no, title, first_name, last_name, sex, telephone,
            weight_kg, height_cm, birth_date, legacy_age, occupation, address, diagnosis_id,
            regimen_id, stage, her2, erpr, allergy, patient_history, counselling,
            appointment_card, treatment_ended, treatment_end_date, record_time
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, CURRENT_TIMESTAMP
         )",
        params![
            input.hn,
            input.cancer_no,
            input.title,
            input.first_name,
            input.last_name,
            input.sex,
            input.telephone,
            input.weight_kg,
            input.height_cm,
            input.birth_date,
            input.age_years,
            input.occupation,
            input.address,
            input.diagnosis_id,
            input.regimen_id,
            input.stage,
            input.her2,
            input.erpr,
            input.allergy,
            input.patient_history,
            input.counselling as i64,
            input.appointment_card as i64,
            input.treatment_ended.map(i64::from),
            input.treatment_end_date,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn update_patient(
    transaction: &Transaction<'_>,
    patient_id: i64,
    input: &PatientInput,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE patients SET
            legacy_hn = ?1, cancer_no = ?2, title = ?3, first_name = ?4,
            last_name = ?5, sex = ?6, telephone = ?7, weight_kg = ?8,
            height_cm = ?9, birth_date = ?10, legacy_age = ?11, occupation = ?12, address = ?13,
            diagnosis_id = ?14, regimen_id = ?15, stage = ?16, her2 = ?17,
            erpr = ?18, allergy = ?19, patient_history = ?20, counselling = ?21,
            appointment_card = ?22, treatment_ended = ?23,
            treatment_end_date = ?24, record_time = CURRENT_TIMESTAMP
         WHERE id = ?25",
        params![
            input.hn,
            input.cancer_no,
            input.title,
            input.first_name,
            input.last_name,
            input.sex,
            input.telephone,
            input.weight_kg,
            input.height_cm,
            input.birth_date,
            input.age_years,
            input.occupation,
            input.address,
            input.diagnosis_id,
            input.regimen_id,
            input.stage,
            input.her2,
            input.erpr,
            input.allergy,
            input.patient_history,
            input.counselling as i64,
            input.appointment_card as i64,
            input.treatment_ended.map(i64::from),
            input.treatment_end_date,
            patient_id,
        ],
    )
}

pub(super) fn form_options(connection: &Connection) -> rusqlite::Result<super::PatientFormOptions> {
    let diagnoses = load_options(
        connection,
        "SELECT id, legacy_diagcode, diagnosis FROM diagnoses ORDER BY diagnosis, id",
    )?;
    let regimens = load_options(
        connection,
        "SELECT id, legacy_regcode, regimen_name FROM regimens ORDER BY regimen_name, id",
    )?;
    Ok(super::PatientFormOptions {
        diagnoses,
        regimens,
    })
}

fn load_options(connection: &Connection, sql: &str) -> rusqlite::Result<Vec<LookupOption>> {
    connection
        .prepare(sql)?
        .query_map([], |row| {
            Ok(LookupOption {
                id: row.get(0)?,
                code: row.get(1)?,
                label: row.get(2)?,
            })
        })?
        .collect()
}

fn map_patient_summary(row: &Row<'_>) -> rusqlite::Result<PatientSummary> {
    Ok(PatientSummary {
        id: row.get(0)?,
        hn: row.get(1)?,
        title: row.get(2)?,
        first_name: row.get(3)?,
        last_name: row.get(4)?,
        diagnosis: row.get(5)?,
        regimen: row.get(6)?,
        last_updated: row.get(7)?,
    })
}

fn map_patient_detail(row: &Row<'_>) -> rusqlite::Result<PatientDetail> {
    let counselling = row.get::<_, i64>(23)? != 0;
    let appointment_card = row.get::<_, i64>(24)? != 0;
    let treatment_ended = row.get::<_, Option<i64>>(25)?.map(|value| value != 0);
    Ok(PatientDetail {
        id: row.get(0)?,
        hn: row.get(1)?,
        cancer_no: row.get(2)?,
        title: row.get(3)?,
        first_name: row.get(4)?,
        last_name: row.get(5)?,
        sex: row.get(6)?,
        telephone: row.get(7)?,
        weight_kg: row.get(8)?,
        height_cm: row.get(9)?,
        birth_date: row.get(10)?,
        age_years: row.get(11)?,
        occupation: row.get(12)?,
        address: row.get(13)?,
        diagnosis_id: row.get(14)?,
        diagnosis: row.get(15)?,
        regimen_id: row.get(16)?,
        regimen: row.get(17)?,
        stage: row.get(18)?,
        her2: row.get(19)?,
        erpr: row.get(20)?,
        allergy: row.get(21)?,
        patient_history: row.get(22)?,
        counselling,
        appointment_card,
        treatment_ended,
        treatment_end_date: row.get(26)?,
        record_by: row.get(27)?,
        record_time: row.get(28)?,
    })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
