//! A file session: the delegation to the file run and the rows the run reads.
use super::types::FileSession;
use super::types::{FileStop, RuleAfter, RuleFile};
use crate::application::file_dag::fact_lookup::service::FactLookup;
use crate::application::file_dag::program::file_session::interface::ProgramFileSessionPort;
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileFinished;
use crate::application::file_dag::program::file_session::types::FinishedReadiness;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::types::WordsToKeep;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::frontier_capture::types::FrontierCapture;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
#[cfg(any(test, feature = "fixtures"))]
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::stored_facts::types::Bucket;
use std::cell::RefCell;
impl ProgramFileSessionPort for FileSession<'_, '_> {
    fn emit_rows(
        &mut self,
        rule: RuleIndex,
        symbols: &mut Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<RowEmission, OperatingError> {
        let read = |relation| {
            source_rows(
                self.parts.read,
                self.parts.elements,
                self.parts.file,
                relation,
            )
        };
        self.parts.inner.emit_rows(rule, &read, symbols, each)
    }
    fn finish(self) -> Result<FileFinished, OperatingError> {
        Ok(FileFinished {
            readiness: match self.parts.inner.finish()? {
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
        let read = |relation| {
            source_rows(
                self.parts.read,
                self.parts.elements,
                self.parts.file,
                relation,
            )
        };
        self.parts.inner.rule(rule, demand, &read, symbols)
    }
    fn capture_frontiers(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
    ) -> Result<
        crate::application::file_dag::program::file_session::types::FrontierCaptures,
        OperatingError,
    > {
        let read = |relation| {
            source_rows(
                self.parts.read,
                self.parts.elements,
                self.parts.file,
                relation,
            )
        };
        let bound = ryo_execution_domain::domain::frontier_capture::types::CaptureScope {
            file: self.parts.file,
            rule_digest: self.parts.program.rule_digest(rule)?,
            worker: capture.worker,
            retained: ryo_execution_domain::domain::finish::types::Retained {
                arena: capture.arena,
                places: capture.places,
            },
            words: capture.words,
        };
        self.parts
            .inner
            .capture_frontiers(rule, first, &read, symbols, bound)
            .map(crate::application::file_dag::program::file_session::types::FrontierCaptures)
    }
    fn commit_frontier(
        &mut self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<(), OperatingError> {
        self.parts.inner.commit_frontier(rule, input)
    }
}
pub(crate) fn source_rows<'a, 'facts: 'a>(
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
        self.parts.inner.source_reuse_eligible()
    }
    /// Weak lifetime observation of the file's relation owner; the caller never retains it.
    #[must_use]
    pub fn owner_lifetime(
        &self,
    ) -> std::rc::Weak<ryo_execution_store::application::file_dag::relations::service::RelationOwner>
    {
        self.parts.inner.owner_lifetime()
    }
    /// The derived owner's resource counts of this file.
    #[must_use]
    pub fn resource_fixture_counts(
        &self,
    ) -> ryo_execution_store::application::file_dag::derived::owner::fixtures::resources::Snapshot
    {
        self.parts.inner.resource_fixture_counts()
    }
    /// The interpreter's readiness counters of this file, for its own tests.
    ///
    /// # Examples
    ///
    /// ```
    /// // A file session exists only inside a running file evaluation, its fields crate-private: the call's types, checked.
    /// use ryo_execution_domain::domain::readiness::types::ReadinessCounts;
    /// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
    /// fn readiness(session: &FileSession<'_, '_>) -> ReadinessCounts {
    ///     session.fixture_readiness_counts()
    /// }
    /// ```
    #[must_use]
    pub fn fixture_readiness_counts(
        &self,
    ) -> ryo_execution_domain::domain::readiness::types::ReadinessCounts {
        self.parts.inner.readiness_counts()
    }
    /// The interpreter's shared-walk counters of this file, for its own tests.
    ///
    /// # Examples
    ///
    /// ```
    /// // A file session exists only inside a running file evaluation, its fields crate-private: the call's types, checked.
    /// use ryo_execution_domain::domain::shared_walk::types::WalkCounts;
    /// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
    /// fn walks(session: &FileSession<'_, '_>) -> WalkCounts {
    ///     session.fixture_walk_counts()
    /// }
    /// ```
    #[must_use]
    pub fn fixture_walk_counts(
        &self,
    ) -> ryo_execution_domain::domain::shared_walk::types::WalkCounts {
        self.parts.inner.walk_counts()
    }
    /// The rows each step of `rule` was fed in this file, for the interpreter's own tests.
    ///
    /// # Examples
    ///
    /// ```
    /// // A file session exists only inside a running file evaluation, its fields crate-private: the call's types, checked.
    /// use ryo_execution_domain::domain::coverage::types::RuleIndex;
    /// use ryo_execution_domain::domain::operator::types::LogicalCounts;
    /// use ryo_execution_domain::domain::operating::errors::OperatingError;
    /// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
    /// fn fed(session: &FileSession<'_, '_>) -> Result<LogicalCounts, OperatingError> {
    ///     session.fixture_counts(RuleIndex(0))
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A stale generation, a rule out of range, or a step that was not placed or has not run.
    pub fn fixture_counts(
        &self,
        rule: RuleIndex,
    ) -> Result<ryo_execution_domain::domain::operator::types::LogicalCounts, OperatingError> {
        self.parts.inner.collected_counts(rule)
    }
}
impl FileSession<'_, '_> {
    /// Reports the rows each step of `rule` was fed to the request's observer, after the checks every count read makes; without
    /// an observer only the checks are made.
    pub(crate) fn observe(&self, rule: RuleIndex) -> Result<(), OperatingError> {
        self.parts.inner.observe(rule)
    }
}

/// The stop of a rule: the facts' own refusal when a read of them was refused, else the engine's `error`.
pub(crate) fn stopped(error: OperatingError, refused: &RefCell<Option<FactsError>>) -> RuleAfter {
    RuleAfter::Stopped(
        refused
            .take()
            .map_or(FileStop::Operating(error), FileStop::Facts),
    )
}

/// What a rule does after its emitted root: its frontiers captured, each one captured kept in `facts` and committed, and its steps
/// reported to the observer, if any. A capture the engine could not make is answered as it is, for the caller to note.
pub(crate) fn rest_of_rule(
    session: &mut FileSession<'_, '_>,
    at: RuleFile<'_>,
    symbols: &mut Symbols,
    capture: CaptureContext<'_>,
    facts: &FactLookup<'_, '_>,
    refused: &RefCell<Option<FactsError>>,
) -> Result<RuleAfter, OperatingError> {
    let first = *at.first.get(at.rule.0).ok_or(OperatingError::NotRun)?;
    let captured = session.capture_frontiers(at.rule, first, symbols, capture)?;
    if let Some(error) = refused.take() {
        return Ok(RuleAfter::Stopped(FileStop::Facts(error)));
    }
    let mut captures = Vec::with_capacity(captured.len());
    for each in captured.0 {
        if let Ok(kept) = &each.outcome {
            if let Err(error) = facts.keep(
                kept.bucket,
                WordsToKeep {
                    words: &kept.words,
                    file: at.file,
                    proof: &kept.proof,
                },
            ) {
                return Ok(RuleAfter::Stopped(FileStop::Facts(error)));
            }
            session.commit_frontier(at.rule, kept.input)?;
        }
        captures.push(each);
    }
    session.observe(at.rule)?;
    Ok(RuleAfter::Ran { captures })
}
