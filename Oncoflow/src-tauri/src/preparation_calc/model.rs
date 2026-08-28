use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PreparationCalculationStatus {
    Calculated,
    PartiallyCalculated,
    Unavailable,
    Unsupported,
}

impl PreparationCalculationStatus {
    pub(crate) const fn as_database(self) -> &'static str {
        match self {
            Self::Calculated => "calculated",
            Self::PartiallyCalculated => "partially_calculated",
            Self::Unavailable => "unavailable",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InventoryProjectionState {
    Normal,
    Low,
    Out,
    Shortage,
    Unknown,
    Untracked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalculationQuantity {
    pub value: String,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentationResult {
    pub amount_per_container: Option<CalculationQuantity>,
    pub volume_per_container_ml: Option<String>,
    pub container_label: Option<String>,
    pub raw_package_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryProjection {
    pub tracking_enabled: bool,
    pub current_stock: Option<String>,
    pub containers_required: Option<String>,
    pub projected_stock: Option<String>,
    pub minimum_stock: Option<String>,
    pub state: InventoryProjectionState,
    pub unit_notice: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LegacyReferenceComparisonStatus {
    FormulaConfirmed,
    NotComparable,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacyReferenceComparison {
    pub stored_quantity: Option<String>,
    pub stored_quantity_semantics: &'static str,
    pub calculated_package_equivalent: Option<String>,
    pub calculated_solution_volume_ml: Option<String>,
    pub comparison_status: LegacyReferenceComparisonStatus,
    pub notice: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalculationTraceStep {
    pub step: &'static str,
    pub expression: String,
    pub result: Option<String>,
    pub confidence: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalculationWarning {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationCalculation {
    pub status: PreparationCalculationStatus,
    pub ruleset_version: &'static str,
    pub rule_id: &'static str,
    pub ordered_dose: Option<CalculationQuantity>,
    pub presentation: PresentationResult,
    pub concentration: Option<String>,
    pub withdrawal_volume_ml: Option<String>,
    pub containers_required: Option<String>,
    pub unused_amount: Option<CalculationQuantity>,
    pub inventory_projection: InventoryProjection,
    pub legacy_reference: LegacyReferenceComparison,
    pub trace: Vec<CalculationTraceStep>,
    pub warnings: Vec<CalculationWarning>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PreparationCalculationInput<'a> {
    pub ordered_dose_text: Option<&'a str>,
    pub ordered_dose_unit: Option<&'a str>,
    pub amount_per_container: Option<&'a str>,
    pub presentation_unit: Option<&'a str>,
    pub volume_per_container_ml: Option<&'a str>,
    pub package_label: Option<&'a str>,
    pub legacy_stored_quantity: Option<&'a str>,
    pub inventory_tracking_enabled: bool,
    pub current_inventory: Option<&'a str>,
    pub minimum_inventory: Option<&'a str>,
}
