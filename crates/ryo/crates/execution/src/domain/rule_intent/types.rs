//! What a file rule is asked to do before its source is walked: the demand of its outputs and of its frontiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitDemand {
    Enabled,
    Disabled,
}
/// Diagnostic output delivery, fixed before admission. Every diagnostic,
/// observation and frontier uses the same semantic evaluation contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticDelivery {
    Disabled,
    Collected,
    Forwarded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationDemand {
    Required,
    NotRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleDemand {
    pub emit: EmitDemand,
    pub observations: ObservationDemand,
}
/// Frontier roots admitted by this file generation. Observation-only producers
/// cannot acquire diagnostic-only frontiers through a later capture call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureIntent {
    Disabled,
    Observations,
    DiagnosticAndObservations,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicabilityRefusal {
    SyntaxUndetermined,
    MetadataUnavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicabilityReason {
    SyntaxRejected,
    DiagnosticNotApplicable,
}
/// Complete per-rule intent, frozen before any file source can be walked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileRuleIntent {
    Undemanded,
    Skipped(ApplicabilityReason),
    Refused(ApplicabilityRefusal),
    Run {
        diagnostic: DiagnosticDelivery,
        observations: ObservationDemand,
        capture: CaptureIntent,
    },
}
