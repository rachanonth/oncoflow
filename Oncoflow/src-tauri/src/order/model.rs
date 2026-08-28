use serde::{Deserialize, Serialize};

use crate::safety::CumulativeDoseSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderWorkflowStatus {
    Active,
    OnHold,
    Legacy,
}

impl OrderWorkflowStatus {
    pub(super) const fn as_database(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::OnHold => "on_hold",
            Self::Legacy => "legacy",
        }
    }

    pub(super) fn from_database(value: &str) -> rusqlite::Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "on_hold" => Ok(Self::OnHold),
            "legacy" => Ok(Self::Legacy),
            value => Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                format!("invalid order workflow status: {value}").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderStatusEvent {
    pub id: i64,
    pub event_type: String,
    pub from_status: OrderWorkflowStatus,
    pub to_status: OrderWorkflowStatus,
    pub effective_date: String,
    pub related_date: Option<String>,
    pub actor_display_name: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderNoShowInput {
    pub scheduled_date: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderRescheduleInput {
    pub missed_date: String,
    pub new_date: String,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderSortField {
    #[default]
    Date,
    OrderId,
    Patient,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    #[default]
    Desc,
    Asc,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderListRequest {
    pub search: Option<String>,
    pub patient_id: Option<i64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub sort_by: OrderSortField,
    pub sort_direction: SortDirection,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSummaryDrug {
    pub drug_name: String,
    pub dose_text: Option<String>,
    pub unit_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSummary {
    pub id: i64,
    pub order_id: String,
    pub patient_id: i64,
    pub patient_hn: String,
    pub patient_name: String,
    pub order_time: Option<String>,
    pub regimen_name: Option<String>,
    pub doctor_name: Option<String>,
    pub ward_name: Option<String>,
    pub order_type: Option<String>,
    pub item_count: u64,
    pub drugs: Vec<OrderSummaryDrug>,
    pub editable: bool,
    pub workflow_status: OrderWorkflowStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderListResponse {
    pub items: Vec<OrderSummary>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetail {
    pub id: i64,
    pub order_id: String,
    pub patient_id: i64,
    pub patient_hn: String,
    pub patient_name: String,
    pub weight_kg: Option<f64>,
    pub height_cm: Option<f64>,
    pub assigned_preparer_user_id: Option<i64>,
    pub assigned_preparer_name: Option<String>,
    pub ward_id: Option<i64>,
    pub ward_name: Option<String>,
    pub doctor_id: Option<i64>,
    pub doctor_name: Option<String>,
    pub regimen_id: Option<i64>,
    pub regimen_name: Option<String>,
    pub note: Option<String>,
    pub order_time: Option<String>,
    pub order_type: Option<String>,
    pub appointment_flag: bool,
    pub legacy_worker: Option<String>,
    pub edit_worker: Option<String>,
    pub side_effect_text: Option<String>,
    pub side_effect_recorder: Option<String>,
    pub side_effect_record_time: Option<String>,
    pub medication_error_text: Option<String>,
    pub editable: bool,
    pub workflow_status: OrderWorkflowStatus,
    pub workflow_status_reason: Option<String>,
    pub workflow_status_changed_at: Option<String>,
    pub workflow_status_changed_by: Option<String>,
    pub status_events: Vec<OrderStatusEvent>,
    pub cumulative_doses: Vec<CumulativeDoseSummary>,
    pub items: Vec<OrderItemDetail>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderWeightInput {
    pub weight_kg: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItemDetail {
    pub id: i64,
    pub drug_id: i64,
    pub drug_name: String,
    pub diluent_id: Option<i64>,
    pub diluent_name: Option<String>,
    pub diluent_volume_ml: Option<f64>,
    pub route_id: Option<i64>,
    pub route_name: Option<String>,
    pub start_date: Option<String>,
    pub stop_date: Option<String>,
    pub dose: Option<f64>,
    pub dose_text: Option<String>,
    pub schedule_time: Option<String>,
    pub number_of_drug: Option<f64>,
    pub missing: bool,
    pub printed: bool,
    pub rate: Option<String>,
    pub ordering_no: Option<i64>,
    pub running_no: Option<i64>,
    pub running_sum: Option<i64>,
    pub inventory_date: Option<String>,
    pub source_regimen_item_id: Option<i64>,
    pub regimen_dose_text: Option<String>,
    pub regimen_unit_text: Option<String>,
    pub regimen_route_text: Option<String>,
    pub regimen_details: Option<String>,
    pub regimen_item_group: Option<String>,
    pub regimen_duration: Option<String>,
    pub regimen_start_day: Option<i64>,
    pub regimen_ordering_no: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderInput {
    pub patient_id: i64,
    pub ward_id: Option<i64>,
    pub doctor_id: Option<i64>,
    pub regimen_id: Option<i64>,
    pub note: Option<String>,
    pub order_time: Option<String>,
    pub order_type: Option<String>,
    pub appointment_flag: bool,
    pub assigned_preparer_user_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderItemInput {
    pub drug_id: i64,
    pub diluent_id: Option<i64>,
    pub diluent_volume_ml: Option<f64>,
    pub route_id: Option<i64>,
    pub start_date: Option<String>,
    pub stop_date: Option<String>,
    pub dose_text: Option<String>,
    pub schedule_time: Option<String>,
    pub number_of_drug: Option<f64>,
    pub missing: bool,
    pub rate: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct NormalizedOrderItemInput {
    pub input: OrderItemInput,
    pub parsed_dose: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderReorderInput {
    pub item_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLookupOption {
    pub id: i64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientOrderLookupOption {
    pub id: i64,
    pub hn: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiluentOrderLookupOption {
    pub id: i64,
    pub label: String,
    pub volume_ml: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLookups {
    pub patients: Vec<PatientOrderLookupOption>,
    pub regimens: Vec<OrderLookupOption>,
    pub drugs: Vec<OrderLookupOption>,
    pub routes: Vec<OrderLookupOption>,
    pub diluents: Vec<DiluentOrderLookupOption>,
    pub doctors: Vec<OrderLookupOption>,
    pub wards: Vec<OrderLookupOption>,
    pub preparation_pharmacists: Vec<OrderLookupOption>,
}
