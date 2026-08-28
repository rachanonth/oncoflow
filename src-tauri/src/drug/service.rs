use rusqlite::TransactionBehavior;
use thiserror::Error;

use crate::db::{Database, DatabaseError};

use super::{
    repository, DrugDetail, DrugFormOptions, DrugInput, DrugListRequest, DrugListResponse,
};

#[derive(Debug, Error)]
pub(crate) enum DrugError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error("a drug with this code already exists")]
    DuplicateCode,
    #[error("drug record was not found")]
    NotFound,
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct DrugService<'a> {
    database: &'a Database,
}

impl<'a> DrugService<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub(crate) fn list(&self, mut request: DrugListRequest) -> Result<DrugListResponse, DrugError> {
        request.search = clean_optional(request.search);
        let connection = self.database.open()?;
        Ok(repository::list_drugs(&connection, &request)?)
    }

    pub(crate) fn get(&self, drug_id: i64) -> Result<DrugDetail, DrugError> {
        let connection = self.database.open()?;
        repository::get_drug(&connection, drug_id)?.ok_or(DrugError::NotFound)
    }

    #[cfg(test)]
    pub(crate) fn get_by_code(&self, code: &str) -> Result<DrugDetail, DrugError> {
        let connection = self.database.open()?;
        repository::get_drug_by_code(&connection, code.trim())?.ok_or(DrugError::NotFound)
    }

    pub(crate) fn create(&self, input: DrugInput) -> Result<DrugDetail, DrugError> {
        let input = validate_and_normalize(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::code_exists(&transaction, &input.code, None)? {
            return Err(DrugError::DuplicateCode);
        }
        validate_lookups(&transaction, &input)?;
        let drug_id = repository::insert_drug(&transaction, &input)?;
        transaction.commit()?;
        repository::get_drug(&connection, drug_id)?.ok_or(DrugError::NotFound)
    }

    pub(crate) fn update(
        &self,
        drug_id: i64,
        mut input: DrugInput,
    ) -> Result<DrugDetail, DrugError> {
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = repository::get_drug(&transaction, drug_id)?.ok_or(DrugError::NotFound)?;
        input.code = existing.code;
        let input = validate_and_normalize(input)?;
        if repository::code_exists(&transaction, &input.code, Some(drug_id))? {
            return Err(DrugError::DuplicateCode);
        }
        validate_lookups(&transaction, &input)?;
        if repository::update_drug(&transaction, drug_id, &input)? == 0 {
            return Err(DrugError::NotFound);
        }
        transaction.commit()?;
        repository::get_drug(&connection, drug_id)?.ok_or(DrugError::NotFound)
    }

    pub(crate) fn form_options(&self) -> Result<DrugFormOptions, DrugError> {
        let connection = self.database.open()?;
        Ok(repository::form_options(&connection)?)
    }
}

fn validate_and_normalize(mut input: DrugInput) -> Result<DrugInput, DrugError> {
    input.code = input.code.trim().to_owned();
    input.name = input.name.trim().to_owned();
    if input.code.is_empty() {
        return Err(validation("code", "Drug code is required"));
    }
    if input.name.is_empty() {
        return Err(validation("name", "Drug name is required"));
    }
    if input.code.chars().count() > 64 {
        return Err(validation(
            "code",
            "Drug code must be 64 characters or fewer",
        ));
    }
    if input.name.chars().count() > 255 {
        return Err(validation(
            "name",
            "Drug name must be 255 characters or fewer",
        ));
    }

    input.package = clean_optional(input.package);
    input.detail = clean_optional(input.detail);
    input.theory = clean_optional(input.theory);
    input.default_rate = clean_optional(input.default_rate);
    input.warning = clean_optional(input.warning);
    input.storage = clean_optional(input.storage);
    input.expiry_time = clean_optional(input.expiry_time);
    input.expiry_storage = clean_optional(input.expiry_storage);
    input.dilution_incompatibility = clean_optional(input.dilution_incompatibility);

    validate_non_negative("dosePerPack", "Dose per pack", input.dose_per_pack)?;
    validate_non_negative(
        "volumePerPackMl",
        "Volume per pack",
        input.volume_per_pack_ml,
    )?;
    validate_non_negative("price", "Price", input.price)?;
    validate_non_negative("maxDose", "Maximum dose", input.max_dose)?;
    validate_non_negative(
        "maxDilutionHard",
        "Maximum dilution threshold",
        input.max_dilution_hard,
    )?;
    validate_non_negative(
        "cumulativeAlertHard",
        "Cumulative alert threshold",
        input.cumulative_alert_hard,
    )?;
    validate_non_negative("inventoryMin", "Minimum inventory", input.inventory_min)?;
    validate_non_negative("inventoryMax", "Maximum inventory", input.inventory_max)?;
    if matches!((input.inventory_min, input.inventory_max), (Some(min), Some(max)) if min > max) {
        return Err(validation(
            "inventoryMax",
            "Maximum inventory must be greater than or equal to minimum inventory",
        ));
    }
    validate_positive_id("unitId", input.unit_id)?;
    validate_positive_id("defaultDiluentId", input.default_diluent_id)?;
    validate_positive_id("defaultRouteId", input.default_route_id)?;
    Ok(input)
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

fn validate_non_negative(
    field: &'static str,
    label: &str,
    value: Option<f64>,
) -> Result<(), DrugError> {
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return Err(validation(
            field,
            format!("{label} must be a finite number of zero or greater"),
        ));
    }
    Ok(())
}

