use serde::{Deserialize, Serialize};

use crate::{auth::UserRole, preparation_calc::PreparationCalculation, safety::SafetyEvaluation};

use super::{EligibilityDecision, PreparationReferenceQuantity};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct PreparationQueueRequest {
    pub search: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub preparation_date: Option<String>,
    pub source_filter: PreparationQueueSourceFilter,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PreparationQueueSourceFilter {
    #[default]
    All,
    SameDay,
    Continuing,
    Rescheduled,
}

impl PreparationQueueSourceFilter {
    pub(super) const fn as_database(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::SameDay => "same_day",
            Self::Continuing => "continuing",
            Self::Rescheduled => "rescheduled",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationQueueItem {
    pub order_id: i64,
    pub order_code: String,
    pub patient_hn: String,
    pub patient_name: String,
    pub ward_name: Option<String>,
    pub regimen_name: Option<String>,
    pub treatment_time: Option<String>,
    pub preparation_date: String,
    pub source_kind: PreparationQueueSourceFilter,
    pub eligible_item_count: u64,
    pub initialized_item_count: u64,
    pub pending_item_count: u64,
    pub prepared_item_count: u64,
    pub verified_item_count: u64,
    pub printed_label_count: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationQueueResponse {
    pub items: Vec<PreparationQueueItem>,
    pub total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PreparationState {
    Pending,
    Prepared,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PreparationInventoryPostingStatus {
    Posted,
    ManualReconciliationRequired,
    NotRequired,
    TrackingDisabled,
}

impl PreparationInventoryPostingStatus {
    pub(super) const fn as_database(self) -> &'static str {
        match self {
            Self::Posted => "posted",
            Self::ManualReconciliationRequired => "manual_reconciliation_required",
            Self::NotRequired => "not_required",
            Self::TrackingDisabled => "tracking_disabled",
        }
    }

    pub(super) fn from_database(value: &str) -> rusqlite::Result<Self> {
        match value {
            "posted" => Ok(Self::Posted),
            "manual_reconciliation_required" => Ok(Self::ManualReconciliationRequired),
            "not_required" => Ok(Self::NotRequired),
            "tracking_disabled" => Ok(Self::TrackingDisabled),
            value => Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                format!("invalid preparation inventory posting status: {value}").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PreparationIssueStockState {
    Normal,
    Low,
    Out,
    Shortage,
}

impl PreparationIssueStockState {
    pub(super) const fn as_database(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Low => "low",
            Self::Out => "out",
            Self::Shortage => "shortage",
        }
    }

    pub(super) fn from_database(value: &str) -> rusqlite::Result<Self> {
        match value {
            "normal" => Ok(Self::Normal),
            "low" => Ok(Self::Low),
            "out" => Ok(Self::Out),
            "shortage" => Ok(Self::Shortage),
            value => Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                format!("invalid preparation issue stock state: {value}").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationInventoryPosting {
    pub id: i64,
    pub status: PreparationInventoryPostingStatus,
    pub inventory_movement_id: Option<i64>,
    pub containers_required: Option<String>,
    pub balance_before: Option<String>,
    pub balance_after: Option<String>,
    pub resulting_stock_state: Option<PreparationIssueStockState>,
    pub calculation_status: String,
    pub calculation_ruleset_version: String,
    pub calculation_rule_id: String,
    pub workflow_rule_id: String,
    pub reason_code: String,
    pub issued_at: Option<String>,
    pub recorded_at: String,
    pub actor: PreparationActor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationActor {
    pub id: i64,
    pub display_name: String,
    pub role: UserRole,
}

impl PreparationState {
    pub(super) fn from_database(value: &str) -> rusqlite::Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "prepared" => Ok(Self::Prepared),
            "verified" => Ok(Self::Verified),
            value => Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                format!("invalid preparation state: {value}").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationTask {
    pub id: i64,
    pub source_order_id: i64,
    pub source_order_item_id: i64,
    pub preparation_date: String,
    pub drug_id: i64,
    pub state: PreparationState,
    pub ordered_dose_text: Option<String>,
    pub dose_unit_text: Option<String>,
    pub diluent_id: Option<i64>,
    pub diluent_name: Option<String>,
    pub diluent_volume_ml: Option<f64>,
    pub route_id: Option<i64>,
    pub route_name: Option<String>,
    pub rate_text: Option<String>,
    pub treatment_day: Option<String>,
    pub start_date: Option<String>,
    pub stop_date: Option<String>,
    pub sequence_no: Option<i64>,
    pub regimen_details: Option<String>,
    pub drug_detail: Option<String>,
    pub drug_storage: Option<String>,
    pub preparation_volume_ml: Option<f64>,
    pub preparation_notes: Option<String>,
    pub final_container_count: u32,
    pub created_at: String,
    pub updated_at: String,
    pub prepared_at: Option<String>,
    pub verified_at: Option<String>,
    pub prepared_by: Option<PreparationActor>,
    pub verified_by: Option<PreparationActor>,
    pub inventory_posting: Option<PreparationInventoryPosting>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourceSnapshot {
    pub source_order_id: i64,
    pub source_order_item_id: i64,
    pub drug_id: i64,
    pub ordered_dose_text: Option<String>,
    pub dose_unit_text: Option<String>,
    pub diluent_id: Option<i64>,
    pub diluent_name: Option<String>,
    pub diluent_volume_ml: Option<f64>,
    pub route_id: Option<i64>,
    pub route_name: Option<String>,
    pub rate_text: Option<String>,
    pub treatment_day: Option<String>,
    pub start_date: Option<String>,
    pub stop_date: Option<String>,
    pub sequence_no: Option<i64>,
    pub regimen_details: Option<String>,
    pub drug_detail: Option<String>,
    pub drug_storage: Option<String>,
}

impl PreparationTask {
    pub(super) fn snapshot(&self) -> SourceSnapshot {
        SourceSnapshot {
            source_order_id: self.source_order_id,
            source_order_item_id: self.source_order_item_id,
            drug_id: self.drug_id,
            ordered_dose_text: self.ordered_dose_text.clone(),
            dose_unit_text: self.dose_unit_text.clone(),
            diluent_id: self.diluent_id,
            diluent_name: self.diluent_name.clone(),
            diluent_volume_ml: self.diluent_volume_ml,
            route_id: self.route_id,
            route_name: self.route_name.clone(),
            rate_text: self.rate_text.clone(),
            treatment_day: self.treatment_day.clone(),
            start_date: self.start_date.clone(),
            stop_date: self.stop_date.clone(),
            sequence_no: self.sequence_no,
            regimen_details: self.regimen_details.clone(),
            drug_detail: self.drug_detail.clone(),
            drug_storage: self.drug_storage.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationWorkspaceItem {
    pub order_item_id: i64,
    pub drug_id: i64,
    pub drug_code: String,
    pub drug_name: String,
    pub ordered_dose_text: Option<String>,
    pub dose_unit_text: Option<String>,
    pub diluent_name: Option<String>,
    pub diluent_volume_ml: Option<f64>,
    pub route_name: Option<String>,
    pub rate_text: Option<String>,
    pub treatment_day: Option<String>,
    pub sequence_no: Option<i64>,
    pub regimen_details: Option<String>,
    pub drug_detail: Option<String>,
    pub drug_storage: Option<String>,
    pub eligibility: EligibilityDecision,
    pub reference_quantity: PreparationReferenceQuantity,
    pub calculation: PreparationCalculation,
    pub default_preparation_volume_ml: Option<String>,
    pub task: Option<PreparationTask>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationWorkspace {
    pub order_id: i64,
    pub order_code: String,
    pub patient_hn: String,
    pub patient_name: String,
    pub ward_name: Option<String>,
    pub regimen_name: Option<String>,
    pub treatment_time: Option<String>,
    pub preparation_date: String,
    pub assigned_preparer: Option<PreparationActor>,
    pub editable: bool,
    pub eligibility_rule_id: &'static str,
    pub excluded_item_count: u64,
    pub pharmacists: Vec<PreparationActor>,
    pub items: Vec<PreparationWorkspaceItem>,
    pub safety: SafetyEvaluation,
    pub safety_acknowledgements: Vec<SafetyAcknowledgement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SafetyAcknowledgement {
    pub preparation_task_id: Option<i64>,
    pub order_item_id: Option<i64>,
    pub finding_id: String,
    pub finding_fingerprint: String,
    pub rule_id: String,
    pub ruleset_version: String,
    pub user: PreparationActor,
    pub acknowledged_at: String,
    pub source_snapshot_stale: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct PreparationTaskInput {
    pub preparation_volume_ml: Option<f64>,
    pub preparation_notes: Option<String>,
    pub final_container_count: Option<u32>,
}
