use std::cmp::Ordering;

const MAX_SCALE: u32 = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LegacyDecimal {
    coefficient: i128,
    scale: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecimalParse {
    Parsed(LegacyDecimal),
    NotNumeric,
    Unsupported,
}

impl LegacyDecimal {
    pub(crate) const ZERO: Self = Self {
        coefficient: 0,
        scale: 0,
    };

    pub(crate) fn parse_access_subset(value: &str) -> DecimalParse {
        let value = value.trim();
        if value.is_empty() {
            return DecimalParse::NotNumeric;
        }
        if is_locale_sensitive_numeric_form(value) {
            return DecimalParse::Unsupported;
        }

        match Self::parse_invariant(value) {
            Some(value) => DecimalParse::Parsed(value),
            None if value.chars().any(|character| character.is_ascii_digit()) => {
                DecimalParse::Unsupported
            }
            None => DecimalParse::NotNumeric,
        }
    }

    fn parse_invariant(value: &str) -> Option<Self> {
        let (mantissa, exponent) = split_exponent(value)?;
        let (negative, mantissa) = match mantissa.as_bytes().first() {
            Some(b'-') => (true, &mantissa[1..]),
            Some(b'+') => (false, &mantissa[1..]),
            _ => (false, mantissa),
        };
        if mantissa.is_empty() {
            return None;
        }
        let mut separator_seen = false;
        let mut fraction_digits = 0_u32;
        let mut digit_count = 0_u32;
        let mut coefficient = 0_i128;
        for character in mantissa.chars() {
            if character == '.' && !separator_seen {
                separator_seen = true;
                continue;
            }
            let digit = character.to_digit(10)?;
            if !character.is_ascii_digit() {
                return None;
            }
            coefficient = coefficient
                .checked_mul(10)?
                .checked_add(i128::from(digit))?;
            digit_count += 1;
            if separator_seen {
                fraction_digits += 1;
            }
        }
        if digit_count == 0 {
            return None;
        }
        if negative {
            coefficient = coefficient.checked_neg()?;
        }

        let target_scale = i64::from(fraction_digits) - i64::from(exponent);
        let value = if target_scale < 0 {
            let power = u32::try_from(-target_scale).ok()?;
            let coefficient = coefficient.checked_mul(power_of_ten(power)?)?;
            Self {
                coefficient,
                scale: 0,
            }
        } else {
            let scale = u32::try_from(target_scale).ok()?;
            if scale > MAX_SCALE {
                return None;
            }
            Self { coefficient, scale }
        };
        Some(value.normalized())
    }

    pub(crate) fn checked_mul(self, other: Self) -> Option<Self> {
        let scale = self.scale.checked_add(other.scale)?;
        if scale > MAX_SCALE {
            return None;
        }
        Some(
            Self {
                coefficient: self.coefficient.checked_mul(other.coefficient)?,
                scale,
            }
            .normalized(),
        )
    }

    pub(crate) fn checked_add(self, other: Self) -> Option<Self> {
        let scale = self.scale.max(other.scale);
        let left = self
            .coefficient
            .checked_mul(power_of_ten(scale - self.scale)?)?;
        let right = other
            .coefficient
            .checked_mul(power_of_ten(scale - other.scale)?)?;
        Some(
            Self {
                coefficient: left.checked_add(right)?,
                scale,
            }
            .normalized(),
        )
    }

    pub(crate) fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_add(Self {
            coefficient: other.coefficient.checked_neg()?,
            scale: other.scale,
        })
    }

    pub(crate) fn checked_mul_integer(self, multiplier: i128) -> Option<Self> {
        Some(
            Self {
                coefficient: self.coefficient.checked_mul(multiplier)?,
                scale: self.scale,
            }
            .normalized(),
        )
    }

    /// Returns an exact terminating decimal quotient for non-negative inputs.
    /// A non-terminating base-10 result is deliberately unsupported rather
    /// than rounded under an unverified compatibility rule.
    pub(crate) fn checked_div_exact_nonnegative(self, divisor: Self) -> Option<Self> {
        let (mut numerator, mut denominator) = self.ratio_parts_nonnegative(divisor)?;
        if numerator == 0 {
            return Some(Self::ZERO);
        }

        let mut twos = 0_u32;
        while denominator % 2 == 0 {
            denominator /= 2;
            twos += 1;
        }
        let mut fives = 0_u32;
        while denominator % 5 == 0 {
            denominator /= 5;
            fives += 1;
        }
        if denominator != 1 {
            return None;
        }

        let scale = twos.max(fives);
        if scale > MAX_SCALE {
            return None;
        }
        numerator = numerator.checked_mul(2_i128.checked_pow(scale - twos)?)?;
        numerator = numerator.checked_mul(5_i128.checked_pow(scale - fives)?)?;
        Some(
            Self {
                coefficient: numerator,
                scale,
            }
            .normalized(),
        )
    }

    /// Divides non-negative values and rounds to the requested decimal scale.
    /// Midpoints round upward because preparation withdrawal volumes are never negative.
    pub(crate) fn checked_div_round_half_up_nonnegative(
        self,
        divisor: Self,
        scale: u32,
    ) -> Option<Self> {
        if scale > MAX_SCALE {
            return None;
        }
        let (numerator, denominator) = self.ratio_parts_nonnegative(divisor)?;
        let scaled_numerator = numerator.checked_mul(power_of_ten(scale)?)?;
        let quotient = scaled_numerator / denominator;
        let remainder = scaled_numerator % denominator;
        let coefficient = if remainder.checked_mul(2)? >= denominator {
            quotient.checked_add(1)?
        } else {
            quotient
        };
        Some(Self { coefficient, scale }.normalized())
    }

    /// Calculates ceil(self / divisor) without first rounding the quotient.
    pub(crate) fn checked_ceil_ratio_nonnegative(self, divisor: Self) -> Option<i128> {
        let (numerator, denominator) = self.ratio_parts_nonnegative(divisor)?;
        let quotient = numerator / denominator;
        if numerator % denominator == 0 {
            Some(quotient)
        } else {
            quotient.checked_add(1)
        }
    }

    pub(crate) fn compare_decimal(self, other: Self) -> Option<Ordering> {
        let scale = self.scale.max(other.scale);
        let left = self
            .coefficient
            .checked_mul(power_of_ten(scale - self.scale)?)?;
        let right = other
            .coefficient
            .checked_mul(power_of_ten(scale - other.scale)?)?;
        Some(left.cmp(&right))
    }

    pub(crate) fn divide_by_power_of_ten(self, power: u32) -> Option<Self> {
        let scale = self.scale.checked_add(power)?;
        if scale > MAX_SCALE {
            return None;
        }
        Some(
            Self {
                coefficient: self.coefficient,
                scale,
            }
            .normalized(),
        )
    }

    pub(crate) fn is_zero(self) -> bool {
        self.coefficient == 0
    }

    pub(crate) fn compare_integer(self, integer: i128) -> Option<Ordering> {
        let scaled = integer.checked_mul(power_of_ten(self.scale)?)?;
        Some(self.coefficient.cmp(&scaled))
    }

    pub(crate) fn floor(self) -> Option<i128> {
        let divisor = power_of_ten(self.scale)?;
        Some(self.coefficient.div_euclid(divisor))
    }

    pub(crate) fn ceil(self) -> Option<i128> {
        let divisor = power_of_ten(self.scale)?;
        let floor = self.coefficient.div_euclid(divisor);
        let remainder = self.coefficient.rem_euclid(divisor);
        if remainder == 0 {
            Some(floor)
        } else {
            floor.checked_add(1)
        }
    }

    pub(crate) fn round_half_even_i16(self) -> Option<i16> {
        i16::try_from(self.round_half_even()?).ok()
    }

    pub(crate) fn round_half_even_i32(self) -> Option<i32> {
        i32::try_from(self.round_half_even()?).ok()
    }

    fn round_half_even(self) -> Option<i128> {
        let divisor = power_of_ten(self.scale)?;
        let absolute = self.coefficient.checked_abs()?;
        let quotient = absolute / divisor;
        let remainder = absolute % divisor;
        let doubled = remainder.checked_mul(2)?;
        let magnitude = match doubled.cmp(&divisor) {
            Ordering::Less => quotient,
            Ordering::Greater => quotient.checked_add(1)?,
            Ordering::Equal if quotient % 2 == 0 => quotient,
            Ordering::Equal => quotient.checked_add(1)?,
        };
        if self.coefficient.is_negative() {
            magnitude.checked_neg()
        } else {
            Some(magnitude)
        }
    }

    pub(crate) fn invariant_string(self) -> Option<String> {
        let value = self.normalized();
        if value.scale == 0 {
            return Some(value.coefficient.to_string());
        }
        let negative = value.coefficient.is_negative();
        let digits = value.coefficient.checked_abs()?.to_string();
        let scale = usize::try_from(value.scale).ok()?;
        let body = if digits.len() <= scale {
            format!("0.{}{}", "0".repeat(scale - digits.len()), digits)
        } else {
            let split = digits.len() - scale;
            format!("{}.{}", &digits[..split], &digits[split..])
        };
        if negative {
            Some(format!("-{body}"))
        } else {
            Some(body)
        }
    }

    fn normalized(mut self) -> Self {
        while self.scale > 0 && self.coefficient % 10 == 0 {
            self.coefficient /= 10;
            self.scale -= 1;
        }
        self
    }

    fn ratio_parts_nonnegative(self, divisor: Self) -> Option<(i128, i128)> {
        if self.coefficient < 0 || divisor.coefficient <= 0 {
            return None;
        }
        if self.coefficient == 0 {
            return Some((0, 1));
        }

        let mut numerator = self.coefficient;
        let mut denominator = divisor.coefficient;
        if divisor.scale >= self.scale {
            let mut scale_factor = power_of_ten(divisor.scale - self.scale)?;
            let cancellation = gcd(denominator, scale_factor);
            denominator /= cancellation;
            scale_factor /= cancellation;
            numerator = numerator.checked_mul(scale_factor)?;
        } else {
            let mut scale_factor = power_of_ten(self.scale - divisor.scale)?;
            let cancellation = gcd(numerator, scale_factor);
            numerator /= cancellation;
            scale_factor /= cancellation;
            denominator = denominator.checked_mul(scale_factor)?;
        }
        let cancellation = gcd(numerator, denominator);
        Some((numerator / cancellation, denominator / cancellation))
    }
}

