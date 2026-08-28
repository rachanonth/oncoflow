use rusqlite::TransactionBehavior;
use serde_json::json;
use thiserror::Error;

use crate::{
    auth::{audit, AuthError, AuthService, AuthSession},
    db::{Database, DatabaseError},
};

use super::{
    repository, DiagnosisInput, DiagnosisRecord, DiluentInput, DiluentRecord, DoctorInput,
    DoctorRecord, MasterDataListRequest, RouteInput, RouteRecord, WardInput, WardRecord,
};

const CODE_MAX_CHARS: usize = 50;
const NAME_MAX_CHARS: usize = 200;
const TELEPHONE_MAX_CHARS: usize = 100;

#[derive(Debug, Error)]
pub(crate) enum MasterDataError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error("doctor was not found")]
    DoctorNotFound,
    #[error("ward was not found")]
    WardNotFound,
    #[error("route was not found")]
    RouteNotFound,
    #[error("diluent was not found")]
    DiluentNotFound,
    #[error("diagnosis was not found")]
    DiagnosisNotFound,
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub(crate) struct MasterDataService<'a> {
    database: &'a Database,
    session: &'a AuthSession,
}

impl<'a> MasterDataService<'a> {
    pub(crate) fn new(database: &'a Database, session: &'a AuthSession) -> Self {
        Self { database, session }
    }

    pub(crate) fn list_doctors(
        &self,
        request: MasterDataListRequest,
    ) -> Result<Vec<DoctorRecord>, MasterDataError> {
        self.require_admin()?;
        let search = normalize_search(request.search);
        Ok(repository::list_doctors(
            &self.database.open()?,
            search.as_deref(),
        )?)
    }

