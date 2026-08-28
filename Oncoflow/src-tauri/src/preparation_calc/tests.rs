use super::*;

fn supported_input<'a>() -> PreparationCalculationInput<'a> {
    PreparationCalculationInput {
        ordered_dose_text: Some("75"),
        ordered_dose_unit: Some("mg."),
        amount_per_container: Some("50"),
        presentation_unit: Some("mg."),
        volume_per_container_ml: Some("10"),
        package_label: Some("Amp."),
        legacy_stored_quantity: None,
        inventory_tracking_enabled: true,
        current_inventory: Some("2"),
        minimum_inventory: Some("1"),
    }
}

#[test]
fn reference_fixture_corpus_is_synthetic_and_versioned() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/preparation_calc/legacy_cytotoxic_v8.json"
    ))
    .expect("valid synthetic fixture");
    assert_eq!(fixture["ruleset"], PREPARATION_CALC_RULESET);
    assert!(fixture["cases"]
        .as_array()
        .is_some_and(|cases| cases.len() >= 8));
    assert!(
        !include_str!("../../../tests/fixtures/preparation_calc/legacy_cytotoxic_v8.json")
            .contains("patient")
    );
}

#[test]
fn confirmed_formula_uses_exact_fixed_point_and_explains_result() {
    let result = calculate_preparation(supported_input());
    assert_eq!(result.status, PreparationCalculationStatus::Calculated);
    assert_eq!(
        result.ruleset_version,
        "legacy-cytotoxic-v8+withdrawal-1dp-v1"
    );
    assert_eq!(result.containers_required.as_deref(), Some("2"));
    assert_eq!(result.withdrawal_volume_ml.as_deref(), Some("15"));
    assert_eq!(
        result
            .unused_amount
            .as_ref()
            .map(|value| value.value.as_str()),
        Some("25")
    );
    assert_eq!(result.concentration.as_deref(), Some("5 mg./mL"));
    assert!(result
        .trace
        .iter()
        .any(|step| step.step == "container-count"));
}

#[test]
fn container_rounding_has_explicit_boundaries() {
    for (dose, expected) in [
        ("0", "0"),
        ("99.999", "1"),
        ("100", "1"),
        ("100.0001", "2"),
        ("200", "2"),
    ] {
        let mut input = supported_input();
        input.ordered_dose_text = Some(dose);
        input.amount_per_container = Some("100");
        assert_eq!(
            calculate_preparation(input).containers_required.as_deref(),
            Some(expected),
            "dose {dose}"
        );
    }
}

#[test]
fn decimal_presentations_remain_exact() {
    let mut input = supported_input();
    input.ordered_dose_text = Some("1.25");
    input.amount_per_container = Some("0.5");
    input.volume_per_container_ml = Some("0.2");
    let result = calculate_preparation(input);
    assert_eq!(result.containers_required.as_deref(), Some("3"));
    assert_eq!(result.withdrawal_volume_ml.as_deref(), Some("0.5"));
    assert_eq!(
        result
            .unused_amount
            .as_ref()
            .map(|value| value.value.as_str()),
        Some("0.25")
    );
}

#[test]
fn non_terminating_withdrawal_uses_the_confirmed_one_decimal_rule() {
    let mut input = supported_input();
    input.ordered_dose_text = Some("1");
    input.amount_per_container = Some("3");
    input.volume_per_container_ml = Some("1");
    let result = calculate_preparation(input);
    assert_eq!(result.status, PreparationCalculationStatus::Calculated);
    assert_eq!(result.containers_required.as_deref(), Some("1"));
    assert_eq!(result.withdrawal_volume_ml.as_deref(), Some("0.3"));
    assert!(result.trace.iter().any(
        |step| step.step == "withdrawal-volume" && step.expression.contains("1 decimal place")
    ));
}

