use std::cmp::Ordering;

use crate::clinical::decimal::LegacyDecimal;

use super::{
    container::{containers_required, unused_amount},
    model::{
        CalculationQuantity, CalculationWarning, InventoryProjection, InventoryProjectionState,
        LegacyReferenceComparison, LegacyReferenceComparisonStatus, PreparationCalculation,
        PreparationCalculationInput, PreparationCalculationStatus, PresentationResult,
    },
    presentation::{cleaned_label, parse_decimal, units_compatible},
    trace::step,
    PREPARATION_CALC_RULESET, PREPARATION_CALC_RULE_ID,
};

const INVENTORY_UNIT_NOTICE: &str =
    "Read-only legacy package/container units; no physical-unit conversion or stock movement.";

pub(crate) fn calculate_preparation(
    input: PreparationCalculationInput<'_>,
) -> PreparationCalculation {
    let package = cleaned_label(input.package_label);
    let raw_unit = cleaned_label(input.presentation_unit);
    let presentation = PresentationResult {
        amount_per_container: None,
        volume_per_container_ml: cleaned_label(input.volume_per_container_ml),
        container_label: package.clone(),
        raw_package_label: package,
    };
    let mut result = base_result(input, presentation);

    let ordered = match parse_decimal(input.ordered_dose_text) {
        Ok(Some(value)) => value,
        Ok(None) => {
            result.trace.push(step(
                "ordered-dose",
                "Read the authoritative ordered dose.",
                None,
                "CONFIRMED",
            ));
            result.warnings.push(warning(
                "ordered-dose-unavailable",
                "A numeric ordered dose is required before preparation quantity can be calculated.",
            ));
            return result;
        }
        Err(message) => return unsupported(result, "ordered-dose", message),
    };
    if ordered.compare_integer(0) == Some(Ordering::Less) {
        return unsupported(
            result,
            "ordered-dose",
            "Negative ordered doses are unsupported by the confirmed legacy preparation rule.",
        );
    }

    let Some(ordered_unit) = cleaned_label(input.ordered_dose_unit) else {
        return unsupported(
            result,
            "unit-compatibility",
            "The ordered-dose unit is missing, so no unit relationship can be established.",
        );
    };
    let Some(presentation_unit) = raw_unit else {
        return unsupported(
            result,
            "unit-compatibility",
            "The presentation amount unit is missing, so no unit relationship can be established.",
        );
    };
    result.ordered_dose = quantity(ordered, &ordered_unit);

    if !units_compatible(Some(&ordered_unit), Some(&presentation_unit)) {
        result.trace.push(step(
            "unit-compatibility",
            format!(
                "Require exact labels after trimming and ASCII case folding: {ordered_unit} vs {presentation_unit}."
            ),
            Some("incompatible".into()),
            "PARTIALLY_CONFIRMED",
        ));
        result.warnings.push(warning(
            "unit-relationship-unknown",
            "Ordered-dose and presentation units are incompatible; no conversion was guessed.",
        ));
        result.status = PreparationCalculationStatus::Unsupported;
        return result;
    }
    result.trace.push(step(
        "unit-compatibility",
        format!("Exact compatible unit label: {ordered_unit}."),
        Some("compatible".into()),
        "PARTIALLY_CONFIRMED",
    ));

    let amount_per_container = match parse_decimal(input.amount_per_container) {
        Ok(Some(value)) => value,
        Ok(None) => {
            result.warnings.push(warning(
                "presentation-unavailable",
                "Amount per container is not configured.",
            ));
            return result;
        }
        Err(message) => return unsupported(result, "presentation", message),
    };
    if amount_per_container.compare_integer(0) != Some(Ordering::Greater) {
        return unsupported(
            result,
            "presentation",
            "Amount per container must be greater than zero.",
        );
    }
    result.presentation.amount_per_container = quantity(amount_per_container, &presentation_unit);

    let Some(container_count) = containers_required(ordered, amount_per_container) else {
        return unsupported(
            result,
            "container-count",
            "The whole-container calculation overflowed the supported fixed-point range.",
        );
    };
    let container_count_text = container_count.to_string();
    result.containers_required = Some(container_count_text.clone());
    result.trace.push(step(
        "container-count",
        "FixNumber(ordered dose / amount per container): exact upward whole-container rounding.",
        Some(container_count_text.clone()),
        "CONFIRMED",
    ));

    let unused = match unused_amount(ordered, amount_per_container, container_count) {
        Some(value) => value,
        None => {
            return unsupported(
                result,
                "unused-amount",
                "The unused-amount calculation overflowed the supported fixed-point range.",
            )
        }
    };
    result.unused_amount = quantity(unused, &presentation_unit);
    result.trace.push(step(
        "unused-amount",
        "(containers required × amount per container) − ordered dose; not classified as waste or reusable.",
        unused.invariant_string(),
        "CONFIRMED",
    ));

    let package_equivalent = ordered.checked_div_exact_nonnegative(amount_per_container);
    result.legacy_reference.calculated_package_equivalent =
        package_equivalent.and_then(LegacyDecimal::invariant_string);

    let mut fully_calculated = true;
    match parse_decimal(input.volume_per_container_ml) {
        Ok(Some(volume)) if volume.compare_integer(0) != Some(Ordering::Less) => {
            let numerator = ordered.checked_mul(volume);
            let withdrawal = numerator.and_then(|value| {
                value.checked_div_round_half_up_nonnegative(amount_per_container, 1)
            });
            if let Some(withdrawal) = withdrawal {
                let withdrawal_text = withdrawal.invariant_string();
                result.withdrawal_volume_ml = withdrawal_text.clone();
                result.legacy_reference.calculated_solution_volume_ml = withdrawal_text.clone();
                result.trace.push(step(
                    "withdrawal-volume",
                    "ordered dose × volume per container ÷ amount per container; round to 1 decimal place (half up)",
                    withdrawal_text,
                    "CONFIRMED_PRODUCT_RULE",
                ));
            } else {
                fully_calculated = false;
                result.trace.push(step(
                    "withdrawal-volume",
                    "The one-decimal withdrawal calculation exceeds the supported fixed-point range.",
                    None,
                    "CONFIRMED_PRODUCT_RULE",
                ));
                result.warnings.push(warning(
                    "withdrawal-calculation-overflow",
                    "Withdrawal volume could not be calculated within the supported numeric range.",
                ));
            }
        }
        Ok(Some(_)) => {
            fully_calculated = false;
            result.warnings.push(warning(
                "presentation-volume-invalid",
                "A negative volume per container is unsupported.",
            ));
        }
        Ok(None) => {
            fully_calculated = false;
            result.warnings.push(warning(
                "presentation-volume-unavailable",
                "Volume per container is unavailable; container count remains supported.",
            ));
        }
        Err(_) => {
            fully_calculated = false;
            result.warnings.push(warning(
                "presentation-volume-malformed",
                "Volume per container is malformed; container count remains supported.",
            ));
        }
    }

    result.concentration = amount_per_container
        .checked_div_exact_nonnegative(
            parse_decimal(input.volume_per_container_ml)
                .ok()
                .flatten()
                .filter(|value| value.compare_integer(0) == Some(Ordering::Greater))
                .unwrap_or(LegacyDecimal::ZERO),
        )
        .and_then(LegacyDecimal::invariant_string)
        .map(|value| format!("{value} {presentation_unit}/mL"));

    result.inventory_projection = inventory_projection(input, container_count);
    if result.inventory_projection.state == InventoryProjectionState::Shortage {
        result.warnings.push(warning(
            "projected-inventory-shortage",
            "Projected inventory is negative. This advisory shortage does not block preparation.",
        ));
    }
    result.legacy_reference.comparison_status = if result.legacy_reference.stored_quantity.is_some()
    {
        LegacyReferenceComparisonStatus::NotComparable
    } else {
        LegacyReferenceComparisonStatus::FormulaConfirmed
    };
    result.status = if fully_calculated {
        PreparationCalculationStatus::Calculated
    } else {
        PreparationCalculationStatus::PartiallyCalculated
    };
    result
}

