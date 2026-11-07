//! What a file session offers a caller.
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileFinished;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::frontier_capture::types::FrontierCapture;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::types::LogicalCounts;
use crate::domain::readiness::types::ReadinessCounts;
use crate::domain::row_emission::types::RowEmission;
use crate::domain::rule_intent::types::RuleDemand;
use crate::domain::shared_walk::types::WalkCounts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::stored_facts::types::Bucket;
/// The intent supplied at file start is immutable throughout this session.
/// Applicability cannot be changed after another rule has started the shared walk.
/// ```compile_fail,E0599
/// use ryo_execution::inbound::use_cases::execution::{
///     interface::FileExecution,
///     types::{ApplicabilityReason, FileSession, RuleIndex},
/// };
/// fn late_skip(file: &mut FileSession<'_, '_>) {
///     file.skip_rule(RuleIndex(0), ApplicabilityReason::SyntaxRejected).unwrap();
/// }
/// ```
pub trait FileExecution {
    /// Actual bound-root readiness polling and dependency wakeups in this owner.
    fn readiness_counts(&self) -> ReadinessCounts;
    /// Actual shared native pass work already performed by this file owner.
    fn walk_counts(&self) -> WalkCounts;
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
    fn counts(&self, rule: RuleIndex) -> Result<LogicalCounts, OperatingError>;
}
