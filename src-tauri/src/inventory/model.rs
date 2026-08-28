use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum InventorySortField {
    #[default]
    Code,
    Name,
    CurrentStock,
    Minimum,
    Maximum,
    State,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct InventoryListRequest {
    pub search: Option<String>,
    pub tracked_only: bool,
    pub low_stock_only: bool,
    pub sort_by: InventorySortField,
    pub sort_direction: SortDirection,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StockState {
    Untracked,
    Unknown,
    Shortage,
    Out,
    Low,
    Normal,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventorySummary {
    pub drug_id: i64,
    pub drug_code: String,
    pub drug_name: String,
    pub legacy_drug_unit: Option<String>,
    pub package: Option<String>,
    pub current_stock: Option<f64>,
    pub minimum_stock: Option<f64>,
    pub maximum_stock: Option<f64>,
    pub tracking_enabled: bool,
    pub stock_state: StockState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryListResponse {
    pub items: Vec<InventorySummary>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryDetail {
    #[serde(flatten)]
    pub summary: InventorySummary,
    pub legacy_inventory_snapshot: Option<f64>,
    pub legacy_inventory_cutoff: Option<bool>,
    pub dose_per_pack: Option<f64>,
    pub volume_per_pack_ml: Option<f64>,
    pub legacy_inventory_event_count: u64,
    pub quantity_semantics: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InventoryMovementType {
    OpeningBalance,
    Receipt,
    ManualIssue,
    AdjustmentIncrease,
    AdjustmentDecrease,
    PreparationIssue,
}

impl InventoryMovementType {
    pub(super) const fn as_database(self) -> &'static str {
        match self {
            Self::OpeningBalance => "opening_balance",
            Self::Receipt => "receipt",
            Self::ManualIssue => "manual_issue",
            Self::AdjustmentIncrease => "adjustment_increase",
            Self::AdjustmentDecrease => "adjustment_decrease",
            Self::PreparationIssue => "preparation_issue",
        }
    }

    pub(super) fn from_database(value: &str) -> rusqlite::Result<Self> {
        match value {
            "opening_balance" => Ok(Self::OpeningBalance),
            "receipt" => Ok(Self::Receipt),
            "manual_issue" => Ok(Self::ManualIssue),
            "adjustment_increase" => Ok(Self::AdjustmentIncrease),
            "adjustment_decrease" => Ok(Self::AdjustmentDecrease),
            "preparation_issue" => Ok(Self::PreparationIssue),
            value => Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                format!("unsupported inventory movement type: {value}").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryMovement {
    pub id: i64,
    pub movement_type: InventoryMovementType,
    pub quantity_delta: f64,
    pub resulting_balance: f64,
    pub occurred_at: Option<String>,
    pub created_at: String,
    pub actor_display_name: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub note: Option<String>,
    pub preparation_task_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct InventoryMovementListRequest {
    pub drug_id: i64,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryMovementListResponse {
    pub items: Vec<InventoryMovement>,
    pub total: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct InventoryReceiptInput {
    pub drug_id: i64,
    pub quantity: f64,
    pub occurred_at: Option<String>,
    pub reference: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AdjustmentDirection {
    #[default]
    Increase,
    Decrease,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct InventoryAdjustmentInput {
    pub drug_id: i64,
    pub direction: AdjustmentDirection,
    pub quantity: f64,
    pub occurred_at: Option<String>,
    pub note: String,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct InventoryManualIssueInput {
    pub drug_id: i64,
    pub quantity: f64,
    pub occurred_at: Option<String>,
    pub note: String,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryMovementResult {
    pub inventory: InventoryDetail,
    pub movement: InventoryMovement,
}