#[test]
fn bleomycin_withdrawal_rounds_to_one_decimal_place() {
    let mut input = supported_input();
    input.ordered_dose_text = Some("19");
    input.amount_per_container = Some("15");
    input.volume_per_container_ml = Some("5");
    input.package_label = Some("Vial");
    let result = calculate_preparation(input);
    assert_eq!(result.status, PreparationCalculationStatus::Calculated);
    assert_eq!(result.withdrawal_volume_ml.as_deref(), Some("6.3"));
    assert_eq!(result.containers_required.as_deref(), Some("2"));
    assert_eq!(
        result
            .unused_amount
            .as_ref()
            .map(|value| value.value.as_str()),
        Some("11")
    );
}

#[test]
fn missing_malformed_and_incompatible_units_are_explicit() {
    let mut missing = supported_input();
    missing.amount_per_container = None;
    assert_eq!(
        calculate_preparation(missing).status,
        PreparationCalculationStatus::Unavailable
    );

    let mut malformed = supported_input();
    malformed.ordered_dose_text = Some("1,000");
    assert_eq!(
        calculate_preparation(malformed).status,
        PreparationCalculationStatus::Unsupported
    );

    let mut incompatible = supported_input();
    incompatible.ordered_dose_unit = Some("mg");
    incompatible.presentation_unit = Some("mcg");
    let result = calculate_preparation(incompatible);
    assert_eq!(result.status, PreparationCalculationStatus::Unsupported);
    assert!(result.containers_required.is_none());
}

#[test]
fn unit_labels_are_only_trimmed_and_ascii_case_folded() {
    let mut compatible = supported_input();
    compatible.ordered_dose_unit = Some(" MG. ");
    assert_eq!(
        calculate_preparation(compatible).status,
        PreparationCalculationStatus::Calculated
    );

    let mut punctuation_variant = supported_input();
    punctuation_variant.ordered_dose_unit = Some("mg");
    assert_eq!(
        calculate_preparation(punctuation_variant).status,
        PreparationCalculationStatus::Unsupported
    );
}

#[test]
fn inventory_projection_allows_zero_and_negative_balances() {
    for (current, expected, state) in [
        ("3", "1", InventoryProjectionState::Low),
        ("2", "0", InventoryProjectionState::Out),
        ("1", "-1", InventoryProjectionState::Shortage),
    ] {
        let mut input = supported_input();
        input.current_inventory = Some(current);
        let projection = calculate_preparation(input).inventory_projection;
        assert_eq!(projection.projected_stock.as_deref(), Some(expected));
        assert_eq!(projection.state, state);
    }
}

#[test]
fn untracked_and_unknown_inventory_do_not_affect_calculation() {
    let mut untracked = supported_input();
    untracked.inventory_tracking_enabled = false;
    assert_eq!(
        calculate_preparation(untracked).inventory_projection.state,
        InventoryProjectionState::Untracked
    );

    let mut unknown = supported_input();
    unknown.current_inventory = None;
    let result = calculate_preparation(unknown);
    assert_eq!(result.status, PreparationCalculationStatus::Calculated);
    assert_eq!(
        result.inventory_projection.state,
        InventoryProjectionState::Unknown
    );
}

#[test]
fn raw_legacy_quantity_is_preserved_but_not_misclassified() {
    let mut input = supported_input();
    input.legacy_stored_quantity = Some("2.5");
    let result = calculate_preparation(input);
    assert_eq!(
        result.legacy_reference.stored_quantity.as_deref(),
        Some("2.5")
    );
    assert_eq!(
        result.legacy_reference.comparison_status,
        LegacyReferenceComparisonStatus::NotComparable
    );
}

#[test]
fn malformed_and_overflowing_values_fail_closed_and_repeat_deterministically() {
    let mut input = supported_input();
    input.ordered_dose_text = Some("9999999999999999999999999999999999999999");
    let first = calculate_preparation(input);
    let second = calculate_preparation(input);
    assert_eq!(first, second);
    assert_eq!(first.status, PreparationCalculationStatus::Unsupported);
}
