//! A candidate whose coverage accounting is complete whatever it finds.
use super::fake::LinterFake;
use crate::outbound::linter::{interface::LinterPort, types::CanonicalText};
use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{CoverageAccounting, PairCount, RuleCoverage, RunReport};
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};

/// Candidate fixture exposes complete execution accounting independently of its findings.
pub struct CoveredCandidate(pub LinterFake);
impl LinterPort for CoveredCandidate {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        LinterPort::prepare(&self.0, request)
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        let mut report = LinterPort::decode(&self.0, request, output)?;
        report.coverage = CoverageAccounting::Accounted(RuleCoverage {
            selected: PairCount(1),
            evaluated: PairCount(1),
            skipped: PairCount(0),
            unknown_or_unread: PairCount(0),
        });
        Ok(report)
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        case: &PreparedCase,
        profile: &Tool,
    ) -> Result<CanonicalText, Error> {
        LinterPort::settings_text(&self.0, normalized, case, profile)
    }
    fn rule_context_text(&self, prepared: &PreparedRuleContext) -> CanonicalText {
        LinterPort::rule_context_text(&self.0, prepared)
    }
}
