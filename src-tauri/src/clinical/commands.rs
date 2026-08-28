use super::{
    anc_cal, anc_grade, fix_number, lab_min_max, platelet, standard_dose, ClinicalCalculationResult,
};

#[tauri::command]
pub(crate) fn clinical_standard_dose(
    dose: Option<String>,
    surface: Option<String>,
) -> ClinicalCalculationResult<String> {
    standard_dose(dose.as_deref(), surface.as_deref())
}

#[tauri::command]
pub(crate) fn clinical_anc_cal(
    wbc: Option<String>,
    neutrophil: Option<String>,
) -> ClinicalCalculationResult<String> {
    anc_cal(wbc.as_deref(), neutrophil.as_deref())
}

#[tauri::command]
pub(crate) fn clinical_anc_grade(anc: Option<String>) -> ClinicalCalculationResult<String> {
    anc_grade(anc.as_deref())
}

#[tauri::command]
pub(crate) fn clinical_platelet(raw_value: Option<String>) -> ClinicalCalculationResult<String> {
    platelet(raw_value.as_deref())
}

#[tauri::command]
pub(crate) fn clinical_lab_min_max(number: Option<String>) -> ClinicalCalculationResult<String> {
    lab_min_max(number.as_deref())
}

#[tauri::command]
pub(crate) fn clinical_fix_number(number: Option<String>) -> ClinicalCalculationResult<String> {
    fix_number(number.as_deref())
}
