use std::cmp::Ordering;

use crate::clinical::decimal::{DecimalParse, LegacyDecimal};

use super::model::{evidence, SafetyEvidence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RuleOutcomeStatus {
    Clear,
    Triggered,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuleOutcome {
    pub status: RuleOutcomeStatus,
    pub detail: String,
    pub evidence: Vec<SafetyEvidence>,
}

impl RuleOutcome {
    fn clear(detail: impl Into<String>, evidence: Vec<SafetyEvidence>) -> Self {
        Self {
            status: RuleOutcomeStatus::Clear,
            detail: detail.into(),
            evidence,
        }
    }

    fn triggered(detail: impl Into<String>, evidence: Vec<SafetyEvidence>) -> Self {
        Self {
            status: RuleOutcomeStatus::Triggered,
            detail: detail.into(),
            evidence,
        }
    }

    fn unsupported(detail: impl Into<String>, evidence: Vec<SafetyEvidence>) -> Self {
        Self {
            status: RuleOutcomeStatus::Unsupported,
            detail: detail.into(),
            evidence,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ConcentrationInput<'a> {
    pub enabled: bool,
    pub dose: Option<&'a str>,
    pub dose_per_pack: Option<&'a str>,
    pub volume_per_pack: Option<&'a str>,
    pub diluent_volume: Option<&'a str>,
    pub threshold: Option<&'a str>,
    pub unit: Option<&'a str>,
}

pub(super) fn evaluate_concentration(input: ConcentrationInput<'_>) -> RuleOutcome {
    if !input.enabled {
        return RuleOutcome::clear("Legacy maximum-dilution alert is disabled", Vec::new());
    }
    if !is_milligram_unit(input.unit) {
        return RuleOutcome::unsupported(
            "Configured concentration units cannot be reconciled without conversion",
            vec![evidence("Stored unit", input.unit.unwrap_or("NULL"))],
        );
    }
    let dose = match required_decimal(input.dose, "Order dose") {
        Ok(value) => value,
        Err(outcome) => return outcome,
    };
    let dose_per_pack = match positive_decimal(input.dose_per_pack, "Dose per pack") {
        Ok(value) => value,
        Err(outcome) => return outcome,
    };
    let volume_per_pack = match positive_decimal(input.volume_per_pack, "Volume per pack") {
        Ok(value) => value,
        Err(outcome) => return outcome,
    };
    let diluent_volume = match nonnegative_decimal(input.diluent_volume, "Diluent volume") {
        Ok(value) => value,
        Err(outcome) => return outcome,
    };
    let threshold = match nonnegative_decimal(input.threshold, "Configured maximum") {
        Ok(value) => value,
        Err(outcome) => return outcome,
    };
    if dose.compare_integer(0) == Some(Ordering::Less) {
        return RuleOutcome::unsupported(
            "Negative order dose is outside the supported safety subset",
            vec![evidence("Order dose", input.dose.unwrap_or("NULL"))],
        );
    }

    // dose / (diluent + dose * volume_per_pack / dose_per_pack) > threshold
    // is compared exactly as:
    // dose*dose_per_pack > threshold*(diluent*dose_per_pack + dose*volume_per_pack)
    let Some(numerator) = dose.checked_mul(dose_per_pack) else {
        return overflow_outcome();
    };
    let Some(diluent_component) = diluent_volume.checked_mul(dose_per_pack) else {
        return overflow_outcome();
    };
    let Some(drug_component) = dose.checked_mul(volume_per_pack) else {
        return overflow_outcome();
    };
    let Some(denominator) = diluent_component.checked_add(drug_component) else {
        return overflow_outcome();
    };
    if denominator.is_zero() {
        return RuleOutcome::unsupported(
            "Concentration denominator is zero",
            vec![evidence("Order dose", input.dose.unwrap_or("NULL"))],
        );
    }
    let Some(threshold_side) = threshold.checked_mul(denominator) else {
        return overflow_outcome();
    };
    let Some(comparison) = numerator.compare_decimal(threshold_side) else {
        return overflow_outcome();
    };
    let observed = concentration_display(input).unwrap_or_else(|| "unavailable".into());
    let values = vec![
        evidence("Observed concentration", format!("{observed} mg/mL")),
        evidence(
            "Configured maximum",
            format!("{} mg/mL", input.threshold.unwrap_or("NULL")),
        ),
        evidence("Comparison", "observed > configured maximum"),
    ];
    if comparison == Ordering::Greater {
        RuleOutcome::triggered(
            "Calculated concentration is above the configured legacy maximum",
            values,
        )
    } else {
        RuleOutcome::clear(
            "Calculated concentration is not above the configured legacy maximum",
            values,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CumulativeInput<'a> {
    pub enabled: bool,
    pub compatible_total_dose: Option<&'a str>,
    pub bsa: Option<&'a str>,
    pub threshold: Option<&'a str>,
    pub unit: Option<&'a str>,
}

pub(super) fn evaluate_cumulative(input: CumulativeInput<'_>) -> RuleOutcome {
    if !input.enabled {
        return RuleOutcome::clear("Legacy cumulative alert is disabled", Vec::new());
    }
    if !is_milligram_unit(input.unit) {
        return RuleOutcome::unsupported(
            "Cumulative units cannot be reconciled without conversion",
            vec![evidence("Stored unit", input.unit.unwrap_or("NULL"))],
        );
    }
    let total = match nonnegative_decimal(input.compatible_total_dose, "Compatible dose total") {
        Ok(value) => value,
        Err(outcome) => return outcome,
    };
    let bsa = match positive_decimal(input.bsa, "Stored legacy BSA") {
        Ok(value) => value,
        Err(outcome) => return outcome,
    };
    let threshold = match nonnegative_decimal(input.threshold, "Cumulative threshold") {
        Ok(value) => value,
        Err(outcome) => return outcome,
    };
    let Some(threshold_total) = threshold.checked_mul(bsa) else {
        return overflow_outcome();
    };
    let Some(comparison) = total.compare_decimal(threshold_total) else {
        return overflow_outcome();
    };
    let observed = ratio_display(input.compatible_total_dose, input.bsa)
        .unwrap_or_else(|| "unavailable".into());
    let values = vec![
        evidence(
            "Compatible stored-dose total",
            input.compatible_total_dose.unwrap_or("NULL"),
        ),
        evidence("Stored legacy BSA", input.bsa.unwrap_or("NULL")),
        evidence("Observed normalized total", format!("{observed} mg/m²")),
        evidence(
            "Configured cumulative threshold",
            format!("{} mg/m²", input.threshold.unwrap_or("NULL")),
        ),
        evidence("Comparison", "observed >= configured threshold"),
    ];
    if matches!(comparison, Ordering::Equal | Ordering::Greater) {
        RuleOutcome::triggered(
            "Compatible cumulative exposure is at or above the configured legacy threshold",
            values,
        )
    } else {
        RuleOutcome::clear(
            "Compatible cumulative exposure is below the configured legacy threshold",
            values,
        )
    }
}

pub(super) fn evaluate_dilution_incompatibility(
    incompatibility_code: Option<&str>,
    diluent_display: Option<&str>,
) -> RuleOutcome {
    let Some(code) = incompatibility_code
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return RuleOutcome::clear(
            "No structured incompatibility code is configured",
            Vec::new(),
        );
    };
    let Some(diluent) = diluent_display
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return RuleOutcome::unsupported(
            "A structured incompatibility code exists but no diluent is selected",
            vec![evidence("Drug incompatibility code", code)],
        );
    };
    let category = diluent_category(diluent);
    let code = code.to_ascii_uppercase();
    let triggered = match code.as_str() {
        "D" => matches!(category, "D" | "T"),
        "S" => matches!(category, "S" | "T"),
        "A" => category == "D",
        "B" => category == "S",
        _ => {
            return RuleOutcome::unsupported(
                "The legacy incompatibility code is not part of the recovered rule matrix",
                vec![
                    evidence("Drug incompatibility code", code),
                    evidence("Diluent category", category),
                ],
            )
        }
    };
    let values = vec![
        evidence("Drug incompatibility code", code),
        evidence("Diluent category", category),
        evidence("Diluent classification source", "legacy DilCompat"),
    ];
    if triggered {
        RuleOutcome::triggered(
            "The selected diluent matches the recovered legacy incompatibility matrix",
            values,
        )
    } else {
        RuleOutcome::clear(
            "The selected diluent does not match the recovered legacy incompatibility matrix",
            values,
        )
    }
}

fn required_decimal(value: Option<&str>, label: &str) -> Result<LegacyDecimal, RuleOutcome> {
    let Some(value) = value else {
        return Err(RuleOutcome::unsupported(
            format!("{label} is NULL"),
            vec![evidence(label, "NULL")],
        ));
    };
    match LegacyDecimal::parse_access_subset(value) {
        DecimalParse::Parsed(value) => Ok(value),
        DecimalParse::NotNumeric | DecimalParse::Unsupported => Err(RuleOutcome::unsupported(
            format!("{label} is not a supported invariant decimal"),
            vec![evidence(label, "unsupported")],
        )),
    }
}

fn positive_decimal(value: Option<&str>, label: &str) -> Result<LegacyDecimal, RuleOutcome> {
    let value = required_decimal(value, label)?;
    if value.compare_integer(0) != Some(Ordering::Greater) {
        return Err(RuleOutcome::unsupported(
            format!("{label} must be greater than zero"),
            vec![evidence(label, "zero or negative")],
        ));
    }
    Ok(value)
}

fn nonnegative_decimal(value: Option<&str>, label: &str) -> Result<LegacyDecimal, RuleOutcome> {
    let value = required_decimal(value, label)?;
    if value.compare_integer(0) == Some(Ordering::Less) {
        return Err(RuleOutcome::unsupported(
            format!("{label} cannot be negative"),
            vec![evidence(label, "negative")],
        ));
    }
    Ok(value)
}

fn overflow_outcome() -> RuleOutcome {
    RuleOutcome::unsupported(
        "Fixed-point safety comparison exceeded the supported deterministic range",
        Vec::new(),
    )
}

pub(super) fn is_milligram_unit(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        value
            .trim()
            .trim_end_matches('.')
            .eq_ignore_ascii_case("mg")
    })
}

fn diluent_category(value: &str) -> &'static str {
    let value = value.to_ascii_uppercase();
    let dextrose = value.contains('D');
    let saline = value.contains('S') || value.contains('N');
    match (dextrose, saline) {
        (true, false) => "D",
        (false, true) => "S",
        (true, true) => "T",
        (false, false) => "-",
    }
}

fn concentration_display(input: ConcentrationInput<'_>) -> Option<String> {
    let dose = input.dose?.parse::<f64>().ok()?;
    let dose_per_pack = input.dose_per_pack?.parse::<f64>().ok()?;
    let volume_per_pack = input.volume_per_pack?.parse::<f64>().ok()?;
    let diluent = input.diluent_volume?.parse::<f64>().ok()?;
    let result = dose / (diluent + dose / (dose_per_pack / volume_per_pack));
    finite_display(result)
}

fn ratio_display(numerator: Option<&str>, denominator: Option<&str>) -> Option<String> {
    let numerator = numerator?.parse::<f64>().ok()?;
    let denominator = denominator?.parse::<f64>().ok()?;
    finite_display(numerator / denominator)
}

fn finite_display(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    let value = format!("{value:.6}");
    Some(value.trim_end_matches('0').trim_end_matches('.').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn concentration(threshold: Option<&str>) -> RuleOutcome {
        evaluate_concentration(ConcentrationInput {
            enabled: true,
            dose: Some("100"),
            dose_per_pack: Some("100"),
            volume_per_pack: Some("10"),
            diluent_volume: Some("90"),
            threshold,
            unit: Some("mg."),
        })
    }

    #[test]
    fn concentration_boundaries_are_strictly_greater_than() {
        assert_eq!(
            concentration(Some("0.9999")).status,
            RuleOutcomeStatus::Triggered
        );
        assert_eq!(concentration(Some("1")).status, RuleOutcomeStatus::Clear);
        assert_eq!(
            concentration(Some("1.0001")).status,
            RuleOutcomeStatus::Clear
        );
    }

    #[test]
    fn concentration_handles_zero_null_negative_and_unsupported_units() {
        let mut input = ConcentrationInput {
            enabled: true,
            dose: Some("0"),
            dose_per_pack: Some("100"),
            volume_per_pack: Some("10"),
            diluent_volume: Some("90"),
            threshold: Some("0"),
            unit: Some("mg."),
        };
        assert_eq!(
            evaluate_concentration(input).status,
            RuleOutcomeStatus::Clear
        );
        input.threshold = None;
        assert_eq!(
            evaluate_concentration(input).status,
            RuleOutcomeStatus::Unsupported
        );
        input.threshold = Some("-1");
        assert_eq!(
            evaluate_concentration(input).status,
            RuleOutcomeStatus::Unsupported
        );
        input.threshold = Some("0");
        input.unit = Some("mcg");
        assert_eq!(
            evaluate_concentration(input).status,
            RuleOutcomeStatus::Unsupported
        );
        input.unit = Some("mg.");
        input.dose = None;
        assert_eq!(
            evaluate_concentration(input).status,
            RuleOutcomeStatus::Unsupported
        );
    }

    #[test]
    fn cumulative_boundary_is_inclusive() {
        for (total, expected) in [
            ("199.9999", RuleOutcomeStatus::Clear),
            ("200", RuleOutcomeStatus::Triggered),
            ("200.0001", RuleOutcomeStatus::Triggered),
        ] {
            let result = evaluate_cumulative(CumulativeInput {
                enabled: true,
                compatible_total_dose: Some(total),
                bsa: Some("2"),
                threshold: Some("100"),
                unit: Some("mg."),
            });
            assert_eq!(result.status, expected);
        }
    }

    #[test]
    fn cumulative_rejects_missing_invalid_and_unsupported_inputs() {
        for input in [
            CumulativeInput {
                enabled: true,
                compatible_total_dose: None,
                bsa: Some("2"),
                threshold: Some("100"),
                unit: Some("mg."),
            },
            CumulativeInput {
                enabled: true,
                compatible_total_dose: Some("200"),
                bsa: Some("0"),
                threshold: Some("100"),
                unit: Some("mg."),
            },
            CumulativeInput {
                enabled: true,
                compatible_total_dose: Some("-1"),
                bsa: Some("2"),
                threshold: Some("100"),
                unit: Some("mg."),
            },
            CumulativeInput {
                enabled: true,
                compatible_total_dose: Some("200"),
                bsa: Some("2"),
                threshold: None,
                unit: Some("mg."),
            },
            CumulativeInput {
                enabled: true,
                compatible_total_dose: Some("200"),
                bsa: Some("2"),
                threshold: Some("100"),
                unit: Some("mcg"),
            },
        ] {
            assert_eq!(
                evaluate_cumulative(input).status,
                RuleOutcomeStatus::Unsupported
            );
        }
    }

    #[test]
    fn dilution_matrix_matches_recovered_vba() {
        for (code, diluent, expected) in [
            ("D", "D5W", RuleOutcomeStatus::Triggered),
            ("D", "D5W NSS", RuleOutcomeStatus::Triggered),
            ("D", "NSS", RuleOutcomeStatus::Clear),
            ("S", "NSS", RuleOutcomeStatus::Triggered),
            ("S", "D5W NSS", RuleOutcomeStatus::Triggered),
            ("S", "D5W", RuleOutcomeStatus::Clear),
            ("A", "D5W", RuleOutcomeStatus::Triggered),
            ("A", "D5W NSS", RuleOutcomeStatus::Clear),
            ("B", "NSS", RuleOutcomeStatus::Triggered),
            ("B", "D5W NSS", RuleOutcomeStatus::Clear),
        ] {
            assert_eq!(
                evaluate_dilution_incompatibility(Some(code), Some(diluent)).status,
                expected
            );
        }
    }

    #[test]
    fn dilution_handles_null_and_unknown_configuration() {
        assert_eq!(
            evaluate_dilution_incompatibility(None, None).status,
            RuleOutcomeStatus::Clear
        );
        assert_eq!(
            evaluate_dilution_incompatibility(Some("D"), None).status,
            RuleOutcomeStatus::Unsupported
        );
        assert_eq!(
            evaluate_dilution_incompatibility(Some("X"), Some("NSS")).status,
            RuleOutcomeStatus::Unsupported
        );
    }
}
