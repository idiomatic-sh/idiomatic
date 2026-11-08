//! A file session: the delegation to the file run and the rows the run reads.
use crate::application::file_dag::program::file_session::interface::FileExecution;
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileFinished;
use crate::application::file_dag::program::file_session::types::FinishedReadiness;
use crate::application::file_dag::program::file_session::types::FileSession;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::frontier_capture::types::FrontierCapture;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::LogicalCounts;
use ryo_execution_domain::domain::readiness::types::ReadinessCounts;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::shared_walk::types::WalkCounts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::stored_facts::types::Bucket;
#[cfg(any(test, feature = "fixtures"))]
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
impl FileExecution for FileSession<'_, '_> {
    fn readiness_counts(&self) -> ReadinessCounts {
        self.inner.readiness_counts()
    }

    fn walk_counts(&self) -> WalkCounts {
        self.inner.walk_counts()
    }

    fn emit_rows(
        &mut self,
        rule: RuleIndex,
        symbols: &mut Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<RowEmission, OperatingError> {
        let read = |relation| source_rows(self.read, self.elements, self.file, relation);
        self.inner.emit_rows(rule, &read, symbols, each)
    }
    fn finish(self) -> Result<FileFinished, OperatingError> {
        Ok(FileFinished {
            readiness: match self.inner.finish()? {
                Some(readiness) => FinishedReadiness::Observed(readiness),
                None => FinishedReadiness::Unobserved,
            },
        })
    }

    fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut Symbols,
    ) -> Result<RuleOutcome, OperatingError> {
        let read = |relation| source_rows(self.read, self.elements, self.file, relation);
        self.inner.rule(rule, demand, &read, symbols)
    }
    fn capture_frontiers(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError> {
        let read = |relation| source_rows(self.read, self.elements, self.file, relation);
        let bound = ryo_execution_domain::domain::frontier_capture::types::CaptureScope {
            file: self.file,
            rule_digest: self.program.rule_digest(rule)?,
            worker: capture.worker,
            retained: ryo_execution_domain::domain::finish::types::Retained {
                arena: capture.arena,
                places: capture.places,
            },
            words: capture.words,
        };
        self.inner
            .capture_frontiers(rule, first, &read, symbols, bound)
    }
    fn commit_frontier(
        &mut self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<(), OperatingError> {
        self.inner.commit_frontier(rule, input)
    }
    fn counts(&self, rule: RuleIndex) -> Result<LogicalCounts, OperatingError> {
        self.inner.counts(rule)
    }
}
fn source_rows<'a, 'facts: 'a>(
    read: &dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<SourceRows<'facts>, OperatingError>,
    elements: &'a ryo_language::domain::elements::types::Elements,
    file: ryo_relations::domain::relations::types::FileId,
    relation: ryo_relations::domain::relations::types::Relation,
) -> Result<ryo_execution_domain::domain::table_model::types::Rows<'a>, OperatingError> {
    use ryo_execution_domain::domain::{
        table_cells::{service::native_form, types::TreeRows},
        table_model::types::Rows,
    };
    if let Some(form) = native_form(relation) {
        return Ok(Rows::Tree(TreeRows {
            form,
            elements,
            file,
        }));
    }
    read(relation).map(|rows| match rows {
        SourceRows::Stored(columns) => Rows::Stored(columns),
        SourceRows::Computed(table) => Rows::Computed(table),
    })
}

/// The run's own observations, for the execution crate's use-case proofs: they read the owner a file session holds, which
/// no caller can name.
#[cfg(any(test, feature = "fixtures"))]
impl FileSession<'_, '_> {
    /// Whether the source of this file may be reused by a later run.
    #[must_use]
    pub fn reuse_eligible(&self) -> ReuseEligibility {
        self.inner.source_reuse_eligible()
    }
    /// Weak lifetime observation of the file's relation owner; the caller never retains it.
    #[must_use]
    pub fn owner_lifetime(&self) -> std::rc::Weak<ryo_execution_store::application::file_dag::relations::RelationOwner> {
        self.inner.owner_lifetime()
    }
    /// The derived owner's resource counts of this file.
    #[must_use]
    pub fn resource_fixture_counts(
        &self,
    ) -> ryo_execution_store::application::file_dag::derived::owner::fixtures::resources::Snapshot {
        self.inner.resource_fixture_counts()
    }
}
