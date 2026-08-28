#[cfg(windows)]
mod windows {
    use std::{ffi::OsStr, iter, os::windows::ffi::OsStrExt, ptr};

    use winapi::{
        shared::minwindef::DWORD,
        um::{
            errhandlingapi::GetLastError,
            winspool::{
                ClosePrinter, EndDocPrinter, EndPagePrinter, EnumPrintersW, OpenPrinterW,
                StartDocPrinterW, StartPagePrinter, WritePrinter, DOC_INFO_1W,
                PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_INFO_4W,
            },
        },
    };

    use super::super::{HardwareError, PrintJobReceipt, LABEL_RENDERER_VERSION};

    pub(super) fn list_printers() -> Result<Vec<String>, HardwareError> {
        unsafe {
            let mut needed: DWORD = 0;
            let mut returned: DWORD = 0;
            EnumPrintersW(
                PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS,
                ptr::null_mut(),
                4,
                ptr::null_mut(),
                0,
                &mut needed,
                &mut returned,
            );
            if needed == 0 {
                return Ok(Vec::new());
            }
            let mut buffer = vec![0_u8; needed as usize];
            if EnumPrintersW(
                PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS,
                ptr::null_mut(),
                4,
                buffer.as_mut_ptr(),
                needed,
                &mut needed,
                &mut returned,
            ) == 0
            {
                return Err(last_error("EnumPrintersW"));
            }
            let printers = std::slice::from_raw_parts(
                buffer.as_ptr().cast::<PRINTER_INFO_4W>(),
                returned as usize,
            );
            let mut names = printers
                .iter()
                .filter_map(|printer| wide_ptr_to_string(printer.pPrinterName))
                .collect::<Vec<_>>();
            names.sort_by_key(|name| name.to_lowercase());
            names.dedup();
            Ok(names)
        }
    }

    pub(super) fn submit_raw(
        queue_name: &str,
        document_name: &str,
        bytes: &[u8],
    ) -> Result<PrintJobReceipt, HardwareError> {
        let queue = wide(queue_name);
        let document = wide(document_name);
        let datatype = wide("RAW");
        let byte_count: DWORD = bytes
            .len()
            .try_into()
            .map_err(|_| HardwareError::PayloadTooLarge)?;
        unsafe {
            let mut handle = ptr::null_mut();
            if OpenPrinterW(queue.as_ptr() as *mut _, &mut handle, ptr::null_mut()) == 0 {
                return Err(last_error("OpenPrinterW"));
            }
            let mut document_info = DOC_INFO_1W {
                pDocName: document.as_ptr() as *mut _,
                pOutputFile: ptr::null_mut(),
                pDatatype: datatype.as_ptr() as *mut _,
            };
            let job_id = StartDocPrinterW(
                handle,
                1,
                (&mut document_info as *mut DOC_INFO_1W).cast::<u8>(),
            );
            if job_id == 0 {
                let error = last_error("StartDocPrinterW");
                ClosePrinter(handle);
                return Err(error);
            }
            if StartPagePrinter(handle) == 0 {
                let error = last_error("StartPagePrinter");
                EndDocPrinter(handle);
                ClosePrinter(handle);
                return Err(error);
            }
            let mut written: DWORD = 0;
            let write_ok = WritePrinter(handle, bytes.as_ptr() as *mut _, byte_count, &mut written);
            let write_error = if write_ok == 0 || written != byte_count {
                Some(last_error("WritePrinter"))
            } else {
                None
            };
            EndPagePrinter(handle);
            EndDocPrinter(handle);
            ClosePrinter(handle);
            if let Some(error) = write_error {
                return Err(error);
            }
            Ok(PrintJobReceipt {
                windows_job_id: job_id,
                bytes_submitted: written,
                renderer_version: LABEL_RENDERER_VERSION,
            })
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(iter::once(0))
            .collect()
    }

    unsafe fn wide_ptr_to_string(pointer: *mut u16) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        let mut length = 0;
        while *pointer.add(length) != 0 {
            length += 1;
        }
        Some(String::from_utf16_lossy(std::slice::from_raw_parts(
            pointer, length,
        )))
    }

    unsafe fn last_error(operation: &'static str) -> HardwareError {
        HardwareError::WindowsSpooler {
            operation,
            code: GetLastError(),
        }
    }
}

#[cfg(windows)]
pub(super) fn list_printers() -> Result<Vec<String>, super::HardwareError> {
    windows::list_printers()
}

#[cfg(windows)]
pub(super) fn submit_raw(
    queue_name: &str,
    document_name: &str,
    bytes: &[u8],
) -> Result<super::PrintJobReceipt, super::HardwareError> {
    windows::submit_raw(queue_name, document_name, bytes)
}

#[cfg(not(windows))]
pub(super) fn list_printers() -> Result<Vec<String>, super::HardwareError> {
    Err(super::HardwareError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub(super) fn submit_raw(
    _queue_name: &str,
    _document_name: &str,
    _bytes: &[u8],
) -> Result<super::PrintJobReceipt, super::HardwareError> {
    Err(super::HardwareError::UnsupportedPlatform)
}
