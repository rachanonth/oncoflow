use super::{
    decimal::{DecimalParse, LegacyDecimal},
    trace::{input, outcome, step},
    CalculationStatus, ClinicalCalculationResult, EvidenceConfidence,
};

const RULE_ID: &str = "FixNumber";

pub(crate) fn fix_number(number: Option<&str>) -> ClinicalCalculationResult<String> {
    let inputs = vec![input("number", number)];
    let Some(number) = number else {
        return unavailable(
            inputs,
            "NULL is not numeric; legacy output is an empty String",
        );
    };
    match LegacyDecimal::parse_access_subset(number) {
        DecimalParse::Parsed(value) => match value.ceil() {
            Some(value) => outcome(
                RULE_ID,
                EvidenceConfidence::Confirmed,
                CalculationStatus::Calculated,
                Some(value.to_string()),
                inputs,
                vec![
                    step("is_numeric", "Input is numeric in the supported invariant subset"),
                    step(
                        "ceiling",
                        "Apply legacy Int comparison: Int(Number)+1 for a fraction, otherwise Int(Number)",
                    ),
                ],
                Vec::new(),
            ),
            None => outcome(
                RULE_ID,
                EvidenceConfidence::Confirmed,
                CalculationStatus::LegacyError,
                None,
                inputs,
                vec![step("overflow", "Ceiling exceeded the checked integer range")],
                vec!["No rounded value was returned.".into()],
            ),
        },
        DecimalParse::NotNumeric => unavailable(
            inputs,
            "IsNumeric is false; legacy function exits with an empty String",
        ),
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
            vec!["No locale-specific interpretation was guessed.".into()],
        ),
    }
}

fn unavailable(
    inputs: Vec<super::CalculationInput>,
    detail: &str,
) -> ClinicalCalculationResult<String> {
    outcome(
        RULE_ID,
        EvidenceConfidence::Confirmed,
        CalculationStatus::Unavailable,
        None,
        inputs,
        vec![step("default_string", detail)],
        vec!["The legacy empty String is represented as no calculated value.".into()],
    )
}
