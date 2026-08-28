use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PatientSortField {
    #[default]
    Hn,
    Name,
    LastUpdated,
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
pub struct PatientListRequest {
    pub search: Option<String>,
    pub sort_by: PatientSortField,
    pub sort_direction: SortDirection,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientSummary {
    pub id: i64,
    pub hn: String,
    pub title: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub diagnosis: Option<String>,
    pub regimen: Option<String>,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientListResponse {
    pub items: Vec<PatientSummary>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientDetail {
    pub id: i64,
    pub hn: String,
    pub cancer_no: Option<String>,
    pub title: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub sex: Option<String>,
    pub telephone: Option<String>,
    pub weight_kg: Option<f64>,
    pub height_cm: Option<f64>,
    pub birth_date: Option<String>,
    pub age_years: Option<f64>,
    pub occupation: Option<String>,
    pub address: Option<String>,
    pub diagnosis_id: Option<i64>,
    pub diagnosis: Option<String>,
    pub regimen_id: Option<i64>,
    pub regimen: Option<String>,
    pub stage: Option<String>,
    pub her2: Option<String>,
    pub erpr: Option<String>,
    pub allergy: Option<String>,
    pub patient_history: Option<String>,
    pub counselling: bool,
    pub appointment_card: bool,
    pub treatment_ended: Option<bool>,
    pub treatment_end_date: Option<String>,
    pub record_by: Option<String>,
    pub record_time: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PatientInput {
    pub hn: String,
    pub cancer_no: Option<String>,
    pub title: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub sex: Option<String>,
    pub telephone: Option<String>,
    pub weight_kg: Option<f64>,
    pub height_cm: Option<f64>,
    pub birth_date: Option<String>,
    pub age_years: Option<f64>,
    pub occupation: Option<String>,
    pub address: Option<String>,
    pub diagnosis_id: Option<i64>,
    pub regimen_id: Option<i64>,
    pub stage: Option<String>,
    pub her2: Option<String>,
    pub erpr: Option<String>,
    pub allergy: Option<String>,
    pub patient_history: Option<String>,
    pub counselling: bool,
    pub appointment_card: bool,
    pub treatment_ended: Option<bool>,
    pub treatment_end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupOption {
    pub id: i64,
    pub code: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientFormOptions {
    pub diagnoses: Vec<LookupOption>,
    pub regimens: Vec<LookupOption>,
}
