use std::collections::HashSet;

use rusqlite::TransactionBehavior;
use thiserror::Error;

use crate::db::{Database, DatabaseError};

use super::{
    repository, NormalizedRegimenItemInput, RegimenDetail, RegimenGroupInput, RegimenInput,
    RegimenItemInput, RegimenListRequest, RegimenListResponse, RegimenLookups, RegimenReorderInput,
};

#[derive(Debug, Error)]
pub(crate) enum RegimenError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error("a regimen with this code already exists")]
    DuplicateCode,
    #[error("regimen record was not found")]
    RegimenNotFound,
    #[error("regimen treatment group was not found")]
    GroupNotFound,
    #[error("regimen item was not found")]
    ItemNotFound,
    #[error("a treatment group containing items cannot be removed")]
    GroupNotEmpty,
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct RegimenService<'a> {
    database: &'a Database,
}

impl<'a> RegimenService<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub(crate) fn list(
        &self,
        mut request: RegimenListRequest,
    ) -> Result<RegimenListResponse, RegimenError> {
        request.search = clean_optional(request.search);
        let connection = self.database.open()?;
        Ok(repository::list_regimens(&connection, &request)?)
    }

    pub(crate) fn get(&self, regimen_id: i64) -> Result<RegimenDetail, RegimenError> {
        let connection = self.database.open()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    #[cfg(test)]
    fn get_by_code(&self, code: &str) -> Result<RegimenDetail, RegimenError> {
        let connection = self.database.open()?;
        repository::get_regimen_by_code(&connection, code.trim())?
            .ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn create(&self, input: RegimenInput) -> Result<RegimenDetail, RegimenError> {
        let input = validate_header(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::code_exists(&transaction, &input.code, None)? {
            return Err(RegimenError::DuplicateCode);
        }
        let regimen_id = repository::insert_regimen(&transaction, &input)?;
        transaction.commit()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn update(
        &self,
        regimen_id: i64,
        input: RegimenInput,
    ) -> Result<RegimenDetail, RegimenError> {
        let input = validate_header(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::code_exists(&transaction, &input.code, Some(regimen_id))? {
            return Err(RegimenError::DuplicateCode);
        }
        if repository::update_regimen(&transaction, regimen_id, &input)? == 0 {
            return Err(RegimenError::RegimenNotFound);
        }
        transaction.commit()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn add_group(
        &self,
        regimen_id: i64,
        input: RegimenGroupInput,
    ) -> Result<RegimenDetail, RegimenError> {
        let input = validate_group(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !repository::regimen_exists(&transaction, regimen_id)? {
            return Err(RegimenError::RegimenNotFound);
        }
        repository::insert_group(&transaction, regimen_id, &input)?;
        transaction.commit()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn update_group(
        &self,
        regimen_id: i64,
        group_id: i64,
        input: RegimenGroupInput,
    ) -> Result<RegimenDetail, RegimenError> {
        let input = validate_group(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_group(&transaction, regimen_id, group_id)?;
        repository::update_group(&transaction, group_id, &input)?;
        transaction.commit()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn delete_group(
        &self,
        regimen_id: i64,
        group_id: i64,
    ) -> Result<RegimenDetail, RegimenError> {
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_group(&transaction, regimen_id, group_id)?;
        if repository::group_item_count(&transaction, group_id)? > 0 {
            return Err(RegimenError::GroupNotEmpty);
        }
        repository::delete_group(&transaction, group_id)?;
        transaction.commit()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn add_item(
        &self,
        regimen_id: i64,
        input: RegimenItemInput,
    ) -> Result<RegimenDetail, RegimenError> {
        let normalized = validate_item(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_item_references(&transaction, regimen_id, &normalized.input)?;
        repository::insert_item(&transaction, &normalized)?;
        transaction.commit()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn update_item(
        &self,
        regimen_id: i64,
        item_id: i64,
        input: RegimenItemInput,
    ) -> Result<RegimenDetail, RegimenError> {
        let normalized = validate_item(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !repository::item_belongs_to_regimen(&transaction, regimen_id, item_id)? {
            return Err(RegimenError::ItemNotFound);
        }
        validate_item_references(&transaction, regimen_id, &normalized.input)?;
        repository::update_item(&transaction, item_id, &normalized)?;
        transaction.commit()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn delete_item(
        &self,
        regimen_id: i64,
        item_id: i64,
    ) -> Result<RegimenDetail, RegimenError> {
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !repository::item_belongs_to_regimen(&transaction, regimen_id, item_id)? {
            return Err(RegimenError::ItemNotFound);
        }
        repository::delete_item(&transaction, item_id)?;
        transaction.commit()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn reorder_items(
        &self,
        regimen_id: i64,
        mut input: RegimenReorderInput,
    ) -> Result<RegimenDetail, RegimenError> {
        input.item_group = clean_optional(input.item_group);
        if input.item_ids.is_empty() {
            return Err(validation(
                "itemIds",
                "At least one regimen item is required",
            ));
        }
        let unique: HashSet<_> = input.item_ids.iter().copied().collect();
        if unique.len() != input.item_ids.len() {
            return Err(validation(
                "itemIds",
                "Regimen item order contains duplicates",
            ));
        }

        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_group(&transaction, regimen_id, input.regimen_group_id)?;
        let current = repository::reorder_candidates(
            &transaction,
            input.regimen_group_id,
            input.item_group.as_deref(),
        )?;
        if current.len() != input.item_ids.len()
            || current.iter().any(|item_id| !unique.contains(item_id))
        {
            return Err(validation(
                "itemIds",
                "Regimen item order does not match the selected legacy group",
            ));
        }
        for (index, item_id) in input.item_ids.iter().enumerate() {
            repository::set_item_order(&transaction, *item_id, index as i64 + 1)?;
        }
        transaction.commit()?;
        repository::get_regimen(&connection, regimen_id)?.ok_or(RegimenError::RegimenNotFound)
    }

    pub(crate) fn lookups(&self) -> Result<RegimenLookups, RegimenError> {
        let connection = self.database.open()?;
        Ok(repository::lookups(&connection)?)
    }
}

fn validate_header(mut input: RegimenInput) -> Result<RegimenInput, RegimenError> {
    input.code = input.code.trim().to_owned();
    input.name = input.name.trim().to_owned();
    if input.code.is_empty() {
        return Err(validation("code", "Regimen code is required"));
    }
    if input.name.is_empty() {
        return Err(validation("name", "Regimen name is required"));
    }
    validate_length("code", "Regimen code", &input.code, 64)?;
    validate_length("name", "Regimen name", &input.name, 255)?;
    Ok(input)
}

fn validate_group(mut input: RegimenGroupInput) -> Result<RegimenGroupInput, RegimenError> {
    input.note = clean_optional(input.note);
    if let Some(note) = input.note.as_deref() {
        validate_length("note", "Treatment-group note", note, 255)?;
    }
    validate_non_negative_integer("cycleDay", "Cycle day", input.cycle_day)?;
    validate_non_negative_integer("cycleCount", "Cycle count", input.cycle_count)?;
    Ok(input)
}

fn validate_item(mut input: RegimenItemInput) -> Result<NormalizedRegimenItemInput, RegimenError> {
    input.dose_text = clean_optional(input.dose_text);
    input.unit_text = clean_optional(input.unit_text);
    input.route_text = clean_optional(input.route_text);
    input.details = clean_optional(input.details);
    input.item_group = clean_optional(input.item_group);
    input.default_rate = clean_optional(input.default_rate);
    if input.regimen_group_id == 0 {
        return Err(validation("regimenGroupId", "Select a treatment group"));
    }
    if input.drug_id == 0 {
        return Err(validation("drugId", "Drug is required"));
    }
    for (field, label, value, maximum) in [
        (
            "doseText",
            "Dose expression",
            input.dose_text.as_deref(),
            64,
        ),
        ("unitText", "Unit", input.unit_text.as_deref(), 64),
        ("routeText", "Route text", input.route_text.as_deref(), 100),
        ("details", "Details", input.details.as_deref(), 500),
        ("itemGroup", "Legacy group", input.item_group.as_deref(), 2),
        (
            "defaultRate",
            "Default rate",
            input.default_rate.as_deref(),
            100,
        ),
    ] {
        if let Some(value) = value {
            validate_length(field, label, value, maximum)?;
        }
    }
    validate_non_negative_integer("duration", "Duration", input.duration)?;
    validate_non_negative_integer("startDay", "Start day", input.start_day)?;
    validate_non_negative_integer("orderingNo", "Ordering", input.ordering_no)?;

    let parsed_dose = input
        .dose_text
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok());
    if parsed_dose.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return Err(validation(
            "doseText",
            "Numeric dose values must be finite and zero or greater",
        ));
    }
    Ok(NormalizedRegimenItemInput { input, parsed_dose })
}

fn validate_item_references(
    transaction: &rusqlite::Transaction<'_>,
    regimen_id: i64,
    input: &RegimenItemInput,
) -> Result<(), RegimenError> {
    ensure_group(transaction, regimen_id, input.regimen_group_id)?;
    for (field, table, id, message) in [
        (
            "drugId",
            "drugs",
            Some(input.drug_id),
            "Selected drug does not exist",
        ),
        (
            "defaultDiluentId",
            "diluents",
            input.default_diluent_id,
            "Selected diluent does not exist",
        ),
        (
            "defaultRouteId",
            "routes",
            input.default_route_id,
            "Selected route does not exist",
        ),
    ] {
        if let Some(id) = id {
            if !repository::lookup_exists(transaction, table, id)? {
                return Err(validation(field, message));
            }
        }
    }
    Ok(())
}

fn ensure_group(
    transaction: &rusqlite::Transaction<'_>,
    regimen_id: i64,
    group_id: i64,
) -> Result<(), RegimenError> {
    if !repository::group_belongs_to_regimen(transaction, regimen_id, group_id)? {
        return Err(RegimenError::GroupNotFound);
    }
    Ok(())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

fn validate_length(
    field: &'static str,
    label: &str,
    value: &str,
    maximum: usize,
) -> Result<(), RegimenError> {
    if value.chars().count() > maximum {
        return Err(validation(
            field,
            format!("{label} must be {maximum} characters or fewer"),
        ));
    }
    Ok(())
}

fn validate_non_negative_integer(
    field: &'static str,
    label: &str,
    value: Option<i64>,
) -> Result<(), RegimenError> {
    if value.is_some_and(|value| value < 0) {
        return Err(validation(field, format!("{label} cannot be negative")));
    }
    Ok(())
}

fn validation(field: &'static str, message: impl Into<String>) -> RegimenError {
    RegimenError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regimen::{RegimenSortField, SortDirection};

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
                    "INSERT INTO units(id,legacy_unitcode,unit_name) VALUES(1,'U','Unit')",
                    [],
                )
                .unwrap();
            connection.execute("INSERT INTO drugs(id,legacy_dcode,drug_name,unit_id) VALUES(1,'D1','Synthetic drug',1)", []).unwrap();
            connection.execute("INSERT INTO drugs(id,legacy_dcode,drug_name,unit_id) VALUES(2,'D2','Second drug',1)", []).unwrap();
            connection.execute("INSERT INTO routes(id,legacy_rcode,route_name) VALUES(1,'R','Synthetic route')", []).unwrap();
            connection.execute("INSERT INTO diluents(id,legacy_dilcode,diluent_name) VALUES(1,'L','Synthetic diluent')", []).unwrap();
            Self {
                _directory: directory,
                database,
            }
        }

        fn service(&self) -> RegimenService<'_> {
            RegimenService::new(&self.database)
        }
    }

    fn header(code: &str, name: &str) -> RegimenInput {
        RegimenInput {
            code: code.to_owned(),
            name: name.to_owned(),
            ..RegimenInput::default()
        }
    }

    fn group(service: &RegimenService<'_>, regimen_id: i64) -> i64 {
        service
            .add_group(regimen_id, RegimenGroupInput::default())
            .unwrap()
            .groups[0]
            .id
    }

    fn item(group_id: i64, drug_id: i64) -> RegimenItemInput {
        RegimenItemInput {
            regimen_group_id: group_id,
            drug_id,
            ..RegimenItemInput::default()
        }
    }

    #[test]
    fn creates_lists_searches_and_updates_regimens_with_thai_text() {
        let fixture = Fixture::new();
        let created = fixture
            .service()
            .create(header("TH-01", "สูตรยาทดสอบ"))
            .unwrap();
        for term in ["TH-01", "ทดสอบ"] {
            let result = fixture
                .service()
                .list(RegimenListRequest {
                    search: Some(term.into()),
                    sort_by: RegimenSortField::Name,
                    sort_direction: SortDirection::Asc,
                    ..RegimenListRequest::default()
                })
                .unwrap();
            assert_eq!(result.total, 1);
            assert_eq!(result.items[0].code, "TH-01");
        }
        let mut changed = header("TH-01", "สูตรยาที่แก้ไข");
        changed.marker = true;
        let updated = fixture.service().update(created.id, changed).unwrap();
        assert_eq!(updated.id, created.id);
        assert!(updated.marker);
    }

    #[test]
    fn rejects_duplicate_codes_case_insensitively() {
        let fixture = Fixture::new();
        fixture.service().create(header("REG-01", "First")).unwrap();
        assert!(matches!(
            fixture.service().create(header(" reg-01 ", "Second")),
            Err(RegimenError::DuplicateCode)
        ));
    }

    #[test]
    fn adds_gets_and_updates_items_with_nullable_lookups_and_raw_dose() {
        let fixture = Fixture::new();
        let regimen = fixture.service().create(header("ITEM", "Items")).unwrap();
        let group_id = group(&fixture.service(), regimen.id);
        let created = fixture
            .service()
            .add_item(
                regimen.id,
                RegimenItemInput {
                    dose_text: Some("100 mg/m2".into()),
                    unit_text: Some(" mg ".into()),
                    ..item(group_id, 1)
                },
            )
            .unwrap();
        let created_item = &created.groups[0].items[0];
        assert_eq!(created_item.dose, None);
        assert_eq!(created_item.dose_text.as_deref(), Some("100 mg/m2"));
        assert!(created_item.default_route_id.is_none());
        let updated = fixture
            .service()
            .update_item(
                regimen.id,
                created_item.id,
                RegimenItemInput {
                    dose_text: Some("125.5".into()),
                    default_route_id: Some(1),
                    default_diluent_id: Some(1),
                    ..item(group_id, 2)
                },
            )
            .unwrap();
        assert_eq!(updated.groups[0].items[0].dose, Some(125.5));
        assert_eq!(updated.groups[0].items[0].drug_id, 2);
    }

    #[test]
    fn rejects_invalid_drug_and_rolls_back_item_creation() {
        let fixture = Fixture::new();
        let regimen = fixture
            .service()
            .create(header("ROLL", "Rollback"))
            .unwrap();
        let group_id = group(&fixture.service(), regimen.id);
        let error = fixture
            .service()
            .add_item(regimen.id, item(group_id, 99_999))
            .unwrap_err();
        assert!(matches!(
            error,
            RegimenError::Validation {
                field: "drugId",
                ..
            }
        ));
        assert!(fixture.service().get(regimen.id).unwrap().groups[0]
            .items
            .is_empty());
    }

    #[test]
    fn removes_items_and_only_removes_empty_groups() {
        let fixture = Fixture::new();
        let regimen = fixture
            .service()
            .create(header("DELETE", "Delete"))
            .unwrap();
        let group_id = group(&fixture.service(), regimen.id);
        let with_item = fixture
            .service()
            .add_item(regimen.id, item(group_id, 1))
            .unwrap();
        let item_id = with_item.groups[0].items[0].id;
        assert!(matches!(
            fixture.service().delete_group(regimen.id, group_id),
            Err(RegimenError::GroupNotEmpty)
        ));
        fixture.service().delete_item(regimen.id, item_id).unwrap();
        let detail = fixture
            .service()
            .delete_group(regimen.id, group_id)
            .unwrap();
        assert!(detail.groups.is_empty());
    }

    #[test]
    fn reorders_items_within_one_legacy_group_transactionally() {
        let fixture = Fixture::new();
        let regimen = fixture.service().create(header("ORDER", "Order")).unwrap();
        let group_id = group(&fixture.service(), regimen.id);
        let first = fixture
            .service()
            .add_item(
                regimen.id,
                RegimenItemInput {
                    item_group: Some("A".into()),
                    ..item(group_id, 1)
                },
            )
            .unwrap()
            .groups[0]
            .items[0]
            .id;
        let detail = fixture
            .service()
            .add_item(
                regimen.id,
                RegimenItemInput {
                    item_group: Some("A".into()),
                    ..item(group_id, 2)
                },
            )
            .unwrap();
        let second = detail.groups[0]
            .items
            .iter()
            .find(|value| value.id != first)
            .unwrap()
            .id;
        let reordered = fixture
            .service()
            .reorder_items(
                regimen.id,
                RegimenReorderInput {
                    regimen_group_id: group_id,
                    item_group: Some("A".into()),
                    item_ids: vec![second, first],
                },
            )
            .unwrap();
        assert_eq!(
            reordered.groups[0]
                .items
                .iter()
                .map(|value| value.id)
                .collect::<Vec<_>>(),
            vec![second, first]
        );
        let error = fixture
            .service()
            .reorder_items(
                regimen.id,
                RegimenReorderInput {
                    regimen_group_id: group_id,
                    item_group: Some("A".into()),
                    item_ids: vec![first, first],
                },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            RegimenError::Validation {
                field: "itemIds",
                ..
            }
        ));
    }

    #[test]
    fn reads_legacy_compatibility_shape_without_changing_ids_or_duplicates() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        connection.execute("INSERT INTO regimens(id,legacy_regcode,regimen_name) VALUES(-1,'LEGACY','Legacy compatibility')", []).unwrap();
        connection.execute("INSERT INTO regimen_groups(id,legacy_code,regimen_id,note) VALUES(20,'20',-1,'Compatibility group')", []).unwrap();
        connection.execute("INSERT INTO regimen_items(id,regimen_group_id,drug_id,legacy_dose_text,item_group,ordering_no) VALUES(30,20,1,'AUC 5','1',1)", []).unwrap();
        connection.execute("INSERT INTO regimen_items(id,regimen_group_id,drug_id,legacy_dose_text,item_group,ordering_no) VALUES(31,20,2,'weekly','1',1)", []).unwrap();
        drop(connection);
        let detail = fixture.service().get_by_code("LEGACY").unwrap();
        assert_eq!(detail.id, -1);
        assert_eq!(detail.groups[0].id, 20);
        assert_eq!(
            detail.groups[0]
                .items
                .iter()
                .map(|value| value.id)
                .collect::<Vec<_>>(),
            vec![30, 31]
        );
    }

    #[test]
    fn returns_all_local_item_lookups() {
        let fixture = Fixture::new();
        let lookups = fixture.service().lookups().unwrap();
        assert_eq!(lookups.drugs.len(), 2);
        assert_eq!(lookups.routes.len(), 1);
        assert_eq!(lookups.diluents.len(), 1);
    }
}
