use std::collections::HashSet;

use rusqlite::TransactionBehavior;
use serde_json::json;
use thiserror::Error;

use crate::{
    auth::audit,
    db::{Database, DatabaseError},
};

use super::{
    repository, NormalizedOrderItemInput, OrderDetail, OrderInput, OrderItemInput,
    OrderListRequest, OrderListResponse, OrderLookups, OrderNoShowInput, OrderReorderInput,
    OrderRescheduleInput, OrderWeightInput, OrderWorkflowStatus,
};

#[derive(Debug, Error)]
pub(crate) enum OrderError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error("order record was not found")]
    OrderNotFound,
    #[error("order item was not found")]
    ItemNotFound,
    #[error("historical orders are read-only")]
    HistoricalReadOnly,
    #[error("the regimen contains an item without a valid local drug")]
    InvalidRegimenItems,
    #[error("the order workflow transition is not available from its current state")]
    InvalidStatusTransition,
    #[error("preparation has already started for this treatment date")]
    PreparationAlreadyStarted,
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct OrderService<'a> {
    database: &'a Database,
}

impl<'a> OrderService<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub(crate) fn list(
        &self,
        mut request: OrderListRequest,
    ) -> Result<OrderListResponse, OrderError> {
        request.search = clean_optional(request.search);
        request.date_from = clean_optional(request.date_from);
        request.date_to = clean_optional(request.date_to);
        validate_date_range(request.date_from.as_deref(), request.date_to.as_deref())?;
        if request.patient_id.is_some_and(|id| id <= 0) {
            return Err(validation("patientId", "Select a valid patient"));
        }
        let connection = self.database.open()?;
        Ok(repository::list_orders(&connection, &request)?)
    }

    pub(crate) fn list_patient_orders(
        &self,
        patient_id: i64,
    ) -> Result<OrderListResponse, OrderError> {
        let mut connection = self.database.open()?;
        let transaction = connection.transaction()?;
        ensure_lookup(&transaction, "patients", patient_id, "patientId", "patient")?;
        transaction.commit()?;
        Ok(repository::list_orders(
            &connection,
            &OrderListRequest {
                patient_id: Some(patient_id),
                ..OrderListRequest::default()
            },
        )?)
    }

    pub(crate) fn get(&self, order_id: i64) -> Result<OrderDetail, OrderError> {
        let connection = self.database.open()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }

    pub(crate) fn create(&self, input: OrderInput) -> Result<OrderDetail, OrderError> {
        self.create_internal(input, false)
    }

    pub(crate) fn create_from_regimen(&self, input: OrderInput) -> Result<OrderDetail, OrderError> {
        self.create_internal(input, true)
    }

    fn create_internal(
        &self,
        input: OrderInput,
        copy_regimen: bool,
    ) -> Result<OrderDetail, OrderError> {
        let input = validate_header(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_header_lookups(&transaction, &input)?;
        if copy_regimen {
            let regimen_id = input.regimen_id.ok_or_else(|| {
                validation("regimenId", "Select a regimen to initialize drug lines")
            })?;
            if repository::regimen_has_unusable_items(&transaction, regimen_id)? {
                return Err(OrderError::InvalidRegimenItems);
            }
        }
        let order_id = repository::insert_order(&transaction, &input)?;
        if copy_regimen {
            repository::copy_regimen_items(
                &transaction,
                order_id,
                input.regimen_id.expect("validated regimen"),
            )?;
        }
        transaction.commit()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }

    pub(crate) fn update(
        &self,
        order_id: i64,
        input: OrderInput,
    ) -> Result<OrderDetail, OrderError> {
        let input = validate_header(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_editable(&transaction, order_id)?;
        validate_header_lookups(&transaction, &input)?;
        if repository::update_order(&transaction, order_id, &input)? == 0 {
            return Err(OrderError::OrderNotFound);
        }
        transaction.commit()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }

    pub(crate) fn update_weight(
        &self,
        order_id: i64,
        input: OrderWeightInput,
    ) -> Result<OrderDetail, OrderError> {
        if let Some(weight) = input.weight_kg {
            if !weight.is_finite() || weight <= 0.0 || weight > 500.0 {
                return Err(validation(
                    "weightKg",
                    "Weight must be greater than 0 and no more than 500",
                ));
            }
        }
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_editable(&transaction, order_id)?;
        if repository::update_order_weight(&transaction, order_id, input.weight_kg)? == 0 {
            return Err(OrderError::OrderNotFound);
        }
        transaction.commit()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }

    pub(crate) fn add_item(
        &self,
        order_id: i64,
        input: OrderItemInput,
    ) -> Result<OrderDetail, OrderError> {
        let input = validate_item(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_editable(&transaction, order_id)?;
        validate_item_lookups(&transaction, &input.input)?;
        repository::insert_order_item(&transaction, order_id, &input)?;
        transaction.commit()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }

    pub(crate) fn update_item(
        &self,
        order_id: i64,
        item_id: i64,
        input: OrderItemInput,
    ) -> Result<OrderDetail, OrderError> {
        let input = validate_item(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_editable(&transaction, order_id)?;
        validate_item_lookups(&transaction, &input.input)?;
        if repository::update_order_item(&transaction, order_id, item_id, &input)? == 0 {
            return Err(OrderError::ItemNotFound);
        }
        transaction.commit()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }

    pub(crate) fn remove_item(
        &self,
        order_id: i64,
        item_id: i64,
    ) -> Result<OrderDetail, OrderError> {
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_editable(&transaction, order_id)?;
        if repository::delete_order_item(&transaction, order_id, item_id)? == 0 {
            return Err(OrderError::ItemNotFound);
        }
        transaction.commit()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }

    pub(crate) fn reorder_items(
        &self,
        order_id: i64,
        input: OrderReorderInput,
    ) -> Result<OrderDetail, OrderError> {
        if input.item_ids.is_empty() {
            return Err(validation(
                "itemIds",
                "Provide every order item in the new order",
            ));
        }
        let unique = input.item_ids.iter().copied().collect::<HashSet<_>>();
        if unique.len() != input.item_ids.len() || unique.iter().any(|id| *id <= 0) {
            return Err(validation(
                "itemIds",
                "Order item identifiers must be unique",
            ));
        }
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_editable(&transaction, order_id)?;
        let existing = repository::order_item_ids(&transaction, order_id)?;
        if unique != existing.into_iter().collect() {
            return Err(validation(
                "itemIds",
                "Reordering must include every order item once",
            ));
        }
        for (index, item_id) in input.item_ids.iter().enumerate() {
            repository::set_item_order(&transaction, order_id, *item_id, index as i64 + 1)?;
        }
        transaction.commit()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }

    pub(crate) fn lookups(&self) -> Result<OrderLookups, OrderError> {
        let connection = self.database.open()?;
        Ok(repository::get_lookups(&connection)?)
    }

    pub(crate) fn record_no_show(
        &self,
        order_id: i64,
        mut input: OrderNoShowInput,
        actor_user_id: i64,
    ) -> Result<OrderDetail, OrderError> {
        input.scheduled_date = input.scheduled_date.trim().to_owned();
        if !is_valid_iso_date(&input.scheduled_date) {
            return Err(validation(
                "scheduledDate",
                "Use a valid missed appointment date in YYYY-MM-DD format",
            ));
        }
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (editable, status) = repository::load_workflow_status(&transaction, order_id)?
            .ok_or(OrderError::OrderNotFound)?;
        if !editable || status == OrderWorkflowStatus::Legacy {
            return Err(OrderError::HistoricalReadOnly);
        }
        if repository::no_show_event_exists(&transaction, order_id, &input.scheduled_date)? {
            if status != OrderWorkflowStatus::OnHold {
                return Err(OrderError::InvalidStatusTransition);
            }
            transaction.commit()?;
            return repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound);
        }
        if status != OrderWorkflowStatus::Active {
            return Err(OrderError::InvalidStatusTransition);
        }
        if repository::has_material_preparation_on_date(
            &transaction,
            order_id,
            &input.scheduled_date,
        )? {
            return Err(OrderError::PreparationAlreadyStarted);
        }
        let event_id = repository::insert_status_event(
            &transaction,
            repository::NewOrderStatusEvent {
                order_id,
                event_type: "no_show",
                from_status: OrderWorkflowStatus::Active,
                to_status: OrderWorkflowStatus::OnHold,
                effective_date: &input.scheduled_date,
                related_date: None,
                actor_user_id,
            },
        )?;
        if repository::update_workflow_status(
            &transaction,
            order_id,
            OrderWorkflowStatus::OnHold,
            Some("no_show"),
            actor_user_id,
        )? == 0
        {
            return Err(OrderError::OrderNotFound);
        }
        audit::append_event(
            &transaction,
            Some(actor_user_id),
            "order_no_show_recorded",
            "order",
            order_id,
            &json!({
                "order_id": order_id,
                "order_status_event_id": event_id,
                "scheduled_date": input.scheduled_date,
                "workflow_status": "on_hold"
            }),
        )?;
        transaction.commit()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }

    pub(crate) fn reschedule(
        &self,
        order_id: i64,
        mut input: OrderRescheduleInput,
        actor_user_id: i64,
    ) -> Result<OrderDetail, OrderError> {
        input.missed_date = input.missed_date.trim().to_owned();
        input.new_date = input.new_date.trim().to_owned();
        if !is_valid_iso_date(&input.missed_date) {
            return Err(validation(
                "missedDate",
                "Use a valid missed appointment date in YYYY-MM-DD format",
            ));
        }
        if !is_valid_iso_date(&input.new_date) {
            return Err(validation(
                "newDate",
                "Use a valid new appointment date in YYYY-MM-DD format",
            ));
        }
        if input.new_date == input.missed_date {
            return Err(validation(
                "newDate",
                "The new appointment date must differ from the missed date",
            ));
        }
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (editable, status) = repository::load_workflow_status(&transaction, order_id)?
            .ok_or(OrderError::OrderNotFound)?;
        if !editable || status == OrderWorkflowStatus::Legacy {
            return Err(OrderError::HistoricalReadOnly);
        }
        if repository::reschedule_event_exists(
            &transaction,
            order_id,
            &input.missed_date,
            &input.new_date,
        )? {
            if status != OrderWorkflowStatus::Active {
                return Err(OrderError::InvalidStatusTransition);
            }
            transaction.commit()?;
            return repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound);
        }
        if status != OrderWorkflowStatus::OnHold
            || !repository::no_show_event_exists(&transaction, order_id, &input.missed_date)?
        {
            return Err(OrderError::InvalidStatusTransition);
        }
        let event_id = repository::insert_status_event(
            &transaction,
            repository::NewOrderStatusEvent {
                order_id,
                event_type: "rescheduled",
                from_status: OrderWorkflowStatus::OnHold,
                to_status: OrderWorkflowStatus::Active,
                effective_date: &input.new_date,
                related_date: Some(&input.missed_date),
                actor_user_id,
            },
        )?;
        if repository::update_workflow_status(
            &transaction,
            order_id,
            OrderWorkflowStatus::Active,
            Some("rescheduled"),
            actor_user_id,
        )? == 0
        {
            return Err(OrderError::OrderNotFound);
        }
        audit::append_event(
            &transaction,
            Some(actor_user_id),
            "order_rescheduled",
            "order",
            order_id,
            &json!({
                "order_id": order_id,
                "order_status_event_id": event_id,
                "missed_date": input.missed_date,
                "new_date": input.new_date,
                "workflow_status": "active"
            }),
        )?;
        transaction.commit()?;
        repository::get_order(&connection, order_id)?.ok_or(OrderError::OrderNotFound)
    }
}

fn validate_header(mut input: OrderInput) -> Result<OrderInput, OrderError> {
    if input.patient_id <= 0 {
        return Err(validation("patientId", "Select a patient"));
    }
    if input.assigned_preparer_user_id.is_none_or(|id| id <= 0) {
        return Err(validation(
            "assignedPreparerUserId",
            "Select the pharmacist assigned to prepare this order",
        ));
    }
    input.note = clean_optional(input.note);
    input.order_time = clean_optional(input.order_time);
    input.order_type = clean_optional(input.order_type);
    for (field, value) in [
        ("wardId", input.ward_id),
        ("doctorId", input.doctor_id),
        ("regimenId", input.regimen_id),
    ] {
        if value.is_some_and(|id| id <= 0) {
            return Err(validation(field, "Select a valid option"));
        }
    }
    if let Some(value) = input.order_type.as_deref() {
        if value.chars().count() > 2 {
            return Err(validation(
                "orderType",
                "Legacy order type is limited to 2 characters",
            ));
        }
    }
    if let Some(value) = input.order_time.as_deref() {
        if !is_valid_local_datetime(value) {
            return Err(validation(
                "orderTime",
                "Use a valid local date and time in YYYY-MM-DDTHH:MM format",
            ));
        }
    }
    Ok(input)
}

fn validate_item(mut input: OrderItemInput) -> Result<NormalizedOrderItemInput, OrderError> {
    if input.drug_id <= 0 {
        return Err(validation("drugId", "Select a drug"));
    }
    input.start_date = clean_optional(input.start_date);
    input.stop_date = clean_optional(input.stop_date);
    input.dose_text = clean_optional(input.dose_text);
    input.schedule_time = clean_optional(input.schedule_time);
    input.rate = clean_optional(input.rate);
    for (field, value) in [("diluentId", input.diluent_id), ("routeId", input.route_id)] {
        if value.is_some_and(|id| id <= 0) {
            return Err(validation(field, "Select a valid option"));
        }
    }
    for (field, value) in [
        ("startDate", input.start_date.as_deref()),
        ("stopDate", input.stop_date.as_deref()),
    ] {
        if value.is_some_and(|value| !is_valid_iso_date(value)) {
            return Err(validation(field, "Use a valid date in YYYY-MM-DD format"));
        }
    }
    if let (Some(start), Some(stop)) = (input.start_date.as_deref(), input.stop_date.as_deref()) {
        if stop < start {
            return Err(validation(
                "stopDate",
                "Stop date cannot be before start date",
            ));
        }
    }
    if input
        .schedule_time
        .as_deref()
        .is_some_and(|value| !is_valid_time(value))
    {
        return Err(validation(
            "scheduleTime",
            "Use a valid time in HH:MM format",
        ));
    }
    if input
        .number_of_drug
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(validation(
            "numberOfDrug",
            "Legacy quantity must be zero or greater",
        ));
    }
    if input
        .diluent_volume_ml
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(validation(
            "diluentVolumeMl",
            "Diluent volume must be zero or greater",
        ));
    }
    let parsed_dose = input
        .dose_text
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok());
    if parsed_dose.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return Err(validation(
            "doseText",
            "Numeric dose must be zero or greater",
        ));
    }
    Ok(NormalizedOrderItemInput { input, parsed_dose })
}

