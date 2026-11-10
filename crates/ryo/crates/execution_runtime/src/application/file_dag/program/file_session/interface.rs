//! What a file session offers a caller.
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileFinished;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::frontier_capture::types::FrontierCapture;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::stored_facts::types::Bucket;
/// The intent supplied at file start is immutable throughout this session.
/// Applicability cannot be changed after another rule has started the shared walk.
/// ```compile_fail,E0599
/// use ryo_execution_runtime::application::file_dag::program::file_session::{interface::FileExecution, types::FileSession};
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_domain::domain::rule_intent::types::ApplicabilityReason;
/// fn late_skip(file: &mut FileSession<'_, '_>) {
///     file.skip_rule(RuleIndex(0), ApplicabilityReason::SyntaxRejected).unwrap();
/// }
/// ```
/// The interpreter's counters and the rows its steps were fed are not the seam's: a caller cannot read its readiness counters,
/// ```compile_fail,E0599
/// use ryo_execution_runtime::application::file_dag::program::file_session::{interface::FileExecution, types::FileSession};
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// fn readiness(file: &FileSession<'_, '_>) {
///     let _ = file.readiness_counts();
/// }
/// ```
/// its walk counters,
/// ```compile_fail,E0599
/// use ryo_execution_runtime::application::file_dag::program::file_session::{interface::FileExecution, types::FileSession};
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// fn walk(file: &FileSession<'_, '_>) {
///     let _ = file.walk_counts();
/// }
/// ```
/// nor the rows a rule's steps were fed.
/// ```compile_fail,E0599
/// use ryo_execution_runtime::application::file_dag::program::file_session::{interface::FileExecution, types::FileSession};
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// fn fed(file: &FileSession<'_, '_>) {
///     let _ = file.counts(RuleIndex(0));
/// }
/// ```
pub trait FileExecution {
    /// Deliver the already committed canonical publication to the callback.
    /// Sink refusal does not repeat admission or semantic evaluation.
    /// A different file-start delivery declaration is refused before source admission.
    ///
    /// # Errors
    ///
    /// The session's refusal of the rule (an undeclared rule, another delivery declaration, a source it cannot read or
    /// admit, an evaluation error), and the first error `each` returns, which stops the delivery.
    fn emit_rows(
        &mut self,
        rule: RuleIndex,
        symbols: &mut Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<RowEmission, OperatingError>;
    /// Ends the session, answering the file's readiness when it observed any.
    ///
    /// # Errors
    ///
    /// The session's refusal to finish: a publication or readiness it cannot complete.
    fn finish(self) -> Result<FileFinished, OperatingError>
    where
        Self: Sized;
    /// Collect exactly the declared roots. Forwarded diagnostic roots cannot change mode.
    ///
    /// # Errors
    ///
    /// The session's refusal of the rule: an undeclared rule or demand, or a source it cannot read or admit.
    fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut Symbols,
    ) -> Result<RuleOutcome, OperatingError>;
    /// Captures the rule's frontiers for the cross-file steps, from the first bucket on.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold, and the session's refusal to capture its
    /// frontiers.
    fn capture_frontiers(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError>;
    /// Commits one captured frontier input of the rule.
    ///
    /// # Errors
    ///
    /// The session's refusal of a frontier it did not capture or already committed.
    fn commit_frontier(
        &mut self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<(), OperatingError>;
}
