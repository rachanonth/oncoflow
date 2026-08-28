use serde::Serialize;

use crate::preparation_calc::{
    LegacyReferenceComparisonStatus, PreparationCalculation, PreparationCalculationStatus,
};

pub(crate) const PREPARATION_ELIGIBILITY_RULE: &str = "legacy-cytotoxic-v8:preparation-marker";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EligibilityStatus {
    Eligible,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EligibilityDecision {
    pub status: EligibilityStatus,
    pub rule_id: &'static str,
    pub reason: &'static str,
}

pub(crate) fn evaluate_eligibility(
    order_editable: bool,
    drug_present: bool,
    legacy_marker: bool,
) -> EligibilityDecision {
    let (status, reason) = if !order_editable {
        (
            EligibilityStatus::Excluded,
            "Historical migrated orders are not converted into OncoFlow preparation tasks.",
        )
    } else if !drug_present {
        (
            EligibilityStatus::Excluded,
            "The order item has no valid local drug reference.",
        )
    } else if !legacy_marker {
        (
            EligibilityStatus::Excluded,
            "The local drug is outside the legacy preparation selector.",
        )
    } else {
        (
            EligibilityStatus::Eligible,
            "The local drug is enabled by the confirmed legacy preparation selector.",
        )
    };
    EligibilityDecision {
        status,
        rule_id: PREPARATION_ELIGIBILITY_RULE,
        reason,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReferenceQuantityStatus {
    Calculated,
    Unavailable,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationReferenceQuantity {
    pub status: ReferenceQuantityStatus,
    pub drug_solution_volume_ml: Option<String>,
    pub package_equivalent: Option<String>,
    pub formula: &'static str,
    pub notice: &'static str,
}

pub(crate) fn reference_quantity(
    calculation: &PreparationCalculation,
) -> PreparationReferenceQuantity {
    const FORMULA: &str = "drug solution volume = ordered dose × volume per pack ÷ dose per pack";
    let has_reference = calculation
        .legacy_reference
        .calculated_solution_volume_ml
        .is_some()
        && calculation
            .legacy_reference
            .calculated_package_equivalent
            .is_some();
    let status = if has_reference {
        ReferenceQuantityStatus::Calculated
    } else if calculation.status == PreparationCalculationStatus::Unsupported
        || calculation.status == PreparationCalculationStatus::PartiallyCalculated
    {
        ReferenceQuantityStatus::Unsupported
    } else {
        ReferenceQuantityStatus::Unavailable
    };
    PreparationReferenceQuantity {
        status,
        drug_solution_volume_ml: calculation
            .legacy_reference
            .calculated_solution_volume_ml
            .clone(),
        package_equivalent: calculation
            .legacy_reference
            .calculated_package_equivalent
            .clone(),
        formula: FORMULA,
        notice: if calculation.legacy_reference.comparison_status
            == LegacyReferenceComparisonStatus::NotComparable
        {
            "Reference only. Legacy noofdrug is preserved separately because its semantics are unknown. No order update or inventory deduction is performed."
        } else {
            "Reference only; no unit conversion, order update, or inventory deduction is performed."
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eligibility_uses_marker_not_a_drug_name() {
        let cytotoxic_named_supportive = evaluate_eligibility(true, true, false);
        let unnamed_protocol_adjunct = evaluate_eligibility(true, true, true);
        assert_eq!(
            cytotoxic_named_supportive.status,
            EligibilityStatus::Excluded
        );
        assert_eq!(unnamed_protocol_adjunct.status, EligibilityStatus::Eligible);
        assert_eq!(
            unnamed_protocol_adjunct.rule_id,
            PREPARATION_ELIGIBILITY_RULE
        );
    }

    #[test]
    fn historical_and_invalid_items_are_excluded_deterministically() {
        let first = evaluate_eligibility(false, true, true);
        let second = evaluate_eligibility(false, true, true);
        assert_eq!(first, second);
        assert_eq!(first.status, EligibilityStatus::Excluded);
        assert_eq!(
            evaluate_eligibility(true, false, true).status,
            EligibilityStatus::Excluded
        );
    }

    #[test]
    fn confirmed_reference_formula_is_non_mutating_display_data() {
        use crate::preparation_calc::{calculate_preparation, PreparationCalculationInput};
        let calculation = calculate_preparation(PreparationCalculationInput {
            ordered_dose_text: Some("75"),
            ordered_dose_unit: Some("mg."),
            amount_per_container: Some("50"),
            presentation_unit: Some("mg."),
            volume_per_container_ml: Some("10"),
            package_label: Some("Amp."),
            legacy_stored_quantity: None,
            inventory_tracking_enabled: false,
            current_inventory: None,
            minimum_inventory: None,
        });
        let result = reference_quantity(&calculation);
        assert_eq!(result.status, ReferenceQuantityStatus::Calculated);
        assert_eq!(result.package_equivalent.as_deref(), Some("1.5"));
        assert_eq!(result.drug_solution_volume_ml.as_deref(), Some("15"));
        assert!(result.notice.contains("Reference only"));
    }
}
