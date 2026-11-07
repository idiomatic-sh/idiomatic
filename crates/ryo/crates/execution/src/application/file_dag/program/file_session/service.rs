//! A file session: the delegation to the file run and the rows the run reads.
use crate::application::file_dag::program::file_session::interface::FileExecution;
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileFinished;
use crate::application::file_dag::program::file_session::types::FileSession;
use crate::application::file_dag::program::file_session::types::SourceRows;
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
use crate::application::file_dag::relations::types::ReuseEligibility;
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
            readiness: self.inner.finish()?,
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
        let bound = crate::domain::frontier_capture::types::CaptureScope {
            file: self.file,
            rule_digest: self.program.rule_digest(rule)?,
            worker: capture.worker,
            retained: crate::domain::finish::types::Retained {
                arena: crate::domain::finish::types::Arena(capture.arena),
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
) -> Result<crate::domain::table_model::types::Rows<'a>, OperatingError> {
    use crate::domain::{
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

impl FileSession<'_, '_> {
    /// Whether the source of this file may be reused by a later run.
    pub(crate) fn reuse_eligible(&self) -> bool {
        self.inner.source_reuse_eligible() == ReuseEligibility::Eligible
    }
}
