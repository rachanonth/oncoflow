use sha2::{Digest, Sha256};

use super::{SafetyFinding, SafetyFindingStatus, SafetySeverity};

const FINGERPRINT_FORMAT: &str = "oncoflow-safety-finding-v1";

pub(crate) fn finding_fingerprint(finding: &SafetyFinding) -> String {
    let mut canonical = Vec::new();
    for value in [
        FINGERPRINT_FORMAT,
        finding.id.as_str(),
        finding.rule_id,
        finding.ruleset_version,
        severity_name(finding.severity),
        status_name(finding.status),
    ] {
        push_component(&mut canonical, value);
    }
    push_component(
        &mut canonical,
        &finding
            .order_item_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NULL".into()),
    );
    push_component(&mut canonical, &finding.evidence.len().to_string());
    for evidence in &finding.evidence {
        push_component(&mut canonical, &evidence.label);
        push_component(&mut canonical, &evidence.value);
    }
    let digest = Sha256::digest(canonical);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn push_component(target: &mut Vec<u8>, value: &str) {
    target.extend_from_slice(value.len().to_string().as_bytes());
    target.push(b':');
    target.extend_from_slice(value.as_bytes());
    target.push(b'|');
}

fn severity_name(value: SafetySeverity) -> &'static str {
    match value {
        SafetySeverity::Info => "info",
        SafetySeverity::Warning => "warning",
    }
}

fn status_name(value: SafetyFindingStatus) -> &'static str {
    match value {
        SafetyFindingStatus::Triggered => "triggered",
        SafetyFindingStatus::Advisory => "advisory",
        SafetyFindingStatus::Unsupported => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::{SafetyEvidence, SafetyFinding};

    fn finding(version: &'static str, observed: &str) -> SafetyFinding {
        let mut finding = SafetyFinding {
            id: "synthetic:item:1".into(),
            fingerprint: String::new(),
            rule_id: "legacy.synthetic",
            ruleset_version: version,
            severity: SafetySeverity::Warning,
            title: "Synthetic warning".into(),
            message: "Synthetic evidence only".into(),
            evidence: vec![SafetyEvidence {
                label: "Observed".into(),
                value: observed.into(),
            }],
            source: "synthetic",
            status: SafetyFindingStatus::Triggered,
            order_item_id: Some(1),
            acknowledgement_required: true,
        };
        finding.fingerprint = finding_fingerprint(&finding);
        finding
    }

    #[test]
    fn fingerprint_is_deterministic_and_contains_no_display_message() {
        let first = finding("legacy-cytotoxic-v8", "11");
        let mut second = finding("legacy-cytotoxic-v8", "11");
        second.title = "Changed display title".into();
        second.message = "Changed display message".into();
        assert_eq!(first.fingerprint, finding_fingerprint(&second));
        assert_eq!(first.fingerprint.len(), 64);
    }

    #[test]
    fn changed_evidence_or_ruleset_invalidates_a_fingerprint() {
        let original = finding("legacy-cytotoxic-v8", "11");
        let changed_input = finding("legacy-cytotoxic-v8", "12");
        let changed_ruleset = finding("legacy-cytotoxic-v9", "11");
        assert_ne!(original.fingerprint, changed_input.fingerprint);
        assert_ne!(original.fingerprint, changed_ruleset.fingerprint);
    }
}
