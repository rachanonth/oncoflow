use serde::Serialize;

pub(crate) const PREPARATION_LABEL_TEMPLATE_VERSION: &str = "oncoflow-preparation-label-v1";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationLabelData {
    pub snapshot_id: i64,
    pub template_version: String,
    pub generated_at: String,
    pub print_time: String,
    pub expiration_at: Option<String>,
    pub preparation_id: i64,
    pub order_id: i64,
    pub order_reference: String,
    pub patient_identifier: String,
    pub patient_name: Option<String>,
    pub hospital_name: Option<String>,
    pub regimen_name: Option<String>,
    pub treatment_at: Option<String>,
    pub treatment_day: Option<String>,
    pub drug_code: String,
    pub drug_name: String,
    pub ordered_dose_text: Option<String>,
    pub dose_unit_text: Option<String>,
    pub diluent_name: Option<String>,
    pub diluent_volume_ml: Option<f64>,
    pub withdrawal_volume_ml: Option<String>,
    pub final_volume_ml: Option<f64>,
    pub route_name: Option<String>,
    pub infusion_rate_or_duration: Option<String>,
    pub warning_text: Option<String>,
    pub expiry_time_text: Option<String>,
    pub expiry_storage_text: Option<String>,
    pub prepared_by: Option<String>,
    pub prepared_at: Option<String>,
    pub verified_by: Option<String>,
    pub verified_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationSummaryData {
    pub preparation_instructions: Option<String>,
    pub preparation_notes: Option<String>,
    pub storage_reference: Option<String>,
    pub safety_review_status: &'static str,
    pub inventory_posting_status: Option<String>,
    pub inventory_movement_id: Option<i64>,
    pub containers_required: Option<i64>,
    pub inventory_balance_before: Option<f64>,
    pub inventory_balance_after: Option<f64>,
    pub inventory_stock_state: Option<String>,
    pub calculation_ruleset_version: Option<String>,
    pub calculation_rule_id: Option<String>,
    pub presentation_notice: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationOutput {
    pub label: PreparationLabelData,
    pub containers: Vec<PreparationContainerLabelData>,
    pub summary: PreparationSummaryData,
    pub print_request_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationContainerLabelData {
    pub container_index: u32,
}

#[derive(Debug, Clone)]
pub(super) struct OutputSource {
    pub preparation_id: i64,
    pub state: String,
    pub order_id: i64,
    pub order_item_id: i64,
    pub order_reference: String,
    pub patient_identifier: String,
    pub patient_name: Option<String>,
    pub regimen_name: Option<String>,
    pub treatment_at: Option<String>,
    pub treatment_day: Option<String>,
    pub drug_code: String,
    pub drug_name: String,
    pub ordered_dose_text: Option<String>,
    pub dose_unit_text: Option<String>,
    pub diluent_name: Option<String>,
    pub diluent_volume_ml: Option<f64>,
    pub withdrawal_volume_ml: Option<String>,
    pub final_volume_ml: Option<f64>,
    pub route_name: Option<String>,
    pub infusion_rate_or_duration: Option<String>,
    pub preparation_instructions: Option<String>,
    pub preparation_notes: Option<String>,
    pub storage_reference: Option<String>,
    pub prepared_by: Option<String>,
    pub prepared_at: Option<String>,
    pub verified_by: Option<String>,
    pub verified_at: Option<String>,
    pub inventory_posting_status: Option<String>,
    pub inventory_movement_id: Option<i64>,
    pub containers_required: Option<i64>,
    pub inventory_balance_before: Option<f64>,
    pub inventory_balance_after: Option<f64>,
    pub inventory_stock_state: Option<String>,
    pub calculation_ruleset_version: Option<String>,
    pub calculation_rule_id: Option<String>,
    pub final_container_count: u32,
    pub hospital_name: Option<String>,
    pub warning_text: Option<String>,
    pub expiry_time_text: Option<String>,
    pub expiry_storage_text: Option<String>,
}
