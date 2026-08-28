use serde::Serialize;

pub(crate) const LEGACY_RULESET: &str = "legacy-cytotoxic-v8";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CalculationStatus {
    Calculated,
    Unavailable,
    Unsupported,
    LegacyError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum EvidenceConfidence {
    Confirmed,
    PartiallyConfirmed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalculationInput {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TraceStep {
    pub step: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClinicalCalculationResult<T> {
    pub value: Option<T>,
    pub status: CalculationStatus,
    pub ruleset: &'static str,
    pub rule_id: &'static str,
    pub confidence: EvidenceConfidence,
    pub inputs: Vec<CalculationInput>,
    pub trace: Vec<TraceStep>,
    pub warnings: Vec<String>,
}
