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
    fn emit_rows(
        &mut self,
        rule: RuleIndex,
        symbols: &mut Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<RowEmission, OperatingError>;
    fn finish(self) -> Result<FileFinished, OperatingError>
    where
        Self: Sized;
    /// Collect exactly the declared roots. Forwarded diagnostic roots cannot change mode.
    fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut Symbols,
    ) -> Result<RuleOutcome, OperatingError>;
    fn capture_frontiers(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError>;
    fn commit_frontier(
        &mut self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<(), OperatingError>;
}
