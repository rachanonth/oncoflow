use serde::Serialize;

use crate::clinical::LEGACY_RULESET;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SafetySeverity {
    Info,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SafetyFindingStatus {
    Triggered,
    Advisory,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SafetyEvaluationMode {
    Active,
    HistoricalNotEvaluated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CumulativeDoseSummary {
    pub drug_id: i64,
    pub drug_name: String,
    pub total_dose: Option<String>,
    pub threshold: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SafetyEvidence {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SafetyFinding {
    pub id: String,
    pub fingerprint: String,
    pub rule_id: &'static str,
    pub ruleset_version: &'static str,
    pub severity: SafetySeverity,
    pub title: String,
    pub message: String,
    pub evidence: Vec<SafetyEvidence>,
    pub source: &'static str,
    pub status: SafetyFindingStatus,
    pub order_item_id: Option<i64>,
    pub acknowledgement_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SafetyEvaluation {
    pub mode: SafetyEvaluationMode,
    pub ruleset_version: &'static str,
    pub findings: Vec<SafetyFinding>,
    pub evaluated_rule_count: usize,
    pub unsupported_rule_count: usize,
    pub notice: String,
}

impl SafetyEvaluation {
    pub(crate) fn active(findings: Vec<SafetyFinding>, evaluated_rule_count: usize) -> Self {
        let unsupported_rule_count = findings
            .iter()
            .filter(|finding| finding.status == SafetyFindingStatus::Unsupported)
            .count();
        Self {
            mode: SafetyEvaluationMode::Active,
            ruleset_version: LEGACY_RULESET,
            findings,
            evaluated_rule_count,
            unsupported_rule_count,
            notice: "Warnings inform pharmacist review and never change saved order values.".into(),
        }
    }

    pub(crate) fn historical() -> Self {
        Self {
            mode: SafetyEvaluationMode::HistoricalNotEvaluated,
            ruleset_version: LEGACY_RULESET,
            findings: Vec::new(),
            evaluated_rule_count: 0,
            unsupported_rule_count: 0,
            notice: "Historical migrated order: no retrospective safety evaluation was run.".into(),
        }
    }
}

pub(super) fn evidence(label: impl Into<String>, value: impl Into<String>) -> SafetyEvidence {
    SafetyEvidence {
        label: label.into(),
        value: value.into(),
    }
}
