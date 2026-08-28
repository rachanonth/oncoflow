use super::{
    decimal::{DecimalParse, LegacyDecimal},
    trace::{input, outcome, step},
    CalculationStatus, ClinicalCalculationResult, EvidenceConfidence,
};

const RULE_ID: &str = "LabMinMax";

pub(crate) fn lab_min_max(number: Option<&str>) -> ClinicalCalculationResult<String> {
    let inputs = vec![input("number", number)];
    let Some(number) = number else {
        return fallback(inputs, "NULL is nonnumeric in the recovered branch");
    };
    match LegacyDecimal::parse_access_subset(number) {
        DecimalParse::Parsed(_) => outcome(
            RULE_ID,
            EvidenceConfidence::Confirmed,
            CalculationStatus::Calculated,
            Some(number.to_owned()),
            inputs,
            vec![
                step(
                    "is_numeric",
                    "Input is numeric in the supported invariant subset",
                ),
                step(
                    "return",
                    "Return the supplied Variant text through the String result",
                ),
            ],
            vec!["No min/max comparison is performed by the recovered function.".into()],
        ),
        DecimalParse::NotNumeric => fallback(inputs, "IsNumeric is false; return '-'"),
        DecimalParse::Unsupported => outcome(
            RULE_ID,
            EvidenceConfidence::Confirmed,
            CalculationStatus::Unsupported,
            None,
            inputs,
            vec![step(
                "is_numeric",
                "Locale-dependent Access numeric syntax is outside the supported subset",
            )],
            vec!["The broader purpose of LabMinMax remains unknown.".into()],
        ),
    }
}

fn fallback(
    inputs: Vec<super::CalculationInput>,
    detail: &str,
) -> ClinicalCalculationResult<String> {
    outcome(
        RULE_ID,
        EvidenceConfidence::Confirmed,
        CalculationStatus::Calculated,
        Some("-".into()),
        inputs,
        vec![step("fallback", detail)],
        vec!["No laboratory range or clinical meaning was inferred.".into()],
    )
}
