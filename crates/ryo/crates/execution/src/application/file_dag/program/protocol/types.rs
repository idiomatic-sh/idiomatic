//! The execution protocol's data, independent of request compilation and run orchestration.
use crate::application::file_dag::program::admission::ApplicabilityReason;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::types::RowsFed;
use ryo_relations::domain::relations::types::Words;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::rows::types::Table;

pub struct CaptureScope<'a> {
    pub file: ryo_relations::domain::relations::types::FileId,
    /// Current rule identity validated by the store before warm replay.
    pub rule_digest: ryo_values::domain::stored_facts::types::Digest,
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    pub retained: crate::domain::finish::types::Retained<'a>,
    pub words: &'a mut Words,
}
pub struct FrontierCapture {
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    pub outcome: Result<CapturedFrontier, OperatingError>,
}
pub struct CapturedFrontier {
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    pub bucket: ryo_values::domain::stored_facts::types::Bucket,
    /// Source-bound root and physical coverage retained with the packed words.
    pub proof: ryo_values::domain::records::types::FrontierProof,
    pub words: ryo_values::domain::stored_facts::types::PackedWords,
}
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
pub enum RootOutcome {
    Complete(Table),
    /// Usable diagnostic rows and the exact physical partitions still uncertain.
    Partial {
        table: Table,
        affected: Vec<Option<ryo_relations::domain::relations::types::FileId>>,
    },
    Skipped(crate::application::file_dag::program::admission::ApplicabilityProof),
    Unknown(OperatingError),
    Failed(OperatingError),
    NoQuery,
    /// This compiled root belongs to another execution phase and has not run.
    Deferred {
        phase: StepScope,
    },
}
/// Opaque application handover of the actual admitted population. Packed bytes
/// may accompany it, but never substitute for its owner/generation evidence.
#[derive(Clone)]
pub struct ObservedPublication {
    pub(super) admitted: crate::application::file_dag::program::admission::AdmittedInput,
}
pub struct LogicalCounts {
    pub fed: Box<[RowsFed]>,
}
/// What became of one declared output of a rule.
pub struct OutputOutcome {
    pub output: ObservationIndex,
    pub outcome: RootOutcome,
}
/// What became of one declared output of a project rule.
pub struct ProjectOutputOutcome {
    pub output: ObservationIndex,
    pub outcome: ProjectObservation,
}
pub struct RuleOutcome {
    pub emit: RootOutcome,
    pub observations: Vec<OutputOutcome>,
    pub counts: LogicalCounts,
}
/// A project named root can be Complete only with the admitted owner proof.
pub enum ProjectObservation {
    Complete {
        table: Table,
        publication: ObservedPublication,
    },
    /// Known rows whose exact uncertainty remains in the admitted publication.
    Partial {
        table: Table,
        publication: ObservedPublication,
    },
    Other(RootOutcome),
}
/// Completed results retain declaration order if the required host guard refuses advancement.
pub enum ProjectBatch<T, E> {
    Complete(Vec<T>),
    Refused { completed: Vec<T>, error: E },
}
pub struct ProjectRuleOutcome {
    pub emit: RootOutcome,
    pub observations: Vec<ProjectOutputOutcome>,
    pub counts: LogicalCounts,
}

/// Successful row delivery preserves physical uncertainty rather than fabricating an empty table.
pub enum RowEmission {
    /// No rows were delivered: the compiled root belongs to this later phase.
    Deferred {
        phase: StepScope,
    },
    Complete,
    Partial {
        affected: Vec<Option<ryo_relations::domain::relations::types::FileId>>,
    },
}
