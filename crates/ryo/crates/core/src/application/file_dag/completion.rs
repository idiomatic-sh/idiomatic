//! Required scope-specific completion. Only successful evaluation can mint rows;
//! unavailable inputs retain provenance and can never become a complete empty table.
use crate::domain::{
    coverage::{
        service::drop_uncertain,
        types::{Partial, Partials, RuleIndex},
    },
    operating::errors::OperatingError,
    trust::{
        service::{blocked, files_for},
        types::{Blocking, RelationRead},
    },
};
use ryo_relations::domain::relations::types::{CellKind, FileId};
use ryo_values::domain::{plan_parts::types::Input, plans::types::Operation, rows::types::Table};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeIdentity {
    File { rule: RuleIndex, file: FileId },
    Project { rule: RuleIndex },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UnavailableReason {
    MissingFrontier { input: Input },
    AbsenceBlockedByPartial,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnavailableCompletion {
    pub scope: ScopeIdentity,
    pub reason: UnavailableReason,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CoverageState {
    Complete,
    Partial { files: Vec<Option<FileId>> },
}
/// Not publicly constructible. This certifies execution completed, independently
/// of whether the input coverage is Complete or Partial.
#[derive(Debug)]
pub(crate) struct CompletedRows<T = Table>(T);
impl<T> CompletedRows<T> {
    pub(crate) fn into_payload(self) -> T {
        self.0
    }
}
#[derive(Debug)]
pub(crate) enum NodeCompletion<T = Table> {
    /// Deferred original failure: other independent roots retain their own state.
    Failed(OperatingError),
    Available {
        rows: CompletedRows<T>,
        coverage: CoverageState,
    },
    Unavailable(UnavailableCompletion),
}

#[derive(Clone)]
enum CompletionBehavior {
    BlockBeforeRun(std::sync::Arc<[Partial]>),
    DropAfterCollect(std::sync::Arc<Partials>),
}
/// No Default: the scope's actual partiality source must be supplied.
#[derive(Clone)]
pub(crate) struct CompletionPolicy {
    scope: ScopeIdentity,
    behavior: CompletionBehavior,
    /// Publication readiness is physical evidence, independent of Rows-mode
    /// evaluation relaxation or diagnostic-only absence requirements.
    publication: CompletionBehavior,
}
impl CompletionPolicy {
    #[cfg(test)]
    pub(crate) fn file(rule: RuleIndex, file: FileId, partials: &[Partial]) -> Self {
        Self::file_inputs(rule, file, partials, partials)
    }
    #[cfg(test)]
    pub(crate) fn project(rule: RuleIndex, partials: &Partials) -> Self {
        Self::project_inputs(rule, partials, partials)
    }
    pub(super) fn file_inputs(
        rule: RuleIndex,
        file: FileId,
        evaluation: &[Partial],
        publication: &[Partial],
    ) -> Self {
        Self {
            scope: ScopeIdentity::File { rule, file },
            behavior: CompletionBehavior::BlockBeforeRun(evaluation.into()),
            publication: CompletionBehavior::BlockBeforeRun(publication.into()),
        }
    }
    pub(super) fn project_inputs(
        rule: RuleIndex,
        evaluation: &Partials,
        publication: &Partials,
    ) -> Self {
        Self {
            scope: ScopeIdentity::Project { rule },
            behavior: CompletionBehavior::DropAfterCollect(std::sync::Arc::new(evaluation.clone())),
            publication: CompletionBehavior::DropAfterCollect(std::sync::Arc::new(
                publication.clone(),
            )),
        }
    }
    pub(super) fn observation_ready(&self, reads: &[RelationRead]) -> bool {
        match &self.publication {
            CompletionBehavior::BlockBeforeRun(partials) => {
                blocked(reads, partials) == Blocking::Free
            }
            CompletionBehavior::DropAfterCollect(partials) => files_for(partials, reads).is_empty(),
        }
    }
    pub(super) fn for_rule(&self, rule: RuleIndex) -> Self {
        let scope = match self.scope {
            ScopeIdentity::File { file, .. } => ScopeIdentity::File { rule, file },
            ScopeIdentity::Project { .. } => ScopeIdentity::Project { rule },
        };
        Self {
            scope,
            behavior: self.behavior.clone(),
            publication: self.publication.clone(),
        }
    }
    /// The common scheduler checks this BEFORE demanding any semantic input.
    pub(super) fn before(&self, needs: &[RelationRead]) -> Result<(), OperatingError> {
        if let CompletionBehavior::BlockBeforeRun(partials) = &self.behavior
            && blocked(needs, partials) == Blocking::Blocked
        {
            let unavailable = UnavailableCompletion {
                scope: self.scope,
                reason: UnavailableReason::AbsenceBlockedByPartial,
            };
            return Err(OperatingError::SemanticIncomplete {
                reason: format!("{unavailable:?}"),
            });
        }
        Ok(())
    }
    /// Scope filtering follows successful semantic evaluation and publication
    /// staging. A failure never becomes an empty/filtered successful node.
    pub(super) fn finish<'f>(
        &self,
        operation: &Operation,
        needs: &[RelationRead],
        probed_kinds: &[CellKind],
        publication: Result<super::relations::PublishedInput, OperatingError>,
        owner: &super::relations::RelationOwner,
        read: &dyn Fn(
            ryo_relations::domain::relations::types::Relation,
        )
            -> Result<crate::domain::table_model::types::Rows<'f>, OperatingError>,
    ) -> NodeCompletion<super::relations::PublishedInput> {
        let publication = match publication {
            Ok(value) => value,
            Err(error) => return NodeCompletion::Failed(error),
        };
        let CompletionBehavior::DropAfterCollect(partials) = &self.behavior else {
            return NodeCompletion::Available {
                rows: CompletedRows(publication),
                coverage: CoverageState::Complete,
            };
        };
        let uncertain = files_for(partials, needs);
        if uncertain.is_empty() {
            return NodeCompletion::Available {
                rows: CompletedRows(publication),
                coverage: CoverageState::Complete,
            };
        }
        let result = (|| {
            let rows = owner.resolve(publication.as_ref(), read)?;
            let mut table = Table::default();
            crate::domain::table_model::service::materialize(&rows, &mut table, &mut Vec::new());
            drop_uncertain(operation, probed_kinds, &mut table, &uncertain);
            owner.computed(table)
        })();
        match result {
            Ok(publication) => NodeCompletion::Available {
                rows: CompletedRows(publication),
                coverage: CoverageState::Partial { files: uncertain },
            },
            Err(error) => NodeCompletion::Failed(error),
        }
    }
    pub(crate) fn missing_frontier(&self, input: Input) -> NodeCompletion {
        NodeCompletion::Unavailable(UnavailableCompletion {
            scope: self.scope,
            reason: UnavailableReason::MissingFrontier { input },
        })
    }
    /// File blocking precedes evaluation; project filtering follows SUCCESSFUL
    /// collection. In particular a project error is not swallowed by partiality.
    #[cfg(test)]
    pub(crate) fn evaluate(
        &self,
        operation: &Operation,
        needs: &[RelationRead],
        probed_kinds: &[CellKind],
        evaluate: impl FnOnce() -> Result<Table, OperatingError>,
    ) -> NodeCompletion {
        if let CompletionBehavior::BlockBeforeRun(partials) = &self.behavior
            && blocked(needs, partials) == Blocking::Blocked
        {
            return NodeCompletion::Unavailable(UnavailableCompletion {
                scope: self.scope,
                reason: UnavailableReason::AbsenceBlockedByPartial,
            });
        }
        // The closure owns staged output. An error cannot mint CompletedRows.
        let mut table = match evaluate() {
            Ok(table) => table,
            Err(error) => return NodeCompletion::Failed(error),
        };
        let coverage = match &self.behavior {
            CompletionBehavior::BlockBeforeRun(_) => CoverageState::Complete,
            CompletionBehavior::DropAfterCollect(partials) => {
                let uncertain = files_for(partials, needs);
                if uncertain.is_empty() {
                    CoverageState::Complete
                } else {
                    drop_uncertain(operation, probed_kinds, &mut table, &uncertain);
                    CoverageState::Partial { files: uncertain }
                }
            }
        };
        NodeCompletion::Available {
            rows: CompletedRows(table),
            coverage,
        }
    }
}

#[cfg(test)]
#[path = "completion/service_test.rs"]
mod tests;
