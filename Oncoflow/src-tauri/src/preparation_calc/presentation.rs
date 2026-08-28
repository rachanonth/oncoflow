use crate::clinical::decimal::{DecimalParse, LegacyDecimal};

pub(super) fn parse_decimal(value: Option<&str>) -> Result<Option<LegacyDecimal>, &'static str> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match LegacyDecimal::parse_access_subset(value) {
        DecimalParse::Parsed(value) => Ok(Some(value)),
        DecimalParse::NotNumeric => Err("The value is not a numeric Access-compatible value."),
        DecimalParse::Unsupported => {
            Err("The value is malformed, locale-sensitive, or outside the fixed-point range.")
        }
    }
}

pub(super) fn cleaned_label(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn units_compatible(ordered: Option<&str>, presentation: Option<&str>) -> bool {
    let Some(ordered) = ordered.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let Some(presentation) = presentation
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    ordered.eq_ignore_ascii_case(presentation)
}
