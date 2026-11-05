//! Required scope-specific completion. Only successful evaluation can mint rows;
//! unavailable inputs retain provenance and can never become a complete empty table.
use crate::domain::{
    coverage::{
        service::drop_uncertain,
        types::{Partial, Partials, RuleIndex},
    },
    trust::{
        service::{blocked, files_for},
        types::{Blocking, RelationRead},
    },
};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::{plans::types::Operation, rows::types::Table};

use super::super::{CoverageState, ScopeIdentity};
#[derive(Clone, PartialEq, Eq)]
enum CompletionBehavior {
    BlockBeforeRun(std::sync::Arc<[Partial]>),
    DropAfterCollect(std::sync::Arc<Partials>),
}
/// No Default: the scope's actual partiality source must be supplied.
#[derive(Clone, PartialEq, Eq)]
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
    pub(crate) fn file_inputs(
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
    pub(crate) fn project_inputs(
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
    pub(crate) fn blocked_coverage(&self) -> CoverageState {
        match self.scope {
            ScopeIdentity::File { file, .. } => CoverageState::Partial {
                files: vec![Some(file)],
            },
            ScopeIdentity::Project { .. } => CoverageState::Partial { files: vec![None] },
        }
    }
    pub(crate) fn coverage(&self, reads: &[RelationRead]) -> CoverageState {
        let files = match &self.publication {
            CompletionBehavior::BlockBeforeRun(partials)
                if blocked(reads, partials) == Blocking::Blocked =>
            {
                match self.scope {
                    ScopeIdentity::File { file, .. } => vec![Some(file)],
                    ScopeIdentity::Project { .. } => {
                        unreachable!("file completion policy has a file scope")
                    }
                }
            }
            CompletionBehavior::BlockBeforeRun(_) => Vec::new(),
            CompletionBehavior::DropAfterCollect(partials) => files_for(partials, reads),
        };
        if files.is_empty() {
            CoverageState::Complete
        } else {
            CoverageState::Partial { files }
        }
    }
    pub(crate) fn observation_ready(&self, reads: &[RelationRead]) -> bool {
        match &self.publication {
            CompletionBehavior::BlockBeforeRun(partials) => {
                blocked(reads, partials) == Blocking::Free
            }
            CompletionBehavior::DropAfterCollect(partials) => files_for(partials, reads).is_empty(),
        }
    }
    pub(crate) fn requires_buffer(&self) -> bool {
        matches!(self.behavior, CompletionBehavior::DropAfterCollect(_))
    }
    pub(crate) fn filter_rows(
        &self,
        operation: &Operation,
        needs: &[RelationRead],
        rows: &mut Table,
    ) {
        if let CompletionBehavior::DropAfterCollect(partials) = &self.behavior {
            let uncertain = files_for(partials, needs);
            let kinds = rows.kinds.clone();
            drop_uncertain(operation, &kinds, rows, &uncertain);
        }
    }
    pub(crate) fn allows_region(&self, needs: &[RelationRead]) -> bool {
        match &self.behavior {
            CompletionBehavior::BlockBeforeRun(partials) => {
                blocked(needs, partials) == Blocking::Free
            }
            CompletionBehavior::DropAfterCollect(_) => true,
        }
    }
    pub(crate) fn for_rule(&self, rule: RuleIndex) -> Self {
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
}
