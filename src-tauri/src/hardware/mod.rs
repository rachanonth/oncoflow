pub(crate) mod commands;
mod model;
mod renderer;
mod spooler;

use thiserror::Error;

use crate::{auth::AuthError, output::OutputError};

pub(crate) use model::*;
#[cfg(test)]
pub(crate) use renderer::render_preparation_label;
pub(crate) use renderer::LABEL_RENDERER_VERSION;

#[derive(Debug, Error)]
pub(crate) enum HardwareError {
    #[error("invalid printer configuration field: {0}")]
    InvalidConfig(&'static str),
    #[cfg(not(windows))]
    #[error("RAW printing is not supported on this platform")]
    UnsupportedPlatform,
    #[error("a Thai-capable label font is unavailable")]
    FontUnavailable,
    #[error("Windows spooler operation {operation} failed with code {code}")]
    WindowsSpooler { operation: &'static str, code: u32 },
    #[error("rendered print payload is too large")]
    PayloadTooLarge,
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Output(#[from] OutputError),
}
