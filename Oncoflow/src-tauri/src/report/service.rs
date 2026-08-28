use thiserror::Error;

use crate::{
    auth::{AuthError, AuthSession},
    db::{Database, DatabaseError},
};

use super::{repository, PreparationCountReport, PreparationCountReportRequest};

#[derive(Debug, Error)]
pub(crate) enum ReportError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct ReportService<'a> {
    database: &'a Database,
    session: &'a AuthSession,
}

impl<'a> ReportService<'a> {
    pub(crate) fn new(database: &'a Database, session: &'a AuthSession) -> Self {
        Self { database, session }
    }

    pub(crate) fn preparation_counts(
        &self,
        request: PreparationCountReportRequest,
    ) -> Result<PreparationCountReport, ReportError> {
        self.session.require_user()?;
        validate_date(&request.date_from, "dateFrom")?;
        validate_date(&request.date_to, "dateTo")?;
        if request.date_from > request.date_to {
            return Err(ReportError::Validation {
                field: "dateTo",
                message: "End date must be on or after start date.".into(),
            });
        }
        let connection = self.database.open()?;
        let rows = repository::preparation_counts(
            &connection,
            request.interval,
            &request.date_from,
            &request.date_to,
        )?;
        let total_prescriptions = rows.iter().map(|row| row.prescription_count).sum();
        let total_bottles = rows.iter().map(|row| row.bottle_count).sum();
        Ok(PreparationCountReport {
            interval: request.interval,
            date_from: request.date_from,
            date_to: request.date_to,
            total_prescriptions,
            total_bottles,
            rows,
        })
    }
}

fn validate_date(value: &str, field: &'static str) -> Result<(), ReportError> {
    let parts: Vec<_> = value.split('-').collect();
    let parsed = (parts.get(0), parts.get(1), parts.get(2));
    let (year, month, day) = match parsed {
        (Some(year), Some(month), Some(day))
            if parts.len() == 3 && year.len() == 4 && month.len() == 2 && day.len() == 2 =>
        {
            (
                year.parse::<i32>().ok(),
                month.parse::<u32>().ok(),
                day.parse::<u32>().ok(),
            )
        }
        _ => (None, None, None),
    };
    let valid = match (year, month, day) {
        (Some(year), Some(month @ 1..=12), Some(day)) => {
            let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            let days = [
                31,
                if leap { 29 } else { 28 },
                31,
                30,
                31,
                30,
                31,
                31,
                30,
                31,
                30,
                31,
            ];
            day >= 1 && day <= days[(month - 1) as usize]
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(ReportError::Validation {
            field,
            message: "Enter a valid date.".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_calendar_dates() {
        assert!(validate_date("2026-02-29", "dateFrom").is_err());
        assert!(validate_date("2028-02-29", "dateFrom").is_ok());
        assert!(validate_date("2026-8-01", "dateFrom").is_err());
    }
}
