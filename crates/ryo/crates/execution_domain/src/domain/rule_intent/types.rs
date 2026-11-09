//! What a file rule is asked to do before its source is walked: the demand of its outputs and of its frontiers.
use ryo_values::domain::runs::types::ApplicabilityRefusal;
/// Whether a rule's diagnostics are emitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitDemand {
    /// They are emitted.
    Enabled,
    /// They are not emitted.
    Disabled,
}
/// Diagnostic output delivery, fixed before admission. Every diagnostic,
/// observation and frontier uses the same semantic evaluation contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticDelivery {
    /// No diagnostic is delivered.
    Disabled,
    /// Diagnostics are collected.
    Collected,
    /// Diagnostics are forwarded as they are made.
    Forwarded,
}
/// Whether another rule observes a rule's outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationDemand {
    /// They are observed.
    Required,
    /// They are not observed.
    NotRequired,
}
/// What a rule is asked to deliver: its diagnostics and its observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleDemand {
    /// Whether its diagnostics are emitted.
    pub emit: EmitDemand,
    /// Whether its outputs are observed.
    pub observations: ObservationDemand,
}
/// Frontier roots admitted by this file generation. Observation-only producers
/// cannot acquire diagnostic-only frontiers through a later capture call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureIntent {
    /// No frontier is captured.
    Disabled,
    /// The frontiers of the rule's observations are captured.
    Observations,
    /// The frontiers of its diagnostics and observations are captured.
    DiagnosticAndObservations,
}
/// Why a rule does not apply to a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicabilityReason {
    /// The file's syntax is rejected.
    SyntaxRejected,
    /// The rule's diagnostics do not apply to the file.
    DiagnosticNotApplicable,
}
/// Complete per-rule intent, frozen before any file source can be walked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileRuleIntent {
    /// The rule is not demanded.
    Undemanded,
    /// The rule does not apply to the file.
    Skipped(ApplicabilityReason),
    /// Whether the rule applies could not be decided.
    Refused(ApplicabilityRefusal),
    /// The rule runs on the file.
    Run {
        /// How its diagnostics are delivered.
        diagnostic: DiagnosticDelivery,
        /// Whether its outputs are observed.
        observations: ObservationDemand,
        /// Which of its frontiers are captured.
        capture: CaptureIntent,
    },
}

/// What a rule's retained frontier serves: its own diagnostics or a named observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierPurpose {
    /// The rule's own diagnostics.
    Diagnostic,
    /// A named observation.
    Observation,
}

/// Whether a rule renders syntax and so needs the semantic session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticNeed {
    /// It renders syntax, so it needs the semantic session.
    Required,
    /// It renders none.
    NotRequired,
}
