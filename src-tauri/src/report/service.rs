use thiserror::Error;

use crate::{
    auth::{AuthError, AuthSession},
    db::{Database, DatabaseError},
};

use super::{
    repository, InventoryUsageReport, InventoryUsageReportRequest, PreparationCountReport,
    PreparationCountReportRequest,
};

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
    pub(crate) fn diluent_usage(
        &self,
        request: super::DiluentUsageReportRequest,
    ) -> Result<super::DiluentUsageReport, ReportError> {
        self.session.require_user()?;
        validate_date(&request.preparation_date, "preparationDate")?;
        let connection = self.database.open()?;
        let rows = repository::diluent_usage(&connection, &request.preparation_date)?;
        Ok(super::DiluentUsageReport {
            preparation_date: request.preparation_date,
            total_bottles: rows.iter().map(|row| row.bottle_count).sum(),
            total_doses: rows.iter().map(|row| row.dose_count).sum(),
            rows,
        })
    }

    pub(crate) fn ward_delivery(
        &self,
        request: super::WardDeliveryReportRequest,
    ) -> Result<super::WardDeliveryReport, ReportError> {
        self.session.require_user()?;
        validate_date(&request.preparation_date, "preparationDate")?;
        let connection = self.database.open()?;
        let rows = repository::ward_delivery(&connection, &request.preparation_date)?;
        Ok(super::WardDeliveryReport {
            preparation_date: request.preparation_date,
            rows,
        })
    }

    pub(crate) fn new(database: &'a Database, session: &'a AuthSession) -> Self {
        Self { database, session }
    }

    pub(crate) fn preparation_counts(
        &self,
        request: PreparationCountReportRequest,
    ) -> Result<PreparationCountReport, ReportError> {
        self.session.require_user()?;
        validate_range(&request.date_from, &request.date_to)?;
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

    pub(crate) fn inventory_usage(
        &self,
        request: InventoryUsageReportRequest,
    ) -> Result<InventoryUsageReport, ReportError> {
        self.session.require_user()?;
        validate_range(&request.date_from, &request.date_to)?;
        let connection = self.database.open()?;
        let rows = repository::inventory_usage(
            &connection,
            request.interval,
            &request.date_from,
            &request.date_to,
        )?;
        let total_prescriptions = rows.iter().map(|row| row.prescription_count).sum();
        let total_prepared_bottles = rows.iter().map(|row| row.prepared_bottle_count).sum();
        let total_issued_source_containers = rows
            .iter()
            .map(|row| row.issued_source_container_count)
            .sum();
        let drug_count = rows
            .iter()
            .map(|row| row.drug_id)
            .collect::<std::collections::HashSet<_>>()
            .len() as i64;
        Ok(InventoryUsageReport {
            interval: request.interval,
            date_from: request.date_from,
            date_to: request.date_to,
            total_prescriptions,
            total_prepared_bottles,
            total_issued_source_containers,
            drug_count,
            rows,
        })
    }
}

fn validate_range(date_from: &str, date_to: &str) -> Result<(), ReportError> {
    validate_date(date_from, "dateFrom")?;
    validate_date(date_to, "dateTo")?;
    if date_from > date_to {
        return Err(ReportError::Validation {
            field: "dateTo",
            message: "End date must be on or after start date.".into(),
        });
    }
    Ok(())
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
    fn delivery_requires_authentication_and_validates_the_requested_date() {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
        let session = AuthSession::default();
        let request = super::super::WardDeliveryReportRequest {
            preparation_date: "2026-09-29".into(),
        };
        assert!(matches!(
            ReportService::new(&database, &session).ward_delivery(request),
            Err(ReportError::Auth(_))
        ));
        crate::auth::AuthService::new(&database, &session)
            .bootstrap(crate::auth::BootstrapUserInput {
                username: "synthetic.report".into(),
                display_name: "Report test".into(),
                password: "Synthetic!Passphrase42".into(),
            })
            .unwrap();
        let invalid = super::super::WardDeliveryReportRequest {
            preparation_date: "2026-02-30".into(),
        };
        assert!(matches!(
            ReportService::new(&database, &session).ward_delivery(invalid),
            Err(ReportError::Validation {
                field: "preparationDate",
                ..
            })
        ));
        let valid = super::super::WardDeliveryReportRequest {
            preparation_date: "2026-09-29".into(),
        };
        assert!(ReportService::new(&database, &session)
            .ward_delivery(valid)
            .unwrap()
            .rows
            .is_empty());
    }

    #[test]
    fn rejects_invalid_calendar_dates() {
        assert!(validate_date("2026-02-29", "dateFrom").is_err());
        assert!(validate_date("2028-02-29", "dateFrom").is_ok());
        assert!(validate_date("2026-8-01", "dateFrom").is_err());
    }

    #[test]
    fn diluent_report_requires_auth_and_valid_date_on_migrated_database() {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
        let session = AuthSession::default();
        let request = super::super::DiluentUsageReportRequest {
            preparation_date: "2026-10-01".into(),
        };
        assert!(matches!(
            ReportService::new(&database, &session).diluent_usage(request.clone()),
            Err(ReportError::Auth(_))
        ));
        crate::auth::AuthService::new(&database, &session)
            .bootstrap(crate::auth::BootstrapUserInput {
                username: "synthetic.diluent".into(),
                display_name: "Diluent test".into(),
                password: "Synthetic!Passphrase42".into(),
            })
            .unwrap();
        let invalid = super::super::DiluentUsageReportRequest {
            preparation_date: "2026-02-30".into(),
        };
        assert!(matches!(
            ReportService::new(&database, &session).diluent_usage(invalid),
            Err(ReportError::Validation {
                field: "preparationDate",
                ..
            })
        ));
        let report = ReportService::new(&database, &session)
            .diluent_usage(request)
            .unwrap();
        assert_eq!(report.preparation_date, "2026-10-01");
        assert!(report.rows.is_empty());
        assert_eq!((report.total_bottles, report.total_doses), (0, 0));
    }
}
