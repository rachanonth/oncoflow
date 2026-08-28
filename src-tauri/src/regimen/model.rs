use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RegimenSortField {
    #[default]
    Code,
    Name,
    Items,
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
pub struct RegimenListRequest {
    pub search: Option<String>,
    pub sort_by: RegimenSortField,
    pub sort_direction: SortDirection,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimenSummary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub marker: bool,
    pub group_count: u64,
    pub item_count: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimenListResponse {
    pub items: Vec<RegimenSummary>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimenDetail {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub marker: bool,
    pub flag: bool,
    pub cycle_check: bool,
    pub auto_mode: bool,
    pub drug_alert: bool,
    pub appointment_alert: bool,
    pub counsel_alert: bool,
    pub groups: Vec<RegimenGroupDetail>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimenGroupDetail {
    pub id: i64,
    pub legacy_code: Option<String>,
    pub note: Option<String>,
    pub cycle_day: Option<i64>,
    pub cycle_count: Option<i64>,
    pub items: Vec<RegimenItemDetail>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimenItemDetail {
    pub id: i64,
    pub regimen_group_id: i64,
    pub drug_id: i64,
    pub drug_code: String,
    pub drug_name: String,
    pub dose: Option<f64>,
    pub dose_text: Option<String>,
    pub unit_text: Option<String>,
    pub route_text: Option<String>,
    pub details: Option<String>,
    pub item_group: Option<String>,
    pub duration: Option<String>,
    pub start_day: Option<i64>,
    pub ordering_no: Option<i64>,
    pub default_diluent_id: Option<i64>,
    pub default_diluent: Option<String>,
    pub default_route_id: Option<i64>,
    pub default_route: Option<String>,
    pub default_rate: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegimenInput {
    pub code: String,
    pub name: String,
    pub marker: bool,
    pub flag: bool,
    pub cycle_check: bool,
    pub auto_mode: bool,
    pub drug_alert: bool,
    pub appointment_alert: bool,
    pub counsel_alert: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegimenGroupInput {
    pub note: Option<String>,
    pub cycle_day: Option<i64>,
    pub cycle_count: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegimenItemInput {
    pub regimen_group_id: i64,
    pub drug_id: i64,
    pub dose_text: Option<String>,
    pub unit_text: Option<String>,
    pub route_text: Option<String>,
    pub details: Option<String>,
    pub item_group: Option<String>,
    pub duration: Option<i64>,
    pub start_day: Option<i64>,
    pub ordering_no: Option<i64>,
    pub default_diluent_id: Option<i64>,
    pub default_route_id: Option<i64>,
    pub default_rate: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct NormalizedRegimenItemInput {
    pub input: RegimenItemInput,
    pub parsed_dose: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegimenReorderInput {
    pub regimen_group_id: i64,
    pub item_group: Option<String>,
    pub item_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimenLookupOption {
    pub id: i64,
    pub code: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimenLookups {
    pub drugs: Vec<RegimenLookupOption>,
    pub routes: Vec<RegimenLookupOption>,
    pub diluents: Vec<RegimenLookupOption>,
}