fn base_result(
    input: PreparationCalculationInput<'_>,
    presentation: PresentationResult,
) -> PreparationCalculation {
    PreparationCalculation {
        status: PreparationCalculationStatus::Unavailable,
        ruleset_version: PREPARATION_CALC_RULESET,
        rule_id: PREPARATION_CALC_RULE_ID,
        ordered_dose: None,
        presentation,
        concentration: None,
        withdrawal_volume_ml: None,
        containers_required: None,
        unused_amount: None,
        inventory_projection: unresolved_inventory_projection(input),
        legacy_reference: LegacyReferenceComparison {
            stored_quantity: cleaned_label(input.legacy_stored_quantity),
            stored_quantity_semantics: "UNKNOWN (legacy noofdrug)",
            calculated_package_equivalent: None,
            calculated_solution_volume_ml: None,
            comparison_status: LegacyReferenceComparisonStatus::Unavailable,
            notice: "The raw legacy quantity is preserved but not treated as a container or volume without evidence.",
        },
        trace: Vec::new(),
        warnings: Vec::new(),
    }
}

fn unresolved_inventory_projection(input: PreparationCalculationInput<'_>) -> InventoryProjection {
    InventoryProjection {
        tracking_enabled: input.inventory_tracking_enabled,
        current_stock: cleaned_label(input.current_inventory),
        containers_required: None,
        projected_stock: None,
        minimum_stock: cleaned_label(input.minimum_inventory),
        state: if input.inventory_tracking_enabled {
            InventoryProjectionState::Unknown
        } else {
            InventoryProjectionState::Untracked
        },
        unit_notice: INVENTORY_UNIT_NOTICE,
    }
}