fn validate_header_lookups(
    transaction: &rusqlite::Transaction<'_>,
    input: &OrderInput,
) -> Result<(), OrderError> {
    ensure_lookup(
        transaction,
        "patients",
        input.patient_id,
        "patientId",
        "patient",
    )?;
    ensure_lookup(
        transaction,
        "preparation_pharmacists",
        input.assigned_preparer_user_id.expect("validated preparer"),
        "assignedPreparerUserId",
        "active preparation pharmacist",
    )?;
    for (table, id, field, label) in [
        ("wards", input.ward_id, "wardId", "ward"),
        ("doctors", input.doctor_id, "doctorId", "doctor"),
        ("regimens", input.regimen_id, "regimenId", "regimen"),
    ] {
        if let Some(id) = id {
            ensure_lookup(transaction, table, id, field, label)?;
        }
    }
    Ok(())
}

fn validate_item_lookups(
    transaction: &rusqlite::Transaction<'_>,
    input: &OrderItemInput,
) -> Result<(), OrderError> {
    ensure_lookup(transaction, "drugs", input.drug_id, "drugId", "drug")?;
    for (table, id, field, label) in [
        ("diluents", input.diluent_id, "diluentId", "diluent"),
        ("routes", input.route_id, "routeId", "route"),
    ] {
        if let Some(id) = id {
            ensure_lookup(transaction, table, id, field, label)?;
        }
    }
    Ok(())
}