    pub(crate) fn create_doctor(
        &self,
        input: DoctorInput,
    ) -> Result<DoctorRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let (code, name) = validate_doctor(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_doctor_code_available(&transaction, code.as_deref(), None)?;
        let doctor_id = repository::insert_doctor(&transaction, code.as_deref(), &name)?;
        audit::append_event(
            &transaction,
            Some(actor.id),
            "doctor_created",
            "doctor",
            doctor_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_doctor(&connection, doctor_id)?.ok_or(MasterDataError::DoctorNotFound)
    }

    pub(crate) fn update_doctor(
        &self,
        doctor_id: i64,
        input: DoctorInput,
    ) -> Result<DoctorRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let (code, name) = validate_doctor(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::load_doctor(&transaction, doctor_id)?.is_none() {
            return Err(MasterDataError::DoctorNotFound);
        }
        ensure_doctor_code_available(&transaction, code.as_deref(), Some(doctor_id))?;
        if repository::update_doctor(&transaction, doctor_id, code.as_deref(), &name)? != 1 {
            return Err(MasterDataError::DoctorNotFound);
        }
        audit::append_event(
            &transaction,
            Some(actor.id),
            "doctor_updated",
            "doctor",
            doctor_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_doctor(&connection, doctor_id)?.ok_or(MasterDataError::DoctorNotFound)
    }

    pub(crate) fn list_wards(
        &self,
        request: MasterDataListRequest,
    ) -> Result<Vec<WardRecord>, MasterDataError> {
        self.require_admin()?;
        let search = normalize_search(request.search);
        Ok(repository::list_wards(
            &self.database.open()?,
            search.as_deref(),
        )?)
    }

    pub(crate) fn create_ward(&self, input: WardInput) -> Result<WardRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let (code, name, telephone) = validate_ward(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_ward_code_available(&transaction, code.as_deref(), None)?;
        let ward_id =
            repository::insert_ward(&transaction, code.as_deref(), &name, telephone.as_deref())?;
        audit::append_event(
            &transaction,
            Some(actor.id),
            "ward_created",
            "ward",
            ward_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_ward(&connection, ward_id)?.ok_or(MasterDataError::WardNotFound)
    }

    pub(crate) fn update_ward(
        &self,
        ward_id: i64,
        input: WardInput,
    ) -> Result<WardRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let (code, name, telephone) = validate_ward(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::load_ward(&transaction, ward_id)?.is_none() {
            return Err(MasterDataError::WardNotFound);
        }
        ensure_ward_code_available(&transaction, code.as_deref(), Some(ward_id))?;
        if repository::update_ward(
            &transaction,
            ward_id,
            code.as_deref(),
            &name,
            telephone.as_deref(),
        )? != 1
        {
            return Err(MasterDataError::WardNotFound);
        }
        audit::append_event(
            &transaction,
            Some(actor.id),
            "ward_updated",
            "ward",
            ward_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_ward(&connection, ward_id)?.ok_or(MasterDataError::WardNotFound)
    }

    pub(crate) fn list_routes(
        &self,
        request: MasterDataListRequest,
    ) -> Result<Vec<RouteRecord>, MasterDataError> {
        self.require_admin()?;
        let search = normalize_search(request.search);
        Ok(repository::list_routes(
            &self.database.open()?,
            search.as_deref(),
        )?)
    }

    pub(crate) fn create_route(&self, input: RouteInput) -> Result<RouteRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let (code, name) = validate_route(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_route_code_available(&transaction, code.as_deref(), None)?;
        let route_id = repository::insert_route(&transaction, code.as_deref(), &name)?;
        audit::append_event(
            &transaction,
            Some(actor.id),
            "route_created",
            "route",
            route_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_route(&connection, route_id)?.ok_or(MasterDataError::RouteNotFound)
    }

    pub(crate) fn update_route(
        &self,
        route_id: i64,
        input: RouteInput,
    ) -> Result<RouteRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let (code, name) = validate_route(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::load_route(&transaction, route_id)?.is_none() {
            return Err(MasterDataError::RouteNotFound);
        }
        ensure_route_code_available(&transaction, code.as_deref(), Some(route_id))?;
        if repository::update_route(&transaction, route_id, code.as_deref(), &name)? != 1 {
            return Err(MasterDataError::RouteNotFound);
        }
        audit::append_event(
            &transaction,
            Some(actor.id),
            "route_updated",
            "route",
            route_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_route(&connection, route_id)?.ok_or(MasterDataError::RouteNotFound)
    }

    pub(crate) fn list_diluents(
        &self,
        request: MasterDataListRequest,
    ) -> Result<Vec<DiluentRecord>, MasterDataError> {
        self.require_admin()?;
        let search = normalize_search(request.search);
        Ok(repository::list_diluents(
            &self.database.open()?,
            search.as_deref(),
        )?)
    }

    pub(crate) fn create_diluent(
        &self,
        input: DiluentInput,
    ) -> Result<DiluentRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let (code, name, volume_ml) = validate_diluent(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_diluent_code_available(&transaction, code.as_deref(), None)?;
        let diluent_id =
            repository::insert_diluent(&transaction, code.as_deref(), &name, volume_ml)?;
        audit::append_event(
            &transaction,
            Some(actor.id),
            "diluent_created",
            "diluent",
            diluent_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_diluent(&connection, diluent_id)?.ok_or(MasterDataError::DiluentNotFound)
    }

    pub(crate) fn update_diluent(
        &self,
        diluent_id: i64,
        input: DiluentInput,
    ) -> Result<DiluentRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let (code, name, volume_ml) = validate_diluent(input)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::load_diluent(&transaction, diluent_id)?.is_none() {
            return Err(MasterDataError::DiluentNotFound);
        }
        ensure_diluent_code_available(&transaction, code.as_deref(), Some(diluent_id))?;
        if repository::update_diluent(&transaction, diluent_id, code.as_deref(), &name, volume_ml)?
            != 1
        {
            return Err(MasterDataError::DiluentNotFound);
        }
        audit::append_event(
            &transaction,
            Some(actor.id),
            "diluent_updated",
            "diluent",
            diluent_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_diluent(&connection, diluent_id)?.ok_or(MasterDataError::DiluentNotFound)
    }

    pub(crate) fn list_diagnoses(
        &self,
        request: MasterDataListRequest,
    ) -> Result<Vec<DiagnosisRecord>, MasterDataError> {
        self.require_admin()?;
        let search = normalize_search(request.search);
        Ok(repository::list_diagnoses(
            &self.database.open()?,
            search.as_deref(),
        )?)
    }

    pub(crate) fn create_diagnosis(
        &self,
        input: DiagnosisInput,
    ) -> Result<DiagnosisRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let name = validate_name(input.name)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let diagnosis_id = repository::insert_diagnosis(&transaction, &name)?;
        audit::append_event(
            &transaction,
            Some(actor.id),
            "diagnosis_created",
            "diagnosis",
            diagnosis_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_diagnosis(&connection, diagnosis_id)?
            .ok_or(MasterDataError::DiagnosisNotFound)
    }

    pub(crate) fn update_diagnosis(
        &self,
        diagnosis_id: i64,
        input: DiagnosisInput,
    ) -> Result<DiagnosisRecord, MasterDataError> {
        let actor = self.require_admin()?;
        let name = validate_name(input.name)?;
        let mut connection = self.database.open()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if repository::load_diagnosis(&transaction, diagnosis_id)?.is_none() {
            return Err(MasterDataError::DiagnosisNotFound);
        }
        if repository::update_diagnosis(&transaction, diagnosis_id, &name)? != 1 {
            return Err(MasterDataError::DiagnosisNotFound);
        }
        audit::append_event(
            &transaction,
            Some(actor.id),
            "diagnosis_updated",
            "diagnosis",
            diagnosis_id,
            &json!({}),
        )?;
        transaction.commit()?;
        repository::load_diagnosis(&connection, diagnosis_id)?
            .ok_or(MasterDataError::DiagnosisNotFound)
    }

    fn require_admin(&self) -> Result<crate::auth::CurrentUser, MasterDataError> {
        Ok(AuthService::new(self.database, self.session).require_admin()?)
    }
}

fn validate_doctor(input: DoctorInput) -> Result<(Option<String>, String), MasterDataError> {
    Ok((
        normalize_code(input.legacy_code)?,
        validate_name(input.name)?,
    ))
}

fn validate_ward(
    input: WardInput,
) -> Result<(Option<String>, String, Option<String>), MasterDataError> {
    let telephone = normalize_optional(input.telephone);
    if telephone
        .as_ref()
        .is_some_and(|value| value.chars().count() > TELEPHONE_MAX_CHARS)
    {
        return Err(validation(
            "telephone",
            "Telephone is limited to 100 characters",
        ));
    }
    Ok((
        normalize_code(input.legacy_code)?,
        validate_name(input.name)?,
        telephone,
    ))
}

fn validate_route(input: RouteInput) -> Result<(Option<String>, String), MasterDataError> {
    Ok((
        normalize_code(input.legacy_code)?,
        validate_name(input.name)?,
    ))
}

fn validate_diluent(
    input: DiluentInput,
) -> Result<(Option<String>, String, Option<f64>), MasterDataError> {
    if input
        .volume_ml
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(validation(
            "volumeMl",
            "Volume must be a finite number greater than or equal to zero",
        ));
    }
    Ok((
        normalize_code(input.legacy_code)?,
        validate_name(input.name)?,
        input.volume_ml,
    ))
}

fn validate_name(value: String) -> Result<String, MasterDataError> {
    let value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > NAME_MAX_CHARS {
        return Err(validation(
            "name",
            "Name is required and limited to 200 characters",
        ));
    }
    Ok(value)
}

fn normalize_code(value: Option<String>) -> Result<Option<String>, MasterDataError> {
    let value = normalize_optional(value);
    if value
        .as_ref()
        .is_some_and(|value| value.chars().count() > CODE_MAX_CHARS)
    {
        return Err(validation("legacyCode", "Code is limited to 50 characters"));
    }
    Ok(value)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn normalize_search(value: Option<String>) -> Option<String> {
    normalize_optional(value)
}

fn ensure_doctor_code_available(
    connection: &rusqlite::Connection,
    code: Option<&str>,
    excluding_id: Option<i64>,
) -> Result<(), MasterDataError> {
    if let Some(code) = code {
        if repository::doctor_code_exists(connection, code, excluding_id)? {
            return Err(validation("legacyCode", "Doctor code is already in use"));
        }
    }
    Ok(())
}

fn ensure_ward_code_available(
    connection: &rusqlite::Connection,
    code: Option<&str>,
    excluding_id: Option<i64>,
) -> Result<(), MasterDataError> {
    if let Some(code) = code {
        if repository::ward_code_exists(connection, code, excluding_id)? {
            return Err(validation("legacyCode", "Ward code is already in use"));
        }
    }
    Ok(())
}

fn ensure_route_code_available(
    connection: &rusqlite::Connection,
    code: Option<&str>,
    excluding_id: Option<i64>,
) -> Result<(), MasterDataError> {
    if let Some(code) = code {
        if repository::route_code_exists(connection, code, excluding_id)? {
            return Err(validation("legacyCode", "Route code is already in use"));
        }
    }
    Ok(())
}

fn ensure_diluent_code_available(
    connection: &rusqlite::Connection,
    code: Option<&str>,
    excluding_id: Option<i64>,
) -> Result<(), MasterDataError> {
    if let Some(code) = code {
        if repository::diluent_code_exists(connection, code, excluding_id)? {
            return Err(validation("legacyCode", "Diluent code is already in use"));
        }
    }
    Ok(())
}

fn validation(field: &'static str, message: impl Into<String>) -> MasterDataError {
    MasterDataError::Validation {
        field,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthService, BootstrapUserInput, CreateUserInput, LoginInput, UserType};

    const ADMIN_PASSWORD: &str = "synthetic administrator password 42!";
    const USER_PASSWORD: &str = "synthetic local user password 84!";

    struct Fixture {
        _directory: tempfile::TempDir,
        database: Database,
        session: AuthSession,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database = Database::initialize(directory.path().join("oncoflow.db")).unwrap();
            let session = AuthSession::default();
            AuthService::new(&database, &session)
                .bootstrap(BootstrapUserInput {
                    username: "local.admin".into(),
                    display_name: "ผู้ดูแลทดสอบ".into(),
                    password: ADMIN_PASSWORD.into(),
                })
                .unwrap();
            Self {
                _directory: directory,
                database,
                session,
            }
        }

        fn service(&self) -> MasterDataService<'_> {
            MasterDataService::new(&self.database, &self.session)
        }
    }

    #[test]
    fn creates_searches_and_updates_thai_doctors_and_wards() {
        let fixture = Fixture::new();
        let doctor = fixture
            .service()
            .create_doctor(DoctorInput {
                legacy_code: Some("DOC-SYN".into()),
                name: "นพ. ทดสอบ ระบบ".into(),
            })
            .unwrap();
        let ward = fixture
            .service()
            .create_ward(WardInput {
                legacy_code: Some("WARD-SYN".into()),
                name: "หอผู้ป่วยทดสอบ".into(),
                telephone: Some("1234".into()),
            })
            .unwrap();
        assert_eq!(
            fixture
                .service()
                .list_doctors(MasterDataListRequest {
                    search: Some("ทดสอบ".into())
                })
                .unwrap(),
            vec![doctor.clone()]
        );
        assert_eq!(
            fixture
                .service()
                .list_wards(MasterDataListRequest {
                    search: Some("ผู้ป่วย".into())
                })
                .unwrap(),
            vec![ward.clone()]
        );

        let doctor = fixture
            .service()
            .update_doctor(
                doctor.id,
                DoctorInput {
                    legacy_code: doctor.legacy_code,
                    name: "พญ. ทดสอบ แก้ไข".into(),
                },
            )
            .unwrap();
        let ward = fixture
            .service()
            .update_ward(
                ward.id,
                WardInput {
                    legacy_code: ward.legacy_code,
                    name: "หอผู้ป่วยแก้ไข".into(),
                    telephone: None,
                },
            )
            .unwrap();
        assert_eq!(doctor.name, "พญ. ทดสอบ แก้ไข");
        assert_eq!(ward.name, "หอผู้ป่วยแก้ไข");
        assert_eq!(ward.telephone, None);
    }

    #[test]
    fn rejects_blank_names_and_duplicate_legacy_codes() {
        let fixture = Fixture::new();
        fixture
            .service()
            .create_doctor(DoctorInput {
                legacy_code: Some("DOC-1".into()),
                name: "Synthetic Doctor".into(),
            })
            .unwrap();
        assert!(matches!(
            fixture.service().create_doctor(DoctorInput {
                legacy_code: Some("doc-1".into()),
                name: "Other Doctor".into(),
            }),
            Err(MasterDataError::Validation {
                field: "legacyCode",
                ..
            })
        ));
        assert!(matches!(
            fixture.service().create_ward(WardInput {
                legacy_code: None,
                name: "  ".into(),
                telephone: None,
            }),
            Err(MasterDataError::Validation { field: "name", .. })
        ));
    }

    #[test]
    fn creates_searches_and_updates_thai_routes_and_diluents() {
        let fixture = Fixture::new();
        let route = fixture
            .service()
            .create_route(RouteInput {
                legacy_code: Some("R-SYN".into()),
                name: "ให้ทางหลอดเลือดดำ".into(),
            })
            .unwrap();
        let diluent = fixture
            .service()
            .create_diluent(DiluentInput {
                legacy_code: Some("D-SYN".into()),
                name: "สารละลายทดสอบ".into(),
                volume_ml: Some(100.5),
            })
            .unwrap();

        assert_eq!(
            fixture
                .service()
                .list_routes(MasterDataListRequest {
                    search: Some("หลอดเลือด".into()),
                })
                .unwrap(),
            vec![route.clone()]
        );
        assert_eq!(
            fixture
                .service()
                .list_diluents(MasterDataListRequest {
                    search: Some("ละลาย".into()),
                })
                .unwrap(),
            vec![diluent.clone()]
        );

        let route = fixture
            .service()
            .update_route(
                route.id,
                RouteInput {
                    legacy_code: route.legacy_code,
                    name: "เส้นทางทดสอบแก้ไข".into(),
                },
            )
            .unwrap();
        let diluent = fixture
            .service()
            .update_diluent(
                diluent.id,
                DiluentInput {
                    legacy_code: diluent.legacy_code,
                    name: "ตัวทำละลายแก้ไข".into(),
                    volume_ml: None,
                },
            )
            .unwrap();
        assert_eq!(route.name, "เส้นทางทดสอบแก้ไข");
        assert_eq!(diluent.name, "ตัวทำละลายแก้ไข");
        assert_eq!(diluent.volume_ml, None);
    }

    #[test]
    fn rejects_invalid_diluent_volume_and_duplicate_route_code() {
        let fixture = Fixture::new();
        fixture
            .service()
            .create_route(RouteInput {
                legacy_code: Some("ROUTE-1".into()),
                name: "Synthetic Route".into(),
            })
            .unwrap();
        assert!(matches!(
            fixture.service().create_route(RouteInput {
                legacy_code: Some("route-1".into()),
                name: "Other Route".into(),
            }),
            Err(MasterDataError::Validation {
                field: "legacyCode",
                ..
            })
        ));
        assert!(matches!(
            fixture.service().create_diluent(DiluentInput {
                legacy_code: None,
                name: "Synthetic Diluent".into(),
                volume_ml: Some(-0.1),
            }),
            Err(MasterDataError::Validation {
                field: "volumeMl",
                ..
            })
        ));
        assert!(matches!(
            fixture.service().create_diluent(DiluentInput {
                legacy_code: None,
                name: "Synthetic Diluent".into(),
                volume_ml: Some(f64::NAN),
            }),
            Err(MasterDataError::Validation {
                field: "volumeMl",
                ..
            })
        ));
    }

    #[test]
    fn manages_thai_diagnosis_names_without_touching_legacy_fields() {
        let fixture = Fixture::new();
        let connection = fixture.database.open().unwrap();
        connection
            .execute(
                "INSERT INTO diagnoses(legacy_diagcode,diagnosis,warning1,warning2)
                 VALUES('DX-KEEP','มะเร็งทดสอบ','legacy-one','legacy-two')",
                [],
            )
            .unwrap();
        let diagnosis_id = connection.last_insert_rowid();
        drop(connection);

        let listed = fixture
            .service()
            .list_diagnoses(MasterDataListRequest {
                search: Some("มะเร็ง".into()),
            })
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "มะเร็งทดสอบ");
        let updated = fixture
            .service()
            .update_diagnosis(
                diagnosis_id,
                DiagnosisInput {
                    name: "มะเร็งทดสอบแก้ไข".into(),
                },
            )
            .unwrap();
        assert_eq!(updated.name, "มะเร็งทดสอบแก้ไข");

        let preserved: (String, String, String) = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT legacy_diagcode,warning1,warning2 FROM diagnoses WHERE id=?1",
                [diagnosis_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            preserved,
            ("DX-KEEP".into(), "legacy-one".into(), "legacy-two".into())
        );

        let created = fixture
            .service()
            .create_diagnosis(DiagnosisInput {
                name: "วินิจฉัยใหม่".into(),
            })
            .unwrap();
        let legacy_code: Option<String> = fixture
            .database
            .open()
            .unwrap()
            .query_row(
                "SELECT legacy_diagcode FROM diagnoses WHERE id=?1",
                [created.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_code, None);
    }

    #[test]
    fn diagnosis_create_and_audit_are_atomic() {
        let fixture = Fixture::new();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_diagnosis_audit_failure BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='diagnosis_created'
                 BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().create_diagnosis(DiagnosisInput {
                name: "Synthetic Private Diagnosis".into(),
            }),
            Err(MasterDataError::Sqlite(_))
        ));
        assert_eq!(
            fixture
                .database
                .open()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM diagnoses WHERE diagnosis='Synthetic Private Diagnosis'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn diluent_create_and_audit_are_atomic() {
        let fixture = Fixture::new();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_diluent_audit_failure BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='diluent_created'
                 BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().create_diluent(DiluentInput {
                legacy_code: None,
                name: "Synthetic Private Diluent".into(),
                volume_ml: Some(100.0),
            }),
            Err(MasterDataError::Sqlite(_))
        ));
        assert_eq!(
            fixture
                .database
                .open()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM diluents WHERE diluent_name='Synthetic Private Diluent'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn non_administrator_cannot_manage_master_data() {
        let fixture = Fixture::new();
        AuthService::new(&fixture.database, &fixture.session)
            .create_user(CreateUserInput {
                username: "local.user".into(),
                display_name: "Local User".into(),
                password: USER_PASSWORD.into(),
                user_type: UserType::NonPharmacist,
            })
            .unwrap();
        AuthService::new(&fixture.database, &fixture.session)
            .logout()
            .unwrap();
        AuthService::new(&fixture.database, &fixture.session)
            .login(LoginInput {
                username: "local.user".into(),
                password: USER_PASSWORD.into(),
            })
            .unwrap();
        assert!(matches!(
            fixture
                .service()
                .list_doctors(MasterDataListRequest::default()),
            Err(MasterDataError::Auth(AuthError::AdminRequired))
        ));
        assert!(matches!(
            fixture
                .service()
                .list_diagnoses(MasterDataListRequest::default()),
            Err(MasterDataError::Auth(AuthError::AdminRequired))
        ));
    }

    #[test]
    fn create_and_audit_event_are_atomic_and_privacy_safe() {
        let fixture = Fixture::new();
        fixture
            .database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER synthetic_doctor_audit_failure BEFORE INSERT ON audit_events
                 WHEN NEW.event_type='doctor_created'
                 BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            fixture.service().create_doctor(DoctorInput {
                legacy_code: Some("ROLLBACK".into()),
                name: "Synthetic Private Doctor".into(),
            }),
            Err(MasterDataError::Sqlite(_))
        ));
        let connection = fixture.database.open().unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM doctors WHERE legacy_doccode='ROLLBACK'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        let metadata: String = connection
            .query_row(
                "SELECT GROUP_CONCAT(metadata_json,'|') FROM audit_events",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!metadata.contains("Synthetic Private Doctor"));
    }
}
