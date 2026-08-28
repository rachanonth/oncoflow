use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DrugSortField {
    #[default]
    Code,
    Name,
    Unit,
    Inventory,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DrugListRequest {
    pub search: Option<String>,
    pub inventory_enabled: Option<bool>,
    pub sort_by: DrugSortField,
    pub sort_direction: SortDirection,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugSummary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub unit: Option<String>,
    pub package: Option<String>,
    pub inventory_enabled: bool,
    pub inventory_min: Option<f64>,
    pub inventory_max: Option<f64>,
    pub inventory_quantity: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugListResponse {
    pub items: Vec<DrugSummary>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugDetail {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub unit_id: Option<i64>,
    pub unit: Option<String>,
    pub dose_per_pack: Option<f64>,
    pub volume_per_pack_ml: Option<f64>,
    pub package: Option<String>,
    pub detail: Option<String>,
    pub price: Option<f64>,
    pub theory: Option<String>,
    pub marker: bool,
    pub default_diluent_id: Option<i64>,
    pub default_diluent: Option<String>,
    pub default_route_id: Option<i64>,
    pub default_route: Option<String>,
    pub default_rate: Option<String>,
    pub warning: Option<String>,
    pub storage: Option<String>,
    pub flag: bool,
    pub expiry_time: Option<String>,
    pub expiry_storage: Option<String>,
    pub max_dose: Option<f64>,
    pub max_dilution_alert: Option<bool>,
    pub max_dilution_hard: Option<f64>,
    pub cumulative_alert: Option<bool>,
    pub cumulative_alert_hard: Option<f64>,
    pub dilution_incompatibility: Option<String>,
    pub inventory_cut: Option<bool>,
    pub inventory_min: Option<f64>,
    pub inventory_max: Option<f64>,
    pub inventory_quantity: Option<f64>,
    pub inventory_enabled: bool,
    pub legacy_mapping_code: Option<String>,
    pub legacy_exp: Option<i64>,
    pub legacy_reg: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DrugInput {
    pub code: String,
    pub name: String,
    pub unit_id: Option<i64>,
    pub dose_per_pack: Option<f64>,
    pub volume_per_pack_ml: Option<f64>,
    pub package: Option<String>,
    pub detail: Option<String>,
    pub price: Option<f64>,
    pub theory: Option<String>,
    pub marker: bool,
    pub default_diluent_id: Option<i64>,
    pub default_route_id: Option<i64>,
    pub default_rate: Option<String>,
    pub warning: Option<String>,
    pub storage: Option<String>,
    pub flag: bool,
    pub expiry_time: Option<String>,
    pub expiry_storage: Option<String>,
    pub max_dose: Option<f64>,
    pub max_dilution_alert: Option<bool>,
    pub max_dilution_hard: Option<f64>,
    pub cumulative_alert: Option<bool>,
    pub cumulative_alert_hard: Option<f64>,
    pub dilution_incompatibility: Option<String>,
    pub inventory_cut: Option<bool>,
    pub inventory_min: Option<f64>,
    pub inventory_max: Option<f64>,
    pub inventory_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugLookupOption {
    pub id: i64,
    pub code: Option<String>,
    pub label: String,
    pub volume_ml: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrugFormOptions {
    pub suggested_code: String,
    pub units: Vec<DrugLookupOption>,
    pub routes: Vec<DrugLookupOption>,
    pub diluents: Vec<DrugLookupOption>,
}
