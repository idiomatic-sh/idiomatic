//! One public prepare/open/run protocol; ownership is never delegated to callers.
use super::types::*;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{
    files::types::CorpusInput, runs::types::Skipped, stored_facts::types::Bucket,
};
pub trait ExecutionUseCase {
    fn prepare(&self, request: PreparedRequest) -> Result<PreparedProgram, OperatingError>;
    fn begin(&self, program: &PreparedProgram, corpus: &[CorpusInput]) -> RequestExecution;
}
pub trait ExecutionRequest {
    fn prepare_frontiers(
        &self,
        input: FrontierRequest<'_>,
        symbols: &mut Symbols,
    ) -> Result<PreparedFrontiers, PreparationError>;
    fn finish(self)
    where
        Self: Sized;
    fn open_file<'a, 'facts: 'a>(
        &'a self,
        input: FileInput<'a, 'facts>,
        symbols: &mut Symbols,
    ) -> Result<FileSession<'a, 'facts>, OperatingError>;
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
    ) -> Result<ProjectScope, OperatingError>;
    fn open_project<'a>(
        &'a self,
        input: ProjectInput<'a>,
        symbols: &mut Symbols,
    ) -> Result<ProjectSession<'a>, OperatingError>;
}
pub trait FileExecution {
    fn finish(self) -> Result<FileFinished, OperatingError>
    where
        Self: Sized;
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
    fn skip_rule(
        &mut self,
        rule: RuleIndex,
        reason: ApplicabilityReason,
    ) -> Result<(), OperatingError>;
    fn counts(&self, rule: RuleIndex) -> Result<LogicalCounts, OperatingError>;
}
pub trait ProjectExecution {
    fn finish(self)
    where
        Self: Sized;
    fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut Symbols,
    ) -> Result<ProjectRuleOutcome, OperatingError>;
}

pub trait ProjectStage {
    fn finish(self) -> Result<(), OperatingError>;
}