fn ensure_lookup(
    transaction: &rusqlite::Transaction<'_>,
    table: &'static str,
    id: i64,
    field: &'static str,
    label: &str,
) -> Result<(), OrderError> {
    if !repository::lookup_exists(transaction, table, id)? {
        return Err(validation(
            field,
            format!("Selected {label} does not exist"),
        ));
    }
    Ok(())
}

fn ensure_editable(
    transaction: &rusqlite::Transaction<'_>,
    order_id: i64,
) -> Result<(), OrderError> {
    match repository::is_editable(transaction, order_id)? {
        None => Err(OrderError::OrderNotFound),
        Some(false) => Err(OrderError::HistoricalReadOnly),
        Some(true) => Ok(()),
    }
}

fn validate_date_range(from: Option<&str>, to: Option<&str>) -> Result<(), OrderError> {
    if from.is_some_and(|value| !is_valid_iso_date(value)) {
        return Err(validation("dateFrom", "Use a valid from date"));
    }
    if to.is_some_and(|value| !is_valid_iso_date(value)) {
        return Err(validation("dateTo", "Use a valid to date"));
    }
    if let (Some(from), Some(to)) = (from, to) {
        if to < from {
            return Err(validation("dateTo", "To date cannot be before from date"));
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

fn is_valid_local_datetime(value: &str) -> bool {
    let normalized = value.replace(' ', "T");
    let Some((date, time)) = normalized.split_once('T') else {
        return false;
    };
    is_valid_iso_date(date) && is_valid_time(time)
}

fn is_valid_time(value: &str) -> bool {
    let parts = value.split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return false;
    }
    let parsed = parts
        .iter()
        .map(|part| part.parse::<u32>())
        .collect::<Result<Vec<_>, _>>();
    let Ok(parts) = parsed else {
        return false;
    };
    parts[0] <= 23 && parts[1] <= 59 && parts.get(2).is_none_or(|second| *second <= 59)
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

fn validation(field: &'static str, message: impl Into<String>) -> OrderError {
    OrderError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::{OrderSortField, SortDirection};

    struct Fixture {
        _directory: tempfile::TempDir,
        database: Database,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
            let connection = database.open().unwrap();
            connection.execute("INSERT INTO users(id,username,display_name,password_hash,role,active,credential_kind,updated_at,user_type) VALUES(1,'synthetic.preparer','เภสัชกรเตรียม','$argon2id$synthetic','user',1,'argon2id',CURRENT_TIMESTAMP,'pharmacist')", []).unwrap();
            connection.execute("INSERT INTO patients(id,legacy_hn,first_name,last_name,weight_kg,height_cm) VALUES(1,'SYN-001','สมชาย','ทดสอบ',70,175)", []).unwrap();
            connection.execute("INSERT INTO patients(id,legacy_hn,first_name,last_name,weight_kg,height_cm) VALUES(2,'SYN-002','Second','Patient',60,160)", []).unwrap();
            connection.execute("INSERT INTO doctors(id,legacy_doccode,doctor_name) VALUES(1,'D','Synthetic doctor')", []).unwrap();
            connection
                .execute(
                    "INSERT INTO wards(id,legacy_wcode,ward_name) VALUES(1,'W','Synthetic ward')",
                    [],
                )
                .unwrap();
            connection.execute("INSERT INTO routes(id,legacy_rcode,route_name) VALUES(1,'R','Synthetic route')", []).unwrap();
            connection.execute("INSERT INTO diluents(id,legacy_dilcode,diluent_name,volume_ml) VALUES(1,'L','Synthetic diluent',100)", []).unwrap();
            connection
                .execute(
                    "INSERT INTO drugs(id,legacy_dcode,drug_name) VALUES(1,'DR1','ยาทดสอบ')",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO drugs(id,legacy_dcode,drug_name) VALUES(2,'DR2','Second drug')",
                    [],
                )
                .unwrap();
            connection.execute("INSERT INTO regimens(id,legacy_regcode,regimen_name) VALUES(1,'RG1','สูตรทดสอบ')", []).unwrap();
            connection.execute("INSERT INTO regimen_groups(id,legacy_code,regimen_id,note) VALUES(1,'G1',1,'Group')", []).unwrap();
            connection.execute("INSERT INTO regimen_items(id,regimen_group_id,drug_id,dose,legacy_dose_text,unit_text,route_text,details,item_group,duration,start_day,ordering_no,default_diluent_id,default_route_id,default_rate) VALUES(1,1,1,NULL,'AUC 5','mg','IV','Raw detail','A','2 days',1,2,1,1,'over 2 hours')", []).unwrap();
            connection.execute("INSERT INTO regimen_items(id,regimen_group_id,drug_id,dose,legacy_dose_text,ordering_no) VALUES(2,1,2,25.5,'25.500',1)", []).unwrap();
            connection.execute("INSERT INTO orders(id,legacy_orderid,patient_id,order_time,oncoflow_created) VALUES(50,'LEG-50',1,'2020-01-02 08:30:00',0)", []).unwrap();
            connection.execute("INSERT INTO order_items(id,order_id,drug_id,dose,ordering_no) VALUES(50,50,1,77.5,1)", []).unwrap();
            Self {
                _directory: directory,
                database,
            }
        }

        fn service(&self) -> OrderService<'_> {
            OrderService::new(&self.database)
        }
    }

    fn header(patient_id: i64) -> OrderInput {
        OrderInput {
            patient_id,
            ward_id: Some(1),
            doctor_id: Some(1),
            regimen_id: Some(1),
            order_time: Some("2026-08-16T09:30".into()),
            note: Some(" บันทึกทดสอบ ".into()),
            assigned_preparer_user_id: Some(1),
            ..OrderInput::default()
        }
    }

    fn item(drug_id: i64) -> OrderItemInput {
        OrderItemInput {
            drug_id,
            dose_text: Some("125.50".into()),
            ..OrderItemInput::default()
        }
    }

    #[test]
    fn lists_patient_orders_and_searches_hn_name_order_and_regimen() {
        let fixture = Fixture::new();
        let created = fixture.service().create(header(1)).unwrap();
        fixture.service().add_item(created.id, item(1)).unwrap();
        fixture.service().add_item(created.id, item(2)).unwrap();
        let patient_orders = fixture.service().list_patient_orders(1).unwrap();
        assert_eq!(patient_orders.total, 2);
        let history_drugs = &patient_orders
            .items
            .iter()
            .find(|order| order.id == created.id)
            .unwrap()
            .drugs;
        assert_eq!(history_drugs[0].drug_name, "ยาทดสอบ");
        assert_eq!(history_drugs[0].dose_text.as_deref(), Some("125.50"));
        assert_eq!(history_drugs[1].drug_name, "Second drug");
        assert_eq!(history_drugs[1].dose_text.as_deref(), Some("125.50"));
        for term in [&created.order_id, "SYN-001", "สมชาย", "สูตรทดสอบ"]
        {
            let result = fixture
                .service()
                .list(OrderListRequest {
                    search: Some(term.to_owned()),
                    sort_by: OrderSortField::Patient,
                    sort_direction: SortDirection::Asc,
                    ..OrderListRequest::default()
                })
                .unwrap();
            assert!(!result.items.is_empty());
        }
    }

    #[test]
    fn gets_historical_order_without_modifying_it() {
        let fixture = Fixture::new();
        let before: String = fixture.database.open().unwrap().query_row(
            "SELECT quote(o.note)||'|'||quote(i.dose)||'|'||quote(i.ordering_no) FROM orders o JOIN order_items i ON i.order_id=o.id WHERE o.id=50",
            [], |row| row.get(0)).unwrap();
        let order = fixture.service().get(50).unwrap();
        assert!(!order.editable);
        assert_eq!(order.items[0].dose, Some(77.5));
        let after: String = fixture.database.open().unwrap().query_row(
            "SELECT quote(o.note)||'|'||quote(i.dose)||'|'||quote(i.ordering_no) FROM orders o JOIN order_items i ON i.order_id=o.id WHERE o.id=50",
            [], |row| row.get(0)).unwrap();
        assert_eq!(before, after);
        assert!(matches!(
            fixture.service().update(50, header(1)),
            Err(OrderError::HistoricalReadOnly)
        ));
    }

    #[test]
    fn creates_and_updates_local_order_header_with_thai_text() {
        let fixture = Fixture::new();
        let created = fixture.service().create(header(1)).unwrap();
        assert!(created.editable);
        assert!(created.order_id.starts_with("OF-"));
        assert_eq!(created.weight_kg, Some(70.0));
        assert_eq!(created.height_cm, Some(175.0));
        let mut changed = header(2);
        changed.note = Some("แก้ไขแล้ว".into());
        changed.regimen_id = None;
        let updated = fixture.service().update(created.id, changed).unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.patient_id, 2);
        assert_eq!(updated.note.as_deref(), Some("แก้ไขแล้ว"));
        assert_eq!(updated.weight_kg, Some(60.0));
        assert_eq!(updated.height_cm, Some(160.0));
    }

    #[test]
    fn order_measurements_are_snapshotted_and_weight_edits_do_not_change_patient_master() {
        let fixture = Fixture::new();
        let created = fixture.service().create(header(1)).unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute(
                "UPDATE patients SET weight_kg=82,height_cm=180 WHERE id=1",
                [],
            )
            .unwrap();

        let unchanged = fixture.service().get(created.id).unwrap();
        assert_eq!(unchanged.weight_kg, Some(70.0));
        assert_eq!(unchanged.height_cm, Some(175.0));

        let updated = fixture
            .service()
            .update_weight(
                created.id,
                OrderWeightInput {
                    weight_kg: Some(72.5),
                },
            )
            .unwrap();
        assert_eq!(updated.weight_kg, Some(72.5));
        assert_eq!(updated.height_cm, Some(175.0));
        assert_eq!(
            fixture
                .database
                .open()
                .unwrap()
                .query_row("SELECT weight_kg FROM patients WHERE id=1", [], |row| row
                    .get::<_, f64>(
                    0
                ))
                .unwrap(),
            82.0,
        );
    }

    #[test]
    fn rejects_invalid_order_weight() {
        let fixture = Fixture::new();
        let created = fixture.service().create(header(1)).unwrap();
        assert!(matches!(
            fixture.service().update_weight(
                created.id,
                OrderWeightInput {
                    weight_kg: Some(0.0),
                },
            ),
            Err(OrderError::Validation {
                field: "weightKg",
                ..
            })
        ));
    }

    #[test]
    fn rejects_invalid_patient_drug_regimen_and_optional_references() {
        let fixture = Fixture::new();
        assert!(matches!(
            fixture.service().create(header(99)),
            Err(OrderError::Validation {
                field: "patientId",
                ..
            })
        ));
        let mut bad_regimen = header(1);
        bad_regimen.regimen_id = Some(99);
        assert!(matches!(
            fixture.service().create(bad_regimen),
            Err(OrderError::Validation {
                field: "regimenId",
                ..
            })
        ));
        let order = fixture.service().create(header(1)).unwrap();
        assert!(matches!(
            fixture.service().add_item(order.id, item(99)),
            Err(OrderError::Validation {
                field: "drugId",
                ..
            })
        ));
        let mut bad_route = item(1);
        bad_route.route_id = Some(99);
        assert!(matches!(
            fixture.service().add_item(order.id, bad_route),
            Err(OrderError::Validation {
                field: "routeId",
                ..
            })
        ));
    }

    #[test]
    fn adds_edits_removes_and_reorders_new_items_with_optional_nulls() {
        let fixture = Fixture::new();
        let order = fixture.service().create(header(1)).unwrap();
        let first = fixture.service().add_item(order.id, item(1)).unwrap();
        assert_eq!(first.items[0].dose_text.as_deref(), Some("125.50"));
        assert_eq!(first.items[0].dose, Some(125.5));
        assert!(first.items[0].route_id.is_none());
        let first_id = first.items[0].id;
        let second = fixture
            .service()
            .add_item(
                order.id,
                OrderItemInput {
                    drug_id: 2,
                    dose_text: Some("AUC 5".into()),
                    route_id: Some(1),
                    diluent_id: Some(1),
                    diluent_volume_ml: Some(250.5),
                    ..OrderItemInput::default()
                },
            )
            .unwrap();
        let second_id = second
            .items
            .iter()
            .find(|value| value.id != first_id)
            .unwrap()
            .id;
        assert_eq!(
            second
                .items
                .iter()
                .find(|value| value.id == second_id)
                .unwrap()
                .diluent_volume_ml,
            Some(250.5),
        );
        let reordered = fixture
            .service()
            .reorder_items(
                order.id,
                OrderReorderInput {
                    item_ids: vec![second_id, first_id],
                },
            )
            .unwrap();
        assert_eq!(
            reordered
                .items
                .iter()
                .map(|value| value.id)
                .collect::<Vec<_>>(),
            vec![second_id, first_id]
        );
        let updated = fixture
            .service()
            .update_item(
                order.id,
                first_id,
                OrderItemInput {
                    drug_id: 1,
                    dose_text: Some("raw dose expression".into()),
                    ..OrderItemInput::default()
                },
            )
            .unwrap();
        let changed = updated
            .items
            .iter()
            .find(|value| value.id == first_id)
            .unwrap();
        assert_eq!(changed.dose, None);
        assert_eq!(changed.dose_text.as_deref(), Some("raw dose expression"));
        let removed = fixture.service().remove_item(order.id, second_id).unwrap();
        assert_eq!(removed.items.len(), 1);
    }

    #[test]
    fn creates_from_regimen_transactionally_and_preserves_raw_configuration() {
        let fixture = Fixture::new();
        let created = fixture.service().create_from_regimen(header(1)).unwrap();
        assert_eq!(created.items.len(), 2);
        assert_eq!(created.items[0].source_regimen_item_id, Some(2));
        assert_eq!(created.items[0].dose_text.as_deref(), Some("25.500"));
        assert_eq!(created.items[1].source_regimen_item_id, Some(1));
        assert_eq!(created.items[1].dose_text.as_deref(), Some("AUC 5"));
        assert_eq!(created.items[1].regimen_duration.as_deref(), Some("2 days"));
        assert_eq!(created.items[1].start_date, None);
        assert_eq!(created.items[1].regimen_start_day, Some(1));
    }

    #[test]
    fn regimen_initialization_rolls_back_header_when_an_item_has_no_drug() {
        let fixture = Fixture::new();
        fixture.database.open().unwrap().execute(
            "INSERT INTO regimen_items(id,regimen_group_id,drug_id,legacy_dose_text) VALUES(3,1,NULL,'ambiguous')", []).unwrap();
        let before: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM orders", [], |row| row.get(0))
            .unwrap();
        assert!(matches!(
            fixture.service().create_from_regimen(header(1)),
            Err(OrderError::InvalidRegimenItems)
        ));
        let after: i64 = fixture
            .database
            .open()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM orders", [], |row| row.get(0))
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn validates_dates_numeric_values_and_reorder_transaction() {
        let fixture = Fixture::new();
        let order = fixture.service().create(header(1)).unwrap();
        let mut bad = item(1);
        bad.start_date = Some("2026-02-30".into());
        assert!(matches!(
            fixture.service().add_item(order.id, bad),
            Err(OrderError::Validation {
                field: "startDate",
                ..
            })
        ));
        let mut bad = item(1);
        bad.number_of_drug = Some(-1.0);
        assert!(matches!(
            fixture.service().add_item(order.id, bad),
            Err(OrderError::Validation {
                field: "numberOfDrug",
                ..
            })
        ));
        let mut bad = item(1);
        bad.diluent_volume_ml = Some(-1.0);
        assert!(matches!(
            fixture.service().add_item(order.id, bad),
            Err(OrderError::Validation {
                field: "diluentVolumeMl",
                ..
            })
        ));
        let current = fixture.service().add_item(order.id, item(1)).unwrap();
        let id = current.items[0].id;
        assert!(matches!(
            fixture.service().reorder_items(
                order.id,
                OrderReorderInput {
                    item_ids: vec![id, id]
                }
            ),
            Err(OrderError::Validation {
                field: "itemIds",
                ..
            })
        ));
        assert_eq!(
            fixture.service().get(order.id).unwrap().items[0].ordering_no,
            Some(1)
        );
    }

    #[test]
    fn returns_all_local_lookups_with_name_only_labels() {
        let fixture = Fixture::new();
        let lookups = fixture.service().lookups().unwrap();
        assert_eq!(lookups.patients[0].label, "Second Patient");
        assert!(lookups.drugs.iter().any(|value| value.label == "ยาทดสอบ"));
        assert_eq!(lookups.routes.len(), 1);
        assert_eq!(lookups.diluents.len(), 1);
        assert_eq!(lookups.diluents[0].volume_ml, Some(100.0));
        assert_eq!(lookups.doctors.len(), 1);
        assert_eq!(lookups.wards.len(), 1);
        assert_eq!(lookups.preparation_pharmacists.len(), 1);
        assert_eq!(lookups.preparation_pharmacists[0].label, "เภสัชกรเตรียม");
    }

    #[test]
    fn requires_an_active_pharmacist_assignment_when_ordering() {
        let fixture = Fixture::new();
        let mut missing = header(1);
        missing.assigned_preparer_user_id = None;
        assert!(matches!(
            fixture.service().create(missing),
            Err(OrderError::Validation {
                field: "assignedPreparerUserId",
                ..
            })
        ));
        let mut unknown = header(1);
        unknown.assigned_preparer_user_id = Some(99_999);
        assert!(matches!(
            fixture.service().create(unknown),
            Err(OrderError::Validation {
                field: "assignedPreparerUserId",
                ..
            })
        ));
    }

    #[test]
    fn safety_review_after_order_updates_does_not_mutate_clinical_values() {
        let fixture = Fixture::new();
        let order = fixture.service().create(header(1)).unwrap();
        let updated = fixture.service().add_item(order.id, item(1)).unwrap();
        let item_id = updated.items[0].id;
        let connection = fixture.database.open().unwrap();
        let before: (Option<f64>, Option<i64>, i64) = connection
            .query_row(
                "SELECT i.dose,o.regimen_id,(SELECT COUNT(*) FROM regimen_items) \
                 FROM orders o JOIN order_items i ON i.order_id=o.id \
                 WHERE o.id=?1 AND i.id=?2",
                [order.id, item_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        let evaluation = crate::safety::evaluate_order(&connection, order.id).unwrap();
        let after: (Option<f64>, Option<i64>, i64) = connection
            .query_row(
                "SELECT i.dose,o.regimen_id,(SELECT COUNT(*) FROM regimen_items) \
                 FROM orders o JOIN order_items i ON i.order_id=o.id \
                 WHERE o.id=?1 AND i.id=?2",
                [order.id, item_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(evaluation.mode, crate::safety::SafetyEvaluationMode::Active);
        assert_eq!(before, after);
    }

    #[test]
    fn failed_safety_review_does_not_poison_a_later_order_transaction() {
        let fixture = Fixture::new();
        let order = fixture.service().create(header(1)).unwrap();
        let with_item = fixture.service().add_item(order.id, item(1)).unwrap();
        let item_id = with_item.items[0].id;
        let connection = fixture.database.open().unwrap();
        connection.execute("DROP TABLE alert_settings", []).unwrap();
        assert!(crate::safety::evaluate_order(&connection, order.id).is_err());
        drop(connection);

        let changed = fixture
            .service()
            .update_item(
                order.id,
                item_id,
                OrderItemInput {
                    drug_id: 1,
                    dose_text: Some("250.00".into()),
                    ..OrderItemInput::default()
                },
            )
            .unwrap();
        assert_eq!(changed.items[0].dose, Some(250.0));
        assert_eq!(changed.regimen_id, Some(1));
    }

    #[test]
    fn no_show_and_reschedule_are_idempotent_audited_and_preserve_order_dates() {
        let fixture = Fixture::new();
        let order = fixture.service().create(header(1)).unwrap();
        let with_item = fixture
            .service()
            .add_item(
                order.id,
                OrderItemInput {
                    drug_id: 1,
                    start_date: Some("2026-08-16".into()),
                    stop_date: Some("2026-08-18".into()),
                    ..OrderItemInput::default()
                },
            )
            .unwrap();
        let item_id = with_item.items[0].id;

        let held = fixture
            .service()
            .record_no_show(
                order.id,
                OrderNoShowInput {
                    scheduled_date: "2026-08-16".into(),
                },
                1,
            )
            .unwrap();
        assert_eq!(held.workflow_status, OrderWorkflowStatus::OnHold);
        assert_eq!(held.workflow_status_reason.as_deref(), Some("no_show"));
        assert_eq!(held.status_events.len(), 1);
        assert_eq!(held.status_events[0].event_type, "no_show");

        fixture
            .service()
            .record_no_show(
                order.id,
                OrderNoShowInput {
                    scheduled_date: "2026-08-16".into(),
                },
                1,
            )
            .unwrap();
        let continued = fixture
            .service()
            .reschedule(
                order.id,
                OrderRescheduleInput {
                    missed_date: "2026-08-16".into(),
                    new_date: "2026-08-20".into(),
                },
                1,
            )
            .unwrap();
        assert_eq!(continued.workflow_status, OrderWorkflowStatus::Active);
        assert_eq!(
            continued.workflow_status_reason.as_deref(),
            Some("rescheduled")
        );
        assert_eq!(continued.status_events.len(), 2);
        fixture
            .service()
            .reschedule(
                order.id,
                OrderRescheduleInput {
                    missed_date: "2026-08-16".into(),
                    new_date: "2026-08-20".into(),
                },
                1,
            )
            .unwrap();

        let connection = fixture.database.open().unwrap();
        let preserved: (String, String, String, i64, i64) = connection
            .query_row(
                "SELECT o.order_time,i.start_date,i.stop_date,
                        (SELECT COUNT(*) FROM order_status_events WHERE order_id=o.id),
                        (SELECT COUNT(*) FROM audit_events
                         WHERE entity_type='order' AND entity_id=CAST(o.id AS TEXT)
                           AND event_type IN ('order_no_show_recorded','order_rescheduled'))
                 FROM orders o JOIN order_items i ON i.id=?1 WHERE o.id=?2",
                [item_id, order.id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            preserved,
            (
                "2026-08-16T09:30".into(),
                "2026-08-16".into(),
                "2026-08-18".into(),
                2,
                2
            )
        );
        assert!(connection
            .execute(
                "UPDATE order_status_events SET effective_date='2026-08-21' WHERE order_id=?1",
                [order.id],
            )
            .is_err());
        assert!(connection
            .execute(
                "DELETE FROM order_status_events WHERE order_id=?1",
                [order.id],
            )
            .is_err());
    }

    #[test]
    fn no_show_rejects_material_preparation_and_audit_failure_rolls_back() {
        let fixture = Fixture::new();
        let order = fixture.service().create(header(1)).unwrap();
        let with_item = fixture.service().add_item(order.id, item(1)).unwrap();
        let item_id = with_item.items[0].id;
        fixture
            .database
            .open()
            .unwrap()
            .execute(
                "INSERT INTO preparation_tasks(
                    source_order_id,source_order_item_id,preparation_date,drug_id,state,
                    prepared_at,prepared_by_user_id
                 ) VALUES(?1,?2,'2026-08-16',1,'prepared',CURRENT_TIMESTAMP,1)",
                [order.id, item_id],
            )
            .unwrap();
        assert!(matches!(
            fixture.service().record_no_show(
                order.id,
                OrderNoShowInput {
                    scheduled_date: "2026-08-16".into()
                },
                1
            ),
            Err(OrderError::PreparationAlreadyStarted)
        ));

        let second = fixture.service().create(header(2)).unwrap();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_no_show_audit_failure
                 BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='order_no_show_recorded'
                 BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().record_no_show(
                second.id,
                OrderNoShowInput {
                    scheduled_date: "2026-08-16".into()
                },
                1
            ),
            Err(OrderError::Sqlite(_))
        ));
        let connection = fixture.database.open().unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT workflow_status FROM orders WHERE id=?1",
                    [second.id],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "active"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM order_status_events WHERE order_id=?1",
                    [second.id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
    }
}
