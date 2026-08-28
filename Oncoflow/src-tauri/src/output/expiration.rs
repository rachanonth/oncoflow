pub(crate) fn expiration_at(print_time: &str, configured_duration: Option<&str>) -> Option<String> {
    let (year, month, day, hour, minute, second) = parse_datetime(print_time)?;
    let duration_seconds = parse_duration_seconds(configured_duration?)?;
    let day_seconds = i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
    let timestamp = days_from_civil(year, month, day)
        .checked_mul(86_400)?
        .checked_add(day_seconds)?
        .checked_add(duration_seconds)?;
    let days = timestamp.div_euclid(86_400);
    let seconds = timestamp.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days)?;
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        seconds / 3_600,
        (seconds % 3_600) / 60,
        seconds % 60
    ))
}

fn parse_datetime(value: &str) -> Option<(i64, u32, u32, u32, u32, u32)> {
    let bytes = value.as_bytes();
    if bytes.len() < 19 || !matches!(bytes.get(10), Some(b'T' | b' ')) {
        return None;
    }
    let year = value.get(0..4)?.parse().ok()?;
    let month = value.get(5..7)?.parse().ok()?;
    let day = value.get(8..10)?.parse().ok()?;
    let hour = value.get(11..13)?.parse().ok()?;
    let minute = value.get(14..16)?.parse().ok()?;
    let second = value.get(17..19)?.parse().ok()?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    Some((year, month, day, hour, minute, second))
}

fn parse_duration_seconds(value: &str) -> Option<i64> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    let clock = value.split(':').collect::<Vec<_>>();
    if matches!(clock.len(), 2 | 3) {
        let hours = clock[0].parse::<i64>().ok()?;
        let minutes = clock[1].parse::<i64>().ok()?;
        let seconds = if clock.len() == 3 {
            clock[2].parse::<i64>().ok()?
        } else {
            0
        };
        if hours < 0 || !(0..60).contains(&minutes) || !(0..60).contains(&seconds) {
            return None;
        }
        return hours
            .checked_mul(3_600)?
            .checked_add(minutes.checked_mul(60)?)?
            .checked_add(seconds);
    }

    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 2 {
        return None;
    }
    let amount = parts[0].parse::<f64>().ok()?;
    if !amount.is_finite() || amount < 0.0 {
        return None;
    }
    let multiplier = if parts[1].starts_with("min") {
        60.0
    } else if parts[1].starts_with("h") {
        3_600.0
    } else {
        return None;
    };
    let seconds = amount * multiplier;
    (seconds <= i64::MAX as f64).then(|| seconds.round() as i64)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let adjusted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> Option<(i64, u32, u32)> {
    let days = days.checked_add(719_468)?;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    Some((year, u32::try_from(month).ok()?, u32::try_from(day).ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_configured_clock_hour_and_minute_durations_across_dates() {
        assert_eq!(
            expiration_at("2026-08-27T22:30:00", Some("08:30:00")).as_deref(),
            Some("2026-08-28T07:00:00")
        );
        assert_eq!(
            expiration_at("2026-12-31T23:30:00", Some("2 hr")).as_deref(),
            Some("2027-01-01T01:30:00")
        );
        assert_eq!(
            expiration_at("2026-08-27T10:00:00", Some("90 min")).as_deref(),
            Some("2026-08-27T11:30:00")
        );
    }

    #[test]
    fn missing_or_invalid_duration_never_invents_an_expiration() {
        assert_eq!(expiration_at("2026-08-27T10:00:00", None), None);
        assert_eq!(
            expiration_at("2026-08-27T10:00:00", Some("overnight")),
            None
        );
        assert_eq!(expiration_at("invalid", Some("2 hr")), None);
    }
}
