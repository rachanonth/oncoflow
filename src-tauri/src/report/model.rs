use serde::{Deserialize, Serialize};

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
