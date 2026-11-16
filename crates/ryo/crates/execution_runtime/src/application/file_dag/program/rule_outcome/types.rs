//! What became of the rules of a request: the outcome of each root, output and project observation.
use crate::application::file_dag::program::observed_publication::service::ObservedPublication;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::rows::types::Table;
/// What became of one root of a rule.
#[derive(Debug)]
pub enum RootOutcome {
    /// The root ran over every row it was owed: its table.
    Complete(Table),
    /// Usable diagnostic rows and the exact physical partitions still uncertain.
    Partial {
        /// The usable diagnostic rows.
        table: Table,
        /// The files whose partitions are still uncertain; `None` stands for a partition no file names.
        affected: Vec<Option<ryo_relations::domain::relations::types::FileId>>,
    },
    /// The root did not run: the applicability proof says why.
    Skipped(crate::application::file_dag::program::admission::service::ApplicabilityProof),
    /// The rows could not be decided; the refusal says why.
    Unknown(OperatingError),
    /// The root's run failed; the refusal says why.
    Failed(OperatingError),
    /// The rule has no query to run.
    NoQuery,
    /// This compiled root belongs to another execution phase and has not run.
    Deferred {
        /// The phase the root belongs to.
        phase: StepScope,
    },
}
/// What became of one declared output of a rule.
#[derive(Debug)]
pub struct OutputOutcome {
    /// The output the outcome belongs to.
    pub output: ObservationIndex,
    /// What became of the output's root.
    pub outcome: RootOutcome,
}
/// What became of one declared output of a project rule.
#[derive(Debug)]
pub struct ProjectOutputOutcome {
    /// The output the outcome belongs to.
    pub output: ObservationIndex,
    /// What became of the output of the project rule.
    pub outcome: ProjectObservation,
}
/// What became of one rule: its emitted root and each declared output. The observation type names the kind of
/// output: a file rule yields `OutputOutcome`, a project rule `ProjectOutputOutcome`.
#[derive(Debug)]
pub struct RuleOutcome<O = OutputOutcome> {
    /// What became of the rule's emitted root.
    pub emit: RootOutcome,
    /// What became of each declared output.
    pub observations: Vec<O>,
}
/// A project named root can be Complete only with the admitted owner proof.
#[derive(Debug)]
pub enum ProjectObservation {
    /// The named root is complete.
    Complete {
        /// The root's rows.
        table: Table,
        /// The publication the owner admitted for the rows.
        publication: ObservedPublication,
    },
    /// Known rows whose exact uncertainty remains in the admitted publication.
    Partial {
        /// The root's known rows.
        table: Table,
        /// The publication the owner admitted, which keeps the rows' uncertainty.
        publication: ObservedPublication,
    },
    /// A root of another outcome than the project's: it ran no project observation.
    Other(RootOutcome),
}
/// Completed results retain declaration order if the required host guard refuses advancement.
#[derive(Debug)]
pub enum ProjectBatch<T, E> {
    /// Every batch completed, in declaration order.
    Complete(Vec<T>),
    /// A batch was refused: the batches completed before it and the refusal.
    Refused {
        /// The batches completed before the refusal, in declaration order.
        completed: Vec<T>,
        /// The refusal of the guard.
        error: E,
    },
}
