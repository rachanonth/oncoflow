use serde::{Deserialize, Serialize};

use crate::inventory::StockState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiluentUsageReportRequest {
    pub preparation_date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiluentUsageRow {
    pub diluent_name: Option<String>,
    pub volume_ml: Option<f64>,
    pub bottle_count: i64,
    pub dose_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiluentUsageReport {
    pub preparation_date: String,
    pub rows: Vec<DiluentUsageRow>,
    pub total_bottles: i64,
    pub total_doses: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WardDeliveryReportRequest {
    pub preparation_date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WardDeliveryRow {
    pub task_id: i64,
    pub ward_id: Option<i64>,
    pub ward_name: String,
    pub patient_id: i64,
    pub patient_hn: String,
    pub patient_name: String,
    pub drug_name: String,
    pub ordered_dose_text: Option<String>,
    pub dose_unit_text: Option<String>,
    pub diluent_name: Option<String>,
    pub diluent_volume_ml: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WardDeliveryReport {
    pub preparation_date: String,
    pub rows: Vec<WardDeliveryRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReportInterval {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationCountReportRequest {
    pub interval: ReportInterval,
    pub date_from: String,
    pub date_to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationCountReportRow {
    pub cancelled: bool,
    pub period_start: String,
    pub drug_id: i64,
    pub drug_name: String,
    pub preparer_user_id: Option<i64>,
    pub preparer_name: String,
    pub prescription_count: i64,
    pub bottle_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationCountReport {
    pub interval: ReportInterval,
    pub date_from: String,
    pub date_to: String,
    pub total_prescriptions: i64,
    pub total_bottles: i64,
    pub rows: Vec<PreparationCountReportRow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryUsageReportRequest {
    pub interval: ReportInterval,
    pub date_from: String,
    pub date_to: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryUsageReportRow {
    pub cancelled_review_count: i64,
    pub period_start: String,
    pub drug_id: i64,
    pub drug_code: String,
    pub drug_name: String,
    pub source_package: String,
    pub prescription_count: i64,
    pub prepared_bottle_count: i64,
    pub issued_source_container_count: i64,
    pub awaiting_verification_count: i64,
    pub manual_reconciliation_count: i64,
    pub tracking_disabled_count: i64,
    pub unrecorded_inventory_count: i64,
    pub current_stock: Option<f64>,
    pub minimum_stock: Option<f64>,
    pub stock_state: StockState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryUsageReport {
    pub interval: ReportInterval,
    pub date_from: String,
    pub date_to: String,
    pub total_prescriptions: i64,
    pub total_prepared_bottles: i64,
    pub total_issued_source_containers: i64,
    pub drug_count: i64,
    pub rows: Vec<InventoryUsageReportRow>,
}
