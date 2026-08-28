use serde::Serialize;
use tauri::State;

use crate::{
    auth::{AuthError, AuthSession},
    db::Database,
    output::{OutputError, OutputService},
};

use super::{
    renderer, spooler, HardwareError, LabelPrinterConfig, PreparationBatchPrintResult,
    PreparationPrintResult, PrintJobReceipt, PrinterQueueStatus, LABEL_RENDERER_VERSION,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandError {
    code: &'static str,
    message: String,
    field: Option<&'static str>,
}

impl From<HardwareError> for CommandError {
    fn from(error: HardwareError) -> Self {
        match error {
            HardwareError::InvalidConfig(field) => Self {
                code: "validation",
                message: "Review the local label-printer configuration.".into(),
                field: Some(field),
            },
            HardwareError::Auth(AuthError::AuthenticationRequired) => Self::plain(
                "authentication_required",
                "Sign in with a local OncoFlow account to use label printing.",
            ),
            HardwareError::Auth(_) => Self::plain(
                "authentication_error",
                "The authenticated local session could not be confirmed.",
            ),
            #[cfg(not(windows))]
            HardwareError::UnsupportedPlatform => Self::plain(
                "unsupported_platform",
                "RAW label printing is available on Windows only.",
            ),
            HardwareError::FontUnavailable => Self::plain(
                "font_unavailable",
                "A Thai-capable Windows font could not be loaded for label rendering.",
            ),
            HardwareError::WindowsSpooler { operation, code } => Self::plain(
                "printer_error",
                format!("Windows print spooler operation {operation} failed (code {code})."),
            ),
            HardwareError::PayloadTooLarge => Self::plain(
                "printer_error",
                "The rendered label is too large for a Windows RAW print job.",
            ),
            HardwareError::Output(error) => Self::from(error),
        }
    }
}

impl From<OutputError> for CommandError {
    fn from(error: OutputError) -> Self {
        match error {
            OutputError::TaskNotFound => {
                Self::plain("not_found", "Preparation task was not found.")
            }
            OutputError::VerificationRequired => Self::plain(
                "preparation_check_required",
                "Only a checked preparation can produce a final label.",
            ),
            OutputError::IncompleteProvenance => Self::plain(
                "incomplete_provenance",
                "The checked preparation does not contain enough provenance for final output.",
            ),
            OutputError::InvalidSelection => Self::plain(
                "invalid_selection",
                "Select preparation items that belong to the current order.",
            ),
            OutputError::Auth(AuthError::AuthenticationRequired) => Self::plain(
                "authentication_required",
                "Sign in with a local OncoFlow account to print a final label.",
            ),
            OutputError::Auth(_) => Self::plain(
                "authentication_error",
                "The authenticated local session could not be confirmed.",
            ),
            OutputError::Database(_) | OutputError::Sqlite(_) => Self::plain(
                "database_error",
                "The local preparation output operation failed.",
            ),
        }
    }
}

impl CommandError {
    fn plain(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
        }
    }
}

#[tauri::command]
pub(crate) fn list_system_printers(
    session: State<'_, AuthSession>,
) -> Result<Vec<String>, CommandError> {
    session.require_user().map_err(HardwareError::from)?;
    spooler::list_printers().map_err(Into::into)
}

#[tauri::command]
pub(crate) fn validate_printer_queue(
    session: State<'_, AuthSession>,
    spooler_name: Option<String>,
) -> Result<PrinterQueueStatus, CommandError> {
    session.require_user().map_err(HardwareError::from)?;
    let printers = spooler::list_printers()?;
    Ok(printer_queue_status(spooler_name, &printers))
}

fn printer_queue_status(spooler_name: Option<String>, printers: &[String]) -> PrinterQueueStatus {
    let configured_queue = spooler_name.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    });
    let available = configured_queue
        .as_ref()
        .is_some_and(|configured| printers.iter().any(|queue| queue == configured));
    PrinterQueueStatus {
        configured_queue,
        available,
        installed_queue_count: printers.len(),
        physical_output_confirmed: false,
    }
}

#[tauri::command]
pub(crate) fn print_test_label(
    session: State<'_, AuthSession>,
    config: LabelPrinterConfig,
) -> Result<PrintJobReceipt, CommandError> {
    session.require_user().map_err(HardwareError::from)?;
    let bytes = renderer::render_test_label(&config)?;
    spooler::submit_raw(&config.spooler_name, "OncoFlow label printer test", &bytes)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn print_preparation_label(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    preparation_id: i64,
    config: LabelPrinterConfig,
) -> Result<PreparationPrintResult, CommandError> {
    let service = OutputService::new(&database, &session);
    let output = service.get_preparation_output(preparation_id)?;
    let bytes = renderer::render_preparation_label(&output, &config)?;
    let job = spooler::submit_raw(
        &config.spooler_name,
        &format!("OncoFlow Preparation {preparation_id}"),
        &bytes,
    )?;
    let output = service
        .record_rendered_label_print_request(
            preparation_id,
            "windows_raw_spooler",
            LABEL_RENDERER_VERSION,
            &output.label.print_time,
        )
        .map_err(|_| {
            CommandError::plain(
                "print_audit_error",
                format!(
                    "Windows accepted print job #{}, but OncoFlow could not append its local audit event. Check the Windows queue before retrying.",
                    job.windows_job_id
                ),
            )
        })?;
    Ok(PreparationPrintResult { output, job })
}

#[tauri::command]
pub(crate) fn print_order_preparation_labels(
    database: State<'_, Database>,
    session: State<'_, AuthSession>,
    order_id: i64,
    preparation_ids: Vec<i64>,
    config: LabelPrinterConfig,
) -> Result<PreparationBatchPrintResult, CommandError> {
    let service = OutputService::new(&database, &session);
    let outputs = service.get_order_outputs(order_id, &preparation_ids)?;
    let bytes = renderer::render_preparation_labels(&outputs, &config)?;
    let job = spooler::submit_raw(
        &config.spooler_name,
        &format!("OncoFlow Order {order_id} Preparation Labels"),
        &bytes,
    )?;
    let mut recorded = Vec::with_capacity(outputs.len());
    for output in outputs {
        recorded.push(service.record_rendered_label_print_request(
            output.label.preparation_id,
            "windows_raw_spooler_batch",
            LABEL_RENDERER_VERSION,
            &output.label.print_time,
        ).map_err(|_| {
            CommandError::plain(
                "print_audit_error",
                format!(
                    "Windows accepted batch print job #{}, but OncoFlow could not append every local print audit event. Check the Windows queue before retrying.",
                    job.windows_job_id
                ),
            )
        })?);
    }
    Ok(PreparationBatchPrintResult {
        outputs: recorded,
        job,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_configured_queue_is_detected_without_submitting_a_print_job() {
        let status = printer_queue_status(
            Some("Xprinter XP-420B".into()),
            &["Synthetic queue".into(), "Microsoft Print to PDF".into()],
        );
        assert!(!status.available);
        assert!(!status.physical_output_confirmed);
        assert_eq!(status.installed_queue_count, 2);

        let available = printer_queue_status(
            Some("Xprinter XP-420B".into()),
            &["Xprinter XP-420B".into()],
        );
        assert!(available.available);
        assert!(!available.physical_output_confirmed);
    }
}
