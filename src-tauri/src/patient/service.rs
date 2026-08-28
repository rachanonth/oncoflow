use rusqlite::TransactionBehavior;
use thiserror::Error;

use crate::db::{Database, DatabaseError};

use super::{
    repository, PatientDetail, PatientFormOptions, PatientInput, PatientListRequest,
    PatientListResponse,
};

#[derive(Debug, Error)]
pub(crate) enum PatientError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error("a patient with this HN already exists")]
    DuplicateHn,
    #[error("patient record was not found")]
    NotFound,
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct PatientService<'a> {
    database: &'a Database,
}

impl<'a> PatientService<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub(crate) fn list(
        &self,
        mut request: PatientListRequest,
    ) -> Result<PatientListResponse, PatientError> {
        request.search = clean_optional(request.search);
        let connection = self.database.open()?;
        Ok(repository::list_patients(&connection, &request)?)
    }

    pub(crate) fn get(&self, patient_id: i64) -> Result<PatientDetail, PatientError> {
        let connection = self.database.open()?;
        repository::get_patient(&connection, patient_id)?.ok_or(PatientError::NotFound)
    }

    #[cfg(test)]
    pub(crate) fn get_by_hn(&self, hn: &str) -> Result<PatientDetail, PatientError> {
        let connection = self.database.open()?;
        repository::get_patient_by_hn(&connection, hn.trim())?.ok_or(PatientError::NotFound)
    }

    pub(crate) fn create(&self, input: PatientInput) -> Result<PatientDetail, PatientError> {
        let input = validate_and_normalize(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::hn_exists(&transaction, &input.hn, None)? {
            return Err(PatientError::DuplicateHn);
        }
        validate_lookups(&transaction, &input)?;
        let patient_id = repository::insert_patient(&transaction, &input)?;
        transaction.commit()?;
        repository::get_patient(&connection, patient_id)?.ok_or(PatientError::NotFound)
    }

    pub(crate) fn update(
        &self,
        patient_id: i64,
        input: PatientInput,
    ) -> Result<PatientDetail, PatientError> {
        let input = validate_and_normalize(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::hn_exists(&transaction, &input.hn, Some(patient_id))? {
            return Err(PatientError::DuplicateHn);
        }
        validate_lookups(&transaction, &input)?;
        if repository::update_patient(&transaction, patient_id, &input)? == 0 {
            return Err(PatientError::NotFound);
        }
        transaction.commit()?;
        repository::get_patient(&connection, patient_id)?.ok_or(PatientError::NotFound)
    }

    pub(crate) fn form_options(&self) -> Result<PatientFormOptions, PatientError> {
        let connection = self.database.open()?;
        Ok(repository::form_options(&connection)?)
    }
}

fn validate_and_normalize(mut input: PatientInput) -> Result<PatientInput, PatientError> {
    input.hn = input.hn.trim().to_owned();
    if input.hn.is_empty() {
        return Err(validation("hn", "HN is required"));
    }
    if input.hn.chars().count() > 64 {
        return Err(validation("hn", "HN must be 64 characters or fewer"));
    }

    input.cancer_no = clean_optional(input.cancer_no);
    input.title = clean_optional(input.title);
    input.first_name = clean_optional(input.first_name);
    input.last_name = clean_optional(input.last_name);
    input.sex = clean_optional(input.sex);
    input.telephone = clean_optional(input.telephone);
    input.birth_date = clean_optional(input.birth_date);
    input.occupation = clean_optional(input.occupation);
    input.address = clean_optional(input.address);
    input.stage = clean_optional(input.stage);
    input.her2 = clean_optional(input.her2);
    input.erpr = clean_optional(input.erpr);
    input.allergy = clean_optional(input.allergy);
    input.patient_history = clean_optional(input.patient_history);
    input.treatment_end_date = clean_optional(input.treatment_end_date);

    validate_measurement("weightKg", "Weight", input.weight_kg, 500.0)?;
    validate_measurement("heightCm", "Height", input.height_cm, 300.0)?;
    validate_age(input.age_years)?;
    validate_optional_date("birthDate", input.birth_date.as_deref())?;
    validate_optional_date("treatmentEndDate", input.treatment_end_date.as_deref())?;
    validate_positive_id("diagnosisId", input.diagnosis_id)?;
    validate_positive_id("regimenId", input.regimen_id)?;

    Ok(input)
}

fn validate_age(value: Option<f64>) -> Result<(), PatientError> {
    if let Some(value) = value {
        if !value.is_finite() || !(0.0..=150.0).contains(&value) || value.fract() != 0.0 {
            return Err(validation(
                "ageYears",
                "Age must be a whole number from 0 to 150",
            ));
        }
    }
    Ok(())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

fn validate_measurement(
    field: &'static str,
    label: &str,
    value: Option<f64>,
    maximum: f64,
) -> Result<(), PatientError> {
    if let Some(value) = value {
        if !value.is_finite() || value <= 0.0 || value > maximum {
            return Err(validation(
                field,
                format!("{label} must be greater than 0 and no more than {maximum}"),
            ));
        }
    }
    Ok(())
}

fn validate_positive_id(field: &'static str, value: Option<i64>) -> Result<(), PatientError> {
    if value.is_some_and(|value| value <= 0) {
        return Err(validation(field, "Select a valid option"));
    }
    Ok(())
}

fn validate_optional_date(field: &'static str, value: Option<&str>) -> Result<(), PatientError> {
    let Some(value) = value else {
        return Ok(());
    };
    if !is_valid_iso_date(value) {
        return Err(validation(field, "Use a valid date in YYYY-MM-DD format"));
    }
    Ok(())
}

fn is_valid_iso_date(value: &str) -> bool {
    let parts = value
        .split('-')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>();
    let Ok(parts) = parts else {
        return false;
    };
    if parts.len() != 3 || value.len() != 10 {
        return false;
    }
    let (year, month, day) = (parts[0], parts[1], parts[2]);
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days).contains(&day)
}

fn validate_lookups(
    transaction: &rusqlite::Transaction<'_>,
    input: &PatientInput,
) -> Result<(), PatientError> {
    if let Some(id) = input.diagnosis_id {
        if !repository::lookup_exists(transaction, "diagnoses", id)? {
            return Err(validation(
                "diagnosisId",
                "Selected diagnosis does not exist",
            ));
        }
    }
    if let Some(id) = input.regimen_id {
        if !repository::lookup_exists(transaction, "regimens", id)? {
            return Err(validation("regimenId", "Selected regimen does not exist"));
        }
    }
    Ok(())
}

fn validation(field: &'static str, message: impl Into<String>) -> PatientError {
    PatientError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patient::{PatientSortField, SortDirection};

    struct Fixture {
        _directory: tempfile::TempDir,
        database: Database,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
            let connection = database.open().unwrap();
            connection
                .execute(
                    "INSERT INTO diagnoses(id, legacy_diagcode, diagnosis) VALUES (1, 'D-SYN', 'Synthetic diagnosis')",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO regimens(id, legacy_regcode, regimen_name) VALUES (1, 'R-SYN', 'Synthetic regimen')",
                    [],
                )
                .unwrap();
            Self {
                _directory: directory,
                database,
            }
        }

        fn service(&self) -> PatientService<'_> {
            PatientService::new(&self.database)
        }
    }

    fn input(hn: &str, first_name: Option<&str>, last_name: Option<&str>) -> PatientInput {
        PatientInput {
            hn: hn.to_owned(),
            first_name: first_name.map(str::to_owned),
            last_name: last_name.map(str::to_owned),
            diagnosis_id: Some(1),
            regimen_id: Some(1),
            ..PatientInput::default()
        }
    }

    #[test]
    fn creates_and_looks_up_patient_by_hn() {
        let fixture = Fixture::new();
        fixture
            .service()
            .create(input("SYN-001", Some("Test"), Some("Patient")))
            .unwrap();

        let patient = fixture.service().get_by_hn("SYN-001").unwrap();

        assert_eq!(patient.hn, "SYN-001");
        assert_eq!(patient.first_name.as_deref(), Some("Test"));
        assert_eq!(patient.diagnosis.as_deref(), Some("Synthetic diagnosis"));
    }

    #[test]
    fn searches_hn_and_thai_names_in_sqlite() {
        let fixture = Fixture::new();
        fixture
            .service()
            .create(input("SYN-TH-01", Some("สมชาย"), Some("ทดสอบ")))
            .unwrap();
        fixture
            .service()
            .create(input("SYN-EN-02", Some("Another"), Some("Person")))
            .unwrap();

        let thai = fixture
            .service()
            .list(PatientListRequest {
                search: Some("สมชาย".to_owned()),
                sort_by: PatientSortField::Name,
                sort_direction: SortDirection::Asc,
                ..PatientListRequest::default()
            })
            .unwrap();
        let hn = fixture
            .service()
            .list(PatientListRequest {
                search: Some("TH-01".to_owned()),
                ..PatientListRequest::default()
            })
            .unwrap();

        assert_eq!(thai.total, 1);
        assert_eq!(thai.items[0].hn, "SYN-TH-01");
        assert_eq!(hn.total, 1);
    }

    #[test]
    fn creates_patient_with_null_optional_fields() {
        let fixture = Fixture::new();
        let patient = fixture
            .service()
            .create(PatientInput {
                hn: "  SYN-NULL  ".to_owned(),
                first_name: Some("   ".to_owned()),
                ..PatientInput::default()
            })
            .unwrap();

        assert_eq!(patient.hn, "SYN-NULL");
        assert!(patient.first_name.is_none());
        assert!(patient.weight_kg.is_none());
        assert!(patient.diagnosis_id.is_none());
    }

    #[test]
    fn updates_patient_without_renumbering_id() {
        let fixture = Fixture::new();
        let patient = fixture
            .service()
            .create(input("SYN-UP-01", Some("Before"), None))
            .unwrap();
        let updated = fixture
            .service()
            .update(
                patient.id,
                PatientInput {
                    weight_kg: Some(65.5),
                    ..input("SYN-UP-01", Some("After"), Some("Edit"))
                },
            )
            .unwrap();

        assert_eq!(updated.id, patient.id);
        assert_eq!(updated.first_name.as_deref(), Some("After"));
        assert_eq!(updated.weight_kg, Some(65.5));
    }

    #[test]
    fn rejects_duplicate_hn_and_preserves_both_records() {
        let fixture = Fixture::new();
        let first = fixture
            .service()
            .create(input("SYN-DUP", Some("First"), None))
            .unwrap();
        let second = fixture
            .service()
            .create(input("SYN-OTHER", Some("Second"), None))
            .unwrap();

        let error = fixture
            .service()
            .update(second.id, input(" syn-dup ", Some("Changed"), None))
            .unwrap_err();

        assert!(matches!(error, PatientError::DuplicateHn));
        assert_eq!(fixture.service().get(second.id).unwrap().hn, "SYN-OTHER");
        assert_eq!(fixture.service().get(first.id).unwrap().hn, "SYN-DUP");
    }

    #[test]
    fn rolls_back_create_when_lookup_validation_fails() {
        let fixture = Fixture::new();
        let mut patient = input("SYN-ROLLBACK", Some("Rollback"), None);
        patient.diagnosis_id = Some(99_999);

        let error = fixture.service().create(patient).unwrap_err();

        assert!(matches!(
            error,
            PatientError::Validation {
                field: "diagnosisId",
                ..
            }
        ));
        assert!(matches!(
            fixture.service().get_by_hn("SYN-ROLLBACK"),
            Err(PatientError::NotFound)
        ));
    }

    #[test]
    fn rejects_invalid_dates_and_measurements() {
        let fixture = Fixture::new();
        let mut patient = input("SYN-BAD", None, None);
        patient.birth_date = Some("2025-02-30".to_owned());
        assert!(matches!(
            fixture.service().create(patient),
            Err(PatientError::Validation {
                field: "birthDate",
                ..
            })
        ));

        let mut patient = input("SYN-BAD-2", None, None);
        patient.weight_kg = Some(-1.0);
        assert!(matches!(
            fixture.service().create(patient),
            Err(PatientError::Validation {
                field: "weightKg",
                ..
            })
        ));
    }

    #[test]
    fn stores_exact_age_and_rejects_out_of_range_values() {
        let fixture = Fixture::new();
        let mut patient = input("SYN-AGE", None, None);
        patient.age_years = Some(42.0);
        let saved = fixture.service().create(patient).unwrap();
        assert_eq!(saved.age_years, Some(42.0));

        let mut patient = input("SYN-BAD-AGE", None, None);
        patient.age_years = Some(150.5);
        assert!(matches!(
            fixture.service().create(patient),
            Err(PatientError::Validation {
                field: "ageYears",
                ..
            })
        ));
    }
}
