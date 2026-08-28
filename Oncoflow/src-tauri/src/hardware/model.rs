use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrinterLanguage {
    Escpos,
    Tspl,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LabelPrinterConfig {
    pub spooler_name: String,
    pub language: PrinterLanguage,
    pub width_mm: f32,
    pub height_mm: f32,
    pub dpi: u32,
    pub gap_mm: f32,
    #[serde(default = "default_preprint_header_spacing_mm")]
    pub preprint_header_spacing_mm: f32,
    #[serde(default)]
    pub font_sizes: LabelFontSizes,
}

const fn default_preprint_header_spacing_mm() -> f32 {
    5.0
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LabelFontSizes {
    pub header: f32,
    pub patient: f32,
    pub withdrawal: f32,
    pub drug: f32,
    pub route_rate: f32,
    pub storage: f32,
    pub warning: f32,
    pub prepared_by: f32,
    pub expiration: f32,
}

impl Default for LabelFontSizes {
    fn default() -> Self {
        Self {
            header: 22.0,
            patient: 20.0,
            withdrawal: 16.0,
            drug: 21.0,
            route_rate: 18.0,
            storage: 16.0,
            warning: 16.0,
            prepared_by: 15.0,
            expiration: 18.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrintJobReceipt {
    pub windows_job_id: u32,
    pub bytes_submitted: u32,
    pub renderer_version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationPrintResult {
    pub output: crate::output::PreparationOutput,
    pub job: PrintJobReceipt,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationBatchPrintResult {
    pub outputs: Vec<crate::output::PreparationOutput>,
    pub job: PrintJobReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrinterQueueStatus {
    pub configured_queue: Option<String>,
    pub available: bool,
    pub installed_queue_count: usize,
    pub physical_output_confirmed: bool,
}
