use serde::Serialize;
use tauri::State;

use crate::{auth::AuthSession, db::Database};

use super::{
    OrderDetail, OrderError, OrderInput, OrderItemInput, OrderListRequest, OrderListResponse,
    OrderLookups, OrderNoShowInput, OrderReorderInput, OrderRescheduleInput, OrderService,
    OrderWeightInput,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<OrderError> for CommandError {
    fn from(error: OrderError) -> Self {
        match error {
            OrderError::Validation { field, message } => Self {
                code: "validation",
                message,
                field: Some(field),
            },
            OrderError::OrderNotFound => Self {
                code: "not_found",
                message: "Order record was not found.".into(),
                field: None,
            },
            OrderError::ItemNotFound => Self {
                code: "not_found",
                message: "Order drug line was not found.".into(),
                field: None,
            },
            OrderError::HistoricalReadOnly => Self {
                code: "historical_read_only",
                message: "Historical migrated orders are read-only.".into(),
                field: None,
            },
            OrderError::InvalidRegimenItems => Self {
                code: "invalid_regimen_items",
                message: "The selected regimen contains a step without a valid local drug.".into(),
                field: Some("regimenId"),
            },
            OrderError::InvalidStatusTransition => Self {
                code: "invalid_status_transition",
                message: "This order status action is no longer available. Reload the order and review its current state.".into(),
                field: None,
            },
            OrderError::PreparationAlreadyStarted => Self {
                code: "preparation_already_started",
                message: "This date already has a prepared or checked item. Use a controlled preparation/inventory correction workflow instead of marking it as no-show.".into(),
                field: None,
            },
            OrderError::Database(_) | OrderError::Sqlite(_) => Self {
                code: "database_error",
                message: "The local order database operation failed.".into(),
                field: None,
            },
        }
    }
}

#[tauri::command]
pub(crate) fn list_orders(
    database: State<'_, Database>,
    request: OrderListRequest,
) -> Result<OrderListResponse, CommandError> {
    OrderService::new(&database)
        .list(request)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_patient_orders(
    database: State<'_, Database>,
    patient_id: i64,
) -> Result<OrderListResponse, CommandError> {
    OrderService::new(&database)
        .list_patient_orders(patient_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_order(
    database: State<'_, Database>,
    order_id: i64,
) -> Result<OrderDetail, CommandError> {
    OrderService::new(&database)
        .get(order_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_order(
    database: State<'_, Database>,
    input: OrderInput,
) -> Result<OrderDetail, CommandError> {
    OrderService::new(&database)
        .create(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn create_order_from_regimen(
    database: State<'_, Database>,
    input: OrderInput,
) -> Result<OrderDetail, CommandError> {
    OrderService::new(&database)
        .create_from_regimen(input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_order(
    database: State<'_, Database>,
    order_id: i64,
    input: OrderInput,
) -> Result<OrderDetail, CommandError> {
    OrderService::new(&database)
        .update(order_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_order_weight(
    database: State<'_, Database>,
    order_id: i64,
    input: OrderWeightInput,
) -> Result<OrderDetail, CommandError> {
    OrderService::new(&database)
        .update_weight(order_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn add_order_item(
    database: State<'_, Database>,
    order_id: i64,
    input: OrderItemInput,
) -> Result<OrderDetail, CommandError> {
    OrderService::new(&database)
        .add_item(order_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_order_item(
    database: State<'_, Database>,
    order_id: i64,
    item_id: i64,
    input: OrderItemInput,
) -> Result<OrderDetail, CommandError> {
    OrderService::new(&database)
        .update_item(order_id, item_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn remove_order_item(
    database: State<'_, Database>,
    order_id: i64,
    item_id: i64,
) -> Result<OrderDetail, CommandError> {
    OrderService::new(&database)
        .remove_item(order_id, item_id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn reorder_order_items(
    database: State<'_, Database>,
    order_id: i64,
    input: OrderReorderInput,
) -> Result<OrderDetail, CommandError> {
    OrderService::new(&database)
        .reorder_items(order_id, input)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_order_lookups(
    database: State<'_, Database>,
) -> Result<OrderLookups, CommandError> {
    OrderService::new(&database).lookups().map_err(Into::into)
}

#[tauri::command]
pub(crate) fn record_order_no_show(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    order_id: i64,
    input: OrderNoShowInput,
) -> Result<OrderDetail, CommandError> {
    let actor = session.require_user().map_err(|_| CommandError {
        code: "authentication_required",
        message: "Sign in before recording an attendance exception.".into(),
        field: None,
    })?;
    OrderService::new(&database)
        .record_no_show(order_id, input, actor.id)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn reschedule_order(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    order_id: i64,
    input: OrderRescheduleInput,
) -> Result<OrderDetail, CommandError> {
    let actor = session.require_user().map_err(|_| CommandError {
        code: "authentication_required",
        message: "Sign in before rescheduling an order.".into(),
        field: None,
    })?;
    OrderService::new(&database)
        .reschedule(order_id, input, actor.id)
        .map_err(Into::into)
}
