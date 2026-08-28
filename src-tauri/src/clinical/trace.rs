use super::{
    CalculationInput, CalculationStatus, ClinicalCalculationResult, EvidenceConfidence, TraceStep,
    LEGACY_RULESET,
};

pub(super) fn input(name: &str, value: Option<&str>) -> CalculationInput {
    CalculationInput {
        name: name.to_owned(),
        value: value.map(str::to_owned),
    }
}

pub(super) fn step(step: &str, detail: impl Into<String>) -> TraceStep {
    TraceStep {
        step: step.to_owned(),
        detail: detail.into(),
    }
}

pub(super) fn outcome<T>(
    rule_id: &'static str,
    confidence: EvidenceConfidence,
    status: CalculationStatus,
    value: Option<T>,
    inputs: Vec<CalculationInput>,
    trace: Vec<TraceStep>,
    warnings: Vec<String>,
) -> ClinicalCalculationResult<T> {
    ClinicalCalculationResult {
        value,
        status,
        ruleset: LEGACY_RULESET,
        rule_id,
        confidence,
        inputs,
        trace,
        warnings,
    }
}
