use super::CalculationTraceStep;

pub(super) fn step(
    step: &'static str,
    expression: impl Into<String>,
    result: Option<String>,
    confidence: &'static str,
) -> CalculationTraceStep {
    CalculationTraceStep {
        step,
        expression: expression.into(),
        result,
        confidence,
    }
}
