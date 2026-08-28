use super::{
    trace::{input, outcome, step},
    CalculationStatus, ClinicalCalculationResult, EvidenceConfidence,
};

const RULE_ID: &str = "Platelet";

pub(crate) fn platelet(raw_value: Option<&str>) -> ClinicalCalculationResult<String> {
    let inputs = vec![input("rawValue", raw_value)];
    match raw_value {
        Some(value) => outcome(
            RULE_ID,
            EvidenceConfidence::PartiallyConfirmed,
            CalculationStatus::Calculated,
            Some(value.to_owned()),
            inputs,
            vec![step(
                "passthrough",
                "Return supplied Platelet count real_res unchanged as String",
            )],
            vec![
                "The recovered Platelet function performs no grading or threshold comparison."
                    .into(),
            ],
        ),
        None => outcome(
            RULE_ID,
            EvidenceConfidence::PartiallyConfirmed,
            CalculationStatus::Unavailable,
            None,
            inputs,
            vec![step(
                "missing",
                "No supplied local value; legacy missing record leaves an empty String",
            )],
            vec!["External HN/date CBC lookup is intentionally unsupported.".into()],
        ),
    }
}
