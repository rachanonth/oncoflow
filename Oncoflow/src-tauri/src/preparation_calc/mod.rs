mod container;
mod model;
mod presentation;
mod quantity;
mod trace;

pub(crate) use model::*;
pub(crate) use quantity::calculate_preparation;

pub(crate) const PREPARATION_CALC_RULESET: &str = "legacy-cytotoxic-v8+withdrawal-1dp-v1";
pub(crate) const PREPARATION_CALC_RULE_ID: &str =
    "legacy-cytotoxic-v8:preparation-container-use-withdrawal-1dp";

#[cfg(test)]
mod tests;