fn validate_positive_id(field: &'static str, value: Option<i64>) -> Result<(), DrugError> {
    if value.is_some_and(|value| value <= 0) {
        return Err(validation(field, "Select a valid option"));
    }
    Ok(())
}

fn validate_lookups(
    transaction: &rusqlite::Transaction<'_>,
    input: &DrugInput,
) -> Result<(), DrugError> {
    for (field, table, id) in [
        ("unitId", "units", input.unit_id),
        ("defaultDiluentId", "diluents", input.default_diluent_id),
        ("defaultRouteId", "routes", input.default_route_id),
    ] {
        if let Some(id) = id {
            if !repository::lookup_exists(transaction, table, id)? {
                return Err(validation(field, "Selected lookup value does not exist"));
            }
        }
    }
    Ok(())
}

fn validation(field: &'static str, message: impl Into<String>) -> DrugError {
    DrugError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drug::{DrugSortField, SortDirection};

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
                    "INSERT INTO units(id, legacy_unitcode, unit_name) VALUES (1, 'U', 'Synthetic unit')",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO routes(id, legacy_rcode, route_name) VALUES (1, 'R', 'Synthetic route')",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO diluents(id, legacy_dilcode, diluent_name, volume_ml) VALUES (1, 'D', 'Synthetic diluent', 100)",
                    [],
                )
                .unwrap();
            Self {
                _directory: directory,
                database,
            }
        }

        fn service(&self) -> DrugService<'_> {
            DrugService::new(&self.database)
        }
    }

    fn input(code: &str, name: &str) -> DrugInput {
        DrugInput {
            code: code.to_owned(),
            name: name.to_owned(),
            unit_id: Some(1),
            default_route_id: Some(1),
            default_diluent_id: Some(1),
            ..DrugInput::default()
        }
    }

    #[test]
    fn creates_lists_and_gets_drug_by_id_and_code() {
        let fixture = Fixture::new();
        let created = fixture
            .service()
            .create(input("SYN-01", "Synthetic medicine"))
            .unwrap();
        let list = fixture.service().list(DrugListRequest::default()).unwrap();
        let by_id = fixture.service().get(created.id).unwrap();
        let by_code = fixture.service().get_by_code("SYN-01").unwrap();

        assert_eq!(list.total, 1);
        assert_eq!(list.items[0].code, "SYN-01");
        assert_eq!(by_id.name, "Synthetic medicine");
        assert_eq!(by_code.id, created.id);
    }

    #[test]
    fn reads_legacy_boolean_fields_stored_with_real_affinity() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        connection
            .execute(
                "INSERT INTO drugs (
                    legacy_dcode, drug_name, max_dilution_alert,
                    cumulative_alert, inventory_cut
                 ) VALUES ('LEGACY-REAL', 'Legacy Boolean medicine', 1.0, 0.0, -1.0)",
                [],
            )
            .unwrap();
        let storage_type: String = connection
            .query_row(
                "SELECT typeof(max_dilution_alert) FROM drugs WHERE legacy_dcode = 'LEGACY-REAL'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(storage_type, "real");
        drop(connection);

        let detail = fixture.service().get_by_code("LEGACY-REAL").unwrap();
        assert_eq!(detail.max_dilution_alert, Some(true));
        assert_eq!(detail.cumulative_alert, Some(false));
        assert_eq!(detail.inventory_cut, Some(true));
    }

    #[test]
    fn searches_by_code_name_and_thai_text() {
        let fixture = Fixture::new();
        fixture
            .service()
            .create(input("TH-SYN", "ยาทดสอบ"))
            .unwrap();
        fixture
            .service()
            .create(input("EN-SYN", "Control medicine"))
            .unwrap();

        for term in ["TH-SYN", "ทดสอบ"] {
            let result = fixture
                .service()
                .list(DrugListRequest {
                    search: Some(term.to_owned()),
                    sort_by: DrugSortField::Name,
                    sort_direction: SortDirection::Asc,
                    ..DrugListRequest::default()
                })
                .unwrap();
            assert_eq!(result.total, 1);
            assert_eq!(result.items[0].code, "TH-SYN");
        }
    }

    #[test]
    fn filters_inventory_enabled_records() {
        let fixture = Fixture::new();
        let mut enabled = input("INV-YES", "Enabled medicine");
        enabled.inventory_enabled = true;
        fixture.service().create(enabled).unwrap();
        fixture
            .service()
            .create(input("INV-NO", "Disabled medicine"))
            .unwrap();

        let result = fixture
            .service()
            .list(DrugListRequest {
                inventory_enabled: Some(true),
                ..DrugListRequest::default()
            })
            .unwrap();
        assert_eq!(result.total, 1);
        assert!(result.items[0].inventory_enabled);
    }

    #[test]
    fn creates_null_optional_fields_and_updates_without_renumbering() {
        let fixture = Fixture::new();
        let created = fixture
            .service()
            .create(DrugInput {
                code: "  NULL-01  ".to_owned(),
                name: "  Nullable medicine  ".to_owned(),
                detail: Some("   ".to_owned()),
                ..DrugInput::default()
            })
            .unwrap();
        assert_eq!(created.code, "NULL-01");
        assert!(created.detail.is_none());
        assert!(created.unit_id.is_none());

        let updated = fixture
            .service()
            .update(
                created.id,
                DrugInput {
                    code: "ATTEMPTED-CHANGE".to_owned(),
                    dose_per_pack: Some(25.5),
                    inventory_min: Some(2.0),
                    inventory_max: Some(10.0),
                    inventory_enabled: true,
                    ..input("NULL-01", "Updated medicine")
                },
            )
            .unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.code, "NULL-01");
        assert_eq!(updated.dose_per_pack, Some(25.5));
        assert_eq!(updated.inventory_min, Some(2.0));
    }

    #[test]
    fn rejects_duplicate_codes_case_insensitively() {
        let fixture = Fixture::new();
        fixture
            .service()
            .create(input("DUP-01", "First medicine"))
            .unwrap();
        let error = fixture
            .service()
            .create(input(" dup-01 ", "Second medicine"))
            .unwrap_err();

        assert!(matches!(error, DrugError::DuplicateCode));
        assert_eq!(
            fixture
                .service()
                .list(DrugListRequest::default())
                .unwrap()
                .total,
            1
        );
    }

    #[test]
    fn rejects_negative_numeric_values_and_invalid_inventory_range() {
        let fixture = Fixture::new();
        let mut negative = input("BAD-NEG", "Negative medicine");
        negative.max_dose = Some(-1.0);
        assert!(matches!(
            fixture.service().create(negative),
            Err(DrugError::Validation {
                field: "maxDose",
                ..
            })
        ));

        let mut range = input("BAD-RANGE", "Range medicine");
        range.inventory_min = Some(20.0);
        range.inventory_max = Some(10.0);
        assert!(matches!(
            fixture.service().create(range),
            Err(DrugError::Validation {
                field: "inventoryMax",
                ..
            })
        ));
    }

    #[test]
    fn retrieves_all_local_lookup_types() {
        let fixture = Fixture::new();
        let options = fixture.service().form_options().unwrap();

        assert_eq!(options.suggested_code, "OF-D000001");
        assert_eq!(options.units.len(), 1);
        assert_eq!(options.routes.len(), 1);
        assert_eq!(options.diluents.len(), 1);
        assert_eq!(options.diluents[0].volume_ml, Some(100.0));
    }

    #[test]
    fn suggests_a_unique_oncoflow_drug_code() {
        let fixture = Fixture::new();
        fixture
            .service()
            .create(input("OF-D000002", "Existing generated-style code"))
            .unwrap();

        assert_eq!(
            fixture.service().form_options().unwrap().suggested_code,
            "OF-D000003"
        );
    }

    #[test]
    fn rolls_back_when_lookup_validation_fails() {
        let fixture = Fixture::new();
        let mut invalid = input("ROLLBACK", "Rollback medicine");
        invalid.unit_id = Some(99_999);
        let error = fixture.service().create(invalid).unwrap_err();

        assert!(matches!(
            error,
            DrugError::Validation {
                field: "unitId",
                ..
            }
        ));
        assert!(matches!(
            fixture.service().get_by_code("ROLLBACK"),
            Err(DrugError::NotFound)
        ));
    }
}
