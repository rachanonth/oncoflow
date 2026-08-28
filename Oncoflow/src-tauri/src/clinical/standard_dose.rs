use std::cmp::Ordering;

use super::{
    decimal::{is_locale_sensitive_numeric_form, DecimalParse, LegacyDecimal},
    trace::{input, outcome, step},
    CalculationStatus, ClinicalCalculationResult, EvidenceConfidence,
};

const RULE_ID: &str = "StandardDose";

pub(crate) fn standard_dose(
    dose: Option<&str>,
    surface: Option<&str>,
) -> ClinicalCalculationResult<String> {
    let inputs = vec![input("dose", dose), input("surface", surface)];
    let Some(surface_text) = surface else {
        return outcome(
            RULE_ID,
            EvidenceConfidence::Unknown,
            CalculationStatus::Unsupported,
            None,
            inputs,
            vec![step(
                "surface",
                "NULL Surface Variant coercion was not reference-verified",
            )],
            vec!["No surface value was calculated or inferred.".into()],
        );
    };
    let surface = match LegacyDecimal::parse_access_subset(surface_text) {
        DecimalParse::Parsed(value) => value,
        DecimalParse::Unsupported => {
            return unsupported_surface(inputs, "Surface uses unsupported Access numeric syntax")
        }
        DecimalParse::NotNumeric => {
            return legacy_error(inputs, "Surface would cause a VBA numeric type mismatch")
        }
    };
    let Some(dose_text) = dose else {
        return outcome(
            RULE_ID,
            EvidenceConfidence::Unknown,
            CalculationStatus::Unsupported,
            None,
            inputs,
            vec![step(
                "dose",
                "NULL Number Variant coercion was not reference-verified",
            )],
            vec!["No dose was calculated or inferred.".into()],
        );
    };

    match LegacyDecimal::parse_access_subset(dose_text) {
        DecimalParse::Parsed(number) => numeric_branch(number, surface, inputs),
        DecimalParse::Unsupported if is_locale_sensitive_numeric_form(dose_text) => outcome(
            RULE_ID,
            EvidenceConfidence::PartiallyConfirmed,
            CalculationStatus::Unsupported,
            None,
            inputs,
            vec![step(
                "is_numeric",
                "Locale-dependent Access IsNumeric syntax is outside the supported subset",
            )],
            vec!["The raw dose expression was not reinterpreted.".into()],
        ),
        DecimalParse::NotNumeric | DecimalParse::Unsupported => {
            range_branch(dose_text, surface, inputs)
        }
    }
}

fn numeric_branch(
    number: LegacyDecimal,
    surface: LegacyDecimal,
    inputs: Vec<super::CalculationInput>,
) -> ClinicalCalculationResult<String> {
    let Some(product) = surface.checked_mul(number) else {
        return legacy_error(
            inputs,
            "Surface × dose exceeded the fixed-point compatibility range",
        );
    };
    let mut trace = vec![
        step(
            "is_numeric",
            "VBA IsNumeric(Number) selected the numeric branch",
        ),
        step(
            "multiply",
            format!(
                "Surface × Number = {}",
                product
                    .invariant_string()
                    .unwrap_or_else(|| "overflow".into())
            ),
        ),
    ];
    let value = match product.compare_integer(10) {
        Some(Ordering::Less) => {
            trace.push(step("threshold", "Product < 10; return without VBA Int"));
            product.invariant_string()
        }
        Some(_) => {
            trace.push(step("threshold", "Product >= 10; apply VBA Int (floor)"));
            product.floor().map(|value| value.to_string())
        }
        None => None,
    };
    match value {
        Some(value) => outcome(
            RULE_ID,
            EvidenceConfidence::PartiallyConfirmed,
            CalculationStatus::Calculated,
            Some(value),
            inputs,
            trace,
            Vec::new(),
        ),
        None => legacy_error(
            inputs,
            "Numeric conversion exceeded the compatibility range",
        ),
    }
}

fn range_branch(
    expression: &str,
    surface: LegacyDecimal,
    inputs: Vec<super::CalculationInput>,
) -> ClinicalCalculationResult<String> {
    let characters = expression.chars().collect::<Vec<_>>();
    let widths = match characters.len() {
        3 => Some((1, 1)),
        4 => Some((1, 2)),
        5 => Some((2, 2)),
        6 => Some((2, 3)),
        7 => Some((3, 3)),
        8 => Some((3, 4)),
        9 => Some((4, 4)),
        _ => None,
    };
    let (lower, upper, split_detail) = if let Some((left, right)) = widths {
        let lower_text = characters.iter().take(left).collect::<String>();
        let upper_text = characters
            .iter()
            .skip(characters.len() - right)
            .collect::<String>();
        let lower = match LegacyDecimal::parse_access_subset(&lower_text) {
            DecimalParse::Parsed(value) => value,
            _ => return legacy_error(inputs, "Legacy lower range slice is not numeric"),
        };
        let upper = match LegacyDecimal::parse_access_subset(&upper_text) {
            DecimalParse::Parsed(value) => value,
            _ => return legacy_error(inputs, "Legacy upper range slice is not numeric"),
        };
        (
            lower,
            upper,
            format!(
                "Length {} selected left {} / right {} characters; delimiter is not validated",
                characters.len(),
                left,
                right
            ),
        )
    } else {
        (
            LegacyDecimal::ZERO,
            LegacyDecimal::ZERO,
            format!(
                "Length {} matches no VBA branch; Empty bounds coerce to zero",
                characters.len()
            ),
        )
    };
    let Some(lower_product) = surface.checked_mul(lower) else {
        return legacy_error(inputs, "Lower range multiplication overflowed");
    };
    let Some(upper_product) = surface.checked_mul(upper) else {
        return legacy_error(inputs, "Upper range multiplication overflowed");
    };
    let Some(lower_output) = lower_product.floor() else {
        return legacy_error(inputs, "VBA Int lower conversion overflowed");
    };
    let Some(upper_output) = upper_product.round_half_even_i16() else {
        return legacy_error(
            inputs,
            "Upper range assignment overflowed VBA signed 16-bit Integer",
        );
    };
    outcome(
        RULE_ID,
        EvidenceConfidence::PartiallyConfirmed,
        CalculationStatus::Calculated,
        Some(format!("{lower_output} - {upper_output}")),
        inputs,
        vec![
            step(
                "is_numeric",
                "Nonnumeric Number selected the legacy range branch",
            ),
            step("split", split_detail),
            step("lower", "Surface × lower bound, then VBA Int (floor)"),
            step(
                "upper",
                "Surface × upper bound, then VBA Integer midpoint-to-even coercion",
            ),
        ],
        vec!["Range parsing intentionally preserves the legacy positional behavior.".into()],
    )
}

fn unsupported_surface(
    inputs: Vec<super::CalculationInput>,
    detail: &str,
) -> ClinicalCalculationResult<String> {
    outcome(
        RULE_ID,
        EvidenceConfidence::PartiallyConfirmed,
        CalculationStatus::Unsupported,
        None,
        inputs,
        vec![step("surface", detail)],
        vec!["No locale-dependent numeric interpretation was guessed.".into()],
    )
}

fn legacy_error(
    inputs: Vec<super::CalculationInput>,
    detail: &str,
) -> ClinicalCalculationResult<String> {
    outcome(
        RULE_ID,
        EvidenceConfidence::PartiallyConfirmed,
        CalculationStatus::LegacyError,
        None,
        inputs,
        vec![step("legacy_error", detail)],
        vec!["The legacy function would not produce a safe value for this input.".into()],
    )
}
