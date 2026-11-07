//! What became of the rules of a request: the outcome of each root, output and project observation.
use crate::application::file_dag::program::observed_publication::types::ObservedPublication;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::types::LogicalCounts;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::rows::types::Table;
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
