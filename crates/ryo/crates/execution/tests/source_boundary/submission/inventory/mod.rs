//! The source inventories every submission route is checked against, and the intents they enumerate.
use ryo_execution::inbound::use_cases::execution::types::*;
use ryo_values::domain::runs::types::Skipped;
use std::{collections::BTreeSet, fs, path::Path};

pub(super) const PROTOCOL: &str = "src/application/file_dag/program/protocol/types.rs";
pub(super) const ADMISSION: &str = "src/application/file_dag/program/admission.rs";
pub(super) fn identifier(text: &str) -> &str {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()
        .unwrap_or_default()
}
/// The variants a declaration lists, read from its source text.
pub(super) fn variants(source: &str, name: &str) -> BTreeSet<String> {
    let start = source
        .find(&format!("enum {name} {{"))
        .unwrap_or_else(|| panic!("{name} is declared in its inventory source"));
    let mut depth = 0;
    let mut found = BTreeSet::new();
    for (at, line) in source[start..].lines().enumerate() {
        let trimmed = line.trim();
        if depth == 1
            && !trimmed.is_empty()
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("#[")
            && !trimmed.starts_with(['}', ')'])
        {
            found.insert(identifier(trimmed).to_owned());
        }
        for c in line.chars() {
            depth += match c {
                '{' | '(' => 1,
                '}' | ')' => -1,
                _ => 0,
            };
        }
        if at > 0 && depth == 0 {
            break;
        }
    }
    found
}
pub(super) fn covered<T: std::fmt::Debug>(values: &[T]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| identifier(&format!("{value:?}")).to_owned())
        .collect()
}
pub(super) fn assert_inventory(root: &Path, file: &str, name: &str, covered: BTreeSet<String>) {
    let source = fs::read_to_string(root.join(file)).unwrap();
    assert_eq!(
        variants(&source, name),
        covered,
        "{name} gained or lost a variant: add or remove its submission control"
    );
}

pub(super) const DELIVERIES: [DiagnosticDelivery; 3] = [
    DiagnosticDelivery::Disabled,
    DiagnosticDelivery::Collected,
    DiagnosticDelivery::Forwarded,
];
pub(super) const DEMANDS: [ObservationDemand; 2] =
    [ObservationDemand::Required, ObservationDemand::NotRequired];
pub(super) const CAPTURES: [CaptureIntent; 3] = [
    CaptureIntent::Disabled,
    CaptureIntent::Observations,
    CaptureIntent::DiagnosticAndObservations,
];
pub(super) const REASONS: [ApplicabilityReason; 2] = [
    ApplicabilityReason::SyntaxRejected,
    ApplicabilityReason::DiagnosticNotApplicable,
];
pub(super) const REFUSALS: [ApplicabilityRefusal; 2] = [
    ApplicabilityRefusal::SyntaxUndetermined,
    ApplicabilityRefusal::MetadataUnavailable,
];
pub(super) fn every_intent() -> Vec<FileRuleIntent> {
    let mut all = vec![FileRuleIntent::Undemanded];
    all.extend(REASONS.map(FileRuleIntent::Skipped));
    all.extend(REFUSALS.map(FileRuleIntent::Refused));
    for diagnostic in DELIVERIES {
        for observations in DEMANDS {
            for capture in CAPTURES {
                all.push(FileRuleIntent::Run {
                    diagnostic,
                    observations,
                    capture,
                });
            }
        }
    }
    all
}
