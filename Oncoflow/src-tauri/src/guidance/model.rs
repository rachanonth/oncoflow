use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageGuidanceRecord {
    pub page_key: String,
    pub guidance: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct UpdatePageGuidanceInput {
    pub page_key: String,
    pub guidance: Option<String>,
}