pub(super) fn is_locale_sensitive_numeric_form(value: &str) -> bool {
    let value = value.trim();
    value.contains(',')
        || value.contains('$')
        || value.contains('/')
        || value.contains(':')
        || value.starts_with("&H")
        || value.starts_with("&h")
        || value.starts_with("&O")
        || value.starts_with("&o")
}

fn split_exponent(value: &str) -> Option<(&str, i32)> {
    let mut parts = value.split(['e', 'E']);
    let mantissa = parts.next()?;
    let exponent = match parts.next() {
        Some(value) if !value.is_empty() => value.parse::<i32>().ok()?,
        Some(_) => return None,
        None => 0,
    };
    if parts.next().is_some() || !(-38..=38).contains(&exponent) {
        return None;
    }
    Some((mantissa, exponent))
}

fn power_of_ten(power: u32) -> Option<i128> {
    10_i128.checked_pow(power)
}

fn gcd(mut left: i128, mut right: i128) -> i128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.abs().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decimal(value: &str) -> LegacyDecimal {
        match LegacyDecimal::parse_access_subset(value) {
            DecimalParse::Parsed(value) => value,
            other => panic!("expected decimal, got {other:?}"),
        }
    }

    #[test]
    fn parses_and_formats_invariant_decimals_and_exponents() {
        for (input, expected) in [
            ("0", "0"),
            ("-0.2500", "-0.25"),
            (".5", "0.5"),
            ("5.", "5"),
            ("1.25e2", "125"),
            ("1e-3", "0.001"),
        ] {
            assert_eq!(decimal(input).invariant_string().as_deref(), Some(expected));
        }
    }

    #[test]
    fn implements_floor_ceiling_and_midpoint_to_even() {
        assert_eq!(decimal("1.5").floor(), Some(1));
        assert_eq!(decimal("-1.5").floor(), Some(-2));
        assert_eq!(decimal("1.0001").ceil(), Some(2));
        assert_eq!(decimal("-1.5").ceil(), Some(-1));
        assert_eq!(decimal("2.5").round_half_even_i16(), Some(2));
        assert_eq!(decimal("3.5").round_half_even_i16(), Some(4));
        assert_eq!(decimal("-2.5").round_half_even_i16(), Some(-2));
        assert_eq!(decimal("-3.5").round_half_even_i16(), Some(-4));
    }

    #[test]
    fn rejects_locale_sensitive_or_overflowing_forms() {
        assert_eq!(
            LegacyDecimal::parse_access_subset("1,000"),
            DecimalParse::Unsupported
        );
        assert_eq!(
            LegacyDecimal::parse_access_subset("$1"),
            DecimalParse::Unsupported
        );
        assert_eq!(
            LegacyDecimal::parse_access_subset("not numeric"),
            DecimalParse::NotNumeric
        );
        assert_eq!(decimal("32767.5").round_half_even_i16(), None);
    }

    #[test]
    fn exact_division_and_ratio_ceiling_do_not_use_floating_point() {
        assert_eq!(
            decimal("75")
                .checked_div_exact_nonnegative(decimal("50"))
                .and_then(LegacyDecimal::invariant_string)
                .as_deref(),
            Some("1.5")
        );
        assert_eq!(
            decimal("1").checked_div_exact_nonnegative(decimal("3")),
            None
        );
        assert_eq!(
            decimal("100.0001").checked_ceil_ratio_nonnegative(decimal("100")),
            Some(2)
        );
        assert_eq!(
            decimal("200").checked_ceil_ratio_nonnegative(decimal("100")),
            Some(2)
        );
    }

    #[test]
    fn one_decimal_division_rounds_nonnegative_midpoints_up_without_floating_point() {
        for (numerator, denominator, expected) in [
            ("1", "3", "0.3"),
            ("2", "3", "0.7"),
            ("1", "4", "0.3"),
            ("3", "4", "0.8"),
            ("15", "1", "15"),
        ] {
            assert_eq!(
                decimal(numerator)
                    .checked_div_round_half_up_nonnegative(decimal(denominator), 1)
                    .and_then(LegacyDecimal::invariant_string)
                    .as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn checked_subtraction_and_integer_multiplication_are_exact() {
        let unused = decimal("50")
            .checked_mul_integer(2)
            .and_then(|opened| opened.checked_sub(decimal("75")))
            .and_then(LegacyDecimal::invariant_string);
        assert_eq!(unused.as_deref(), Some("25"));
    }
}
