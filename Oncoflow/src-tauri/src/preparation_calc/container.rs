use crate::clinical::decimal::LegacyDecimal;

pub(super) fn containers_required(
    ordered_amount: LegacyDecimal,
    amount_per_container: LegacyDecimal,
) -> Option<i128> {
    ordered_amount.checked_ceil_ratio_nonnegative(amount_per_container)
}

pub(super) fn unused_amount(
    ordered_amount: LegacyDecimal,
    amount_per_container: LegacyDecimal,
    containers: i128,
) -> Option<LegacyDecimal> {
    amount_per_container
        .checked_mul_integer(containers)?
        .checked_sub(ordered_amount)
}
