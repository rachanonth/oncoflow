use std::cmp::Ordering;

use super::{
    decimal::{DecimalParse, LegacyDecimal},
    trace::{input, outcome, step},
    CalculationStatus, ClinicalCalculationResult, EvidenceConfidence,
};

const CALC_RULE_ID: &str = "ANCCal";
const GRADE_RULE_ID: &str = "ANCGrade";

pub(crate) fn anc_cal(
    wbc: Option<&str>,
    neutrophil: Option<&str>,
) -> ClinicalCalculationResult<String> {
    let inputs = vec![input("wbc", wbc), input("neutrophil", neutrophil)];
    let (Some(wbc), Some(neutrophil)) = (wbc, neutrophil) else {
        return anc_unavailable(inputs, "A required CBC component is missing");
    };
    let wbc = match parse_component(wbc, "WBC", &inputs) {
        Ok(value) => value,
        Err(result) => return *result,
    };
    let neutrophil = match parse_component(neutrophil, "Neutrophil", &inputs) {
        Ok(value) => value,
        Err(result) => return *result,
    };

    let Some(wbc_gate) = wbc.round_half_even_i32() else {
        return anc_error(inputs, "VBA logical And conversion of WBC overflowed Long");
    };
    if wbc_gate == 0 || neutrophil.is_zero() {
        return anc_unavailable(
            inputs,
            "Legacy `W And N <> 0` gate is false; output remains an empty String",
        );
    }
    let Some(product) = neutrophil.checked_mul(wbc) else {
        return anc_error(inputs, "Neutrophil × WBC overflowed fixed-point range");
    };
    let Some(anc) = product.divide_by_power_of_ten(2) else {
        return anc_error(inputs, "Division by 100 exceeded fixed-point scale");
    };
    let Some(value) = anc.invariant_string() else {
        return anc_error(inputs, "ANC output formatting overflowed");
    };
    let warnings = if value.starts_with('-') {
        vec!["Negative inputs are preserved because the legacy function has no validation.".into()]
    } else {
        Vec::new()
    };
    outcome(
        CALC_RULE_ID,
        EvidenceConfidence::PartiallyConfirmed,
        CalculationStatus::Calculated,
        Some(value),
        inputs,
        vec![
            step(
                "gate",
                "Apply VBA `W And N <> 0`: WBC midpoint-to-even Long coercion must be nonzero and Neutrophil must be nonzero",
            ),
            step("formula", "ANC = (Neutrophil × WBC) / 100"),
            step("rounding", "No explicit legacy rounding"),
        ],
        warnings,
    )
}

pub(crate) fn anc_grade(anc: Option<&str>) -> ClinicalCalculationResult<String> {
    let inputs = vec![input("anc", anc)];
    let Some(anc) = anc else {
        return outcome(
            GRADE_RULE_ID,
            EvidenceConfidence::Confirmed,
            CalculationStatus::Unavailable,
            None,
            inputs,
            vec![step("missing", "No calculated ANC value was supplied")],
            vec!["No grade was inferred.".into()],
        );
    };
    let anc = match LegacyDecimal::parse_access_subset(anc) {
        DecimalParse::Parsed(value) => value,
        DecimalParse::NotNumeric => {
            return grade_error(inputs, "ANC is nonnumeric and cannot be compared")
        }
        DecimalParse::Unsupported => {
            return outcome(
                GRADE_RULE_ID,
                EvidenceConfidence::Confirmed,
                CalculationStatus::Unsupported,
                None,
                inputs,
                vec![step(
                    "parse",
                    "Locale-dependent Access numeric syntax is outside the supported subset",
                )],
                vec!["No grade was inferred.".into()],
            )
        }
    };
    let (value, detail) = if anc.compare_integer(1500) == Some(Ordering::Greater) {
        ("-", "ANC > 1500")
    } else if matches!(
        anc.compare_integer(1000),
        Some(Ordering::Equal | Ordering::Greater)
    ) {
        ("1", "1000 <= ANC <= 1500")
    } else if matches!(
        anc.compare_integer(500),
        Some(Ordering::Equal | Ordering::Greater)
    ) {
        ("2", "500 <= ANC < 1000")
    } else if matches!(
        anc.compare_integer(100),
        Some(Ordering::Equal | Ordering::Greater)
    ) {
        ("3", "100 <= ANC < 500")
    } else if anc.compare_integer(100) == Some(Ordering::Less) {
        ("4", "ANC < 100")
    } else {
        return grade_error(inputs, "ANC comparison exceeded fixed-point range");
    };
    let warnings = if anc.compare_integer(0) == Some(Ordering::Less) {
        vec!["Negative ANC maps to grade 4 because the legacy source has no validation.".into()]
    } else {
        Vec::new()
    };
    outcome(
        GRADE_RULE_ID,
        EvidenceConfidence::Confirmed,
        CalculationStatus::Calculated,
        Some(value.into()),
        inputs,
        vec![step("classification", detail)],
        warnings,
    )
}

fn parse_component(
    value: &str,
    label: &str,
    inputs: &[super::CalculationInput],
) -> Result<LegacyDecimal, Box<ClinicalCalculationResult<String>>> {
    match LegacyDecimal::parse_access_subset(value) {
        DecimalParse::Parsed(value) => Ok(value),
        DecimalParse::NotNumeric => Err(Box::new(anc_error(
            inputs.to_vec(),
            &format!("{label} is nonnumeric and would cause a legacy type mismatch"),
        ))),
        DecimalParse::Unsupported => Err(Box::new(outcome(
            CALC_RULE_ID,
            EvidenceConfidence::PartiallyConfirmed,
            CalculationStatus::Unsupported,
            None,
            inputs.to_vec(),
            vec![step(
                "parse",
                format!("{label} uses unsupported Access numeric syntax"),
            )],
            vec!["No locale-specific interpretation was guessed.".into()],
        ))),
    }
}

fn anc_unavailable(
    inputs: Vec<super::CalculationInput>,
    detail: &str,
) -> ClinicalCalculationResult<String> {
    outcome(
        CALC_RULE_ID,
        EvidenceConfidence::PartiallyConfirmed,
        CalculationStatus::Unavailable,
        None,
        inputs,
        vec![step("default_string", detail)],
        vec!["The legacy empty String is represented as no calculated value.".into()],
    )
}

fn anc_error(
    inputs: Vec<super::CalculationInput>,
    detail: &str,
) -> ClinicalCalculationResult<String> {
    outcome(
        CALC_RULE_ID,
        EvidenceConfidence::PartiallyConfirmed,
        CalculationStatus::LegacyError,
        None,
        inputs,
        vec![step("legacy_error", detail)],
        vec!["No ANC value was returned.".into()],
    )
}

fn grade_error(
    inputs: Vec<super::CalculationInput>,
    detail: &str,
) -> ClinicalCalculationResult<String> {
    outcome(
        GRADE_RULE_ID,
        EvidenceConfidence::Confirmed,
        CalculationStatus::LegacyError,
        None,
        inputs,
        vec![step("legacy_error", detail)],
        vec!["No ANC grade was returned.".into()],
    )
}