fn inventory_projection(
    input: PreparationCalculationInput<'_>,
    required_containers: i128,
) -> InventoryProjection {
    if !input.inventory_tracking_enabled {
        return InventoryProjection {
            tracking_enabled: false,
            current_stock: cleaned_label(input.current_inventory),
            containers_required: None,
            projected_stock: None,
            minimum_stock: cleaned_label(input.minimum_inventory),
            state: InventoryProjectionState::Untracked,
            unit_notice: INVENTORY_UNIT_NOTICE,
        };
    }
    let current = parse_decimal(input.current_inventory).ok().flatten();
    let minimum = parse_decimal(input.minimum_inventory).ok().flatten();
    let Some(current) = current else {
        return InventoryProjection {
            tracking_enabled: true,
            current_stock: cleaned_label(input.current_inventory),
            containers_required: Some(required_containers.to_string()),
            projected_stock: None,
            minimum_stock: cleaned_label(input.minimum_inventory),
            state: InventoryProjectionState::Unknown,
            unit_notice: INVENTORY_UNIT_NOTICE,
        };
    };
    let projected = current.checked_sub(
        LegacyDecimal::parse_access_subset(&required_containers.to_string())
            .into_parsed()
            .unwrap_or(LegacyDecimal::ZERO),
    );
    let state = match projected {
        Some(value) if value.compare_integer(0) == Some(Ordering::Less) => {
            InventoryProjectionState::Shortage
        }
        Some(value) if value.is_zero() => InventoryProjectionState::Out,
        Some(value)
            if minimum.is_some_and(|minimum| {
                value.compare_decimal(minimum).is_some_and(Ordering::is_le)
            }) =>
        {
            InventoryProjectionState::Low
        }
        Some(_) => InventoryProjectionState::Normal,
        None => InventoryProjectionState::Unknown,
    };
    InventoryProjection {
        tracking_enabled: true,
        current_stock: current.invariant_string(),
        containers_required: Some(required_containers.to_string()),
        projected_stock: projected.and_then(LegacyDecimal::invariant_string),
        minimum_stock: minimum.and_then(LegacyDecimal::invariant_string),
        state,
        unit_notice: INVENTORY_UNIT_NOTICE,
    }
}

fn unsupported(
    mut result: PreparationCalculation,
    trace_step: &'static str,
    message: &'static str,
) -> PreparationCalculation {
    result.status = PreparationCalculationStatus::Unsupported;
    result
        .trace
        .push(step(trace_step, message, None, "UNKNOWN_OR_UNSUPPORTED"));
    result.warnings.push(warning("unsupported-input", message));
    result
}

fn quantity(value: LegacyDecimal, unit: &str) -> Option<CalculationQuantity> {
    Some(CalculationQuantity {
        value: value.invariant_string()?,
        unit: unit.to_owned(),
    })
}

fn warning(code: &'static str, message: &'static str) -> CalculationWarning {
    CalculationWarning { code, message }
}

trait DecimalParseExt {
    fn into_parsed(self) -> Option<LegacyDecimal>;
}

impl DecimalParseExt for crate::clinical::decimal::DecimalParse {
    fn into_parsed(self) -> Option<LegacyDecimal> {
        match self {
            crate::clinical::decimal::DecimalParse::Parsed(value) => Some(value),
            _ => None,
        }
    }
}
