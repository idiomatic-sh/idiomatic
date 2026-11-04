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
pub(crate) struct CompletedRows(Table);
impl CompletedRows {
    pub(crate) fn into_table(self) -> Table {
        self.0
    }
}
#[derive(Debug)]
pub(crate) enum NodeCompletion {
    Available {
        rows: CompletedRows,
        coverage: CoverageState,
    },
    Unavailable(UnavailableCompletion),
}

#[derive(Clone, Copy)]
enum CompletionBehavior<'a> {
    BlockBeforeRun(&'a [Partial]),
    DropAfterCollect(&'a Partials),
}
/// No Default: the scope's actual partiality source must be supplied.
pub(crate) struct CompletionPolicy<'a> {
    scope: ScopeIdentity,
    behavior: CompletionBehavior<'a>,
}
impl<'a> CompletionPolicy<'a> {
    pub(crate) fn file(rule: RuleIndex, file: FileId, partials: &'a [Partial]) -> Self {
        Self {
            scope: ScopeIdentity::File { rule, file },
            behavior: CompletionBehavior::BlockBeforeRun(partials),
        }
    }
    pub(crate) fn project(rule: RuleIndex, partials: &'a Partials) -> Self {
        Self {
            scope: ScopeIdentity::Project { rule },
            behavior: CompletionBehavior::DropAfterCollect(partials),
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
    pub(crate) fn evaluate(
        &self,
        operation: &Operation,
        needs: &[RelationRead],
        probed_kinds: &[CellKind],
        evaluate: impl FnOnce() -> Result<Table, OperatingError>,
    ) -> Result<NodeCompletion, OperatingError> {
        if let CompletionBehavior::BlockBeforeRun(partials) = self.behavior
            && blocked(needs, partials) == Blocking::Blocked
        {
            return Ok(NodeCompletion::Unavailable(UnavailableCompletion {
                scope: self.scope,
                reason: UnavailableReason::AbsenceBlockedByPartial,
            }));
        }
        // The closure owns staged output. An error cannot mint CompletedRows.
        let mut table = evaluate()?;
        let coverage = match self.behavior {
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
        Ok(NodeCompletion::Available {
            rows: CompletedRows(table),
            coverage,
        })
    }
}

#[cfg(test)]
#[path = "completion/service_test.rs"]
mod tests;
