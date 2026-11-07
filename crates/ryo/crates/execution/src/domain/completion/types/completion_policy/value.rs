//! Required scope-specific completion. Only successful evaluation can mint rows;
//! unavailable inputs retain provenance and can never become a complete empty table.
use crate::domain::{
    coverage::{
        service::{drop_inapplicable, drop_uncertain},
        types::{Partial, Partials, RuleIndex},
    },
    trust::{
        service::{blocked, files_for, partial_files_for, skipped_files},
        types::{Blocking, RelationRead},
    },
};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::{plans::types::Operation, rows::types::Table};

use super::super::{
    CoverageState, ObservationAdmission, RegionAdmission, RowRetention, ScopeIdentity,
};
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
            ScopeIdentity::File { file, .. } => {
                crate::domain::completion::service::partial_any(vec![Some(file)])
            }
            ScopeIdentity::Project { .. } => {
                crate::domain::completion::service::partial_any(vec![None])
            }
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
            // A skipped file reaches the answer through the readiness of its inputs, which knows what the answer is keyed by.
            CompletionBehavior::DropAfterCollect(partials) => partial_files_for(partials, reads),
        };
        if files.is_empty() {
            CoverageState::Complete
        } else {
            CoverageState::Partial {
                files,
                lacks: Vec::new(),
            }
        }
    }
    /// The coverage of a relation as it is read, its rows in the relation's own columns: what the partial relations leave
    /// missing is kept by file, so a consumer that rules the missing kinds out is not made partial.
    pub(crate) fn relation_coverage(
        &self,
        relation: ryo_relations::domain::relations::types::Relation,
    ) -> CoverageState {
        use crate::domain::completion::service::{missing_of_partials, partial_of_lacks};
        use crate::domain::completion::types::FileLacks;
        let read = [RelationRead {
            relation,
            kinds: vec![],
        }];
        let coverage = self.coverage(&read);
        let CoverageState::Partial { files, .. } = coverage else {
            return coverage;
        };
        let missing = |file: Option<FileId>| match &self.publication {
            CompletionBehavior::BlockBeforeRun(partials) => {
                missing_of_partials(partials.iter().filter(|each| each.relation == relation))
            }
            CompletionBehavior::DropAfterCollect(partials) => missing_of_partials(
                partials
                    .entries
                    .iter()
                    .filter(|each| {
                        each.file
                            == file.map_or(
                                crate::domain::coverage::types::PartialPlace::Repository,
                                crate::domain::coverage::types::PartialPlace::File,
                            )
                            && each.partial.relation == relation
                    })
                    .map(|each| &each.partial),
            ),
        };
        partial_of_lacks(
            files
                .into_iter()
                .map(|file| FileLacks {
                    file,
                    missing: missing(file),
                })
                .collect(),
        )
    }
    pub(crate) fn observation_admission(&self, reads: &[RelationRead]) -> ObservationAdmission {
        let ready = match &self.publication {
            CompletionBehavior::BlockBeforeRun(partials) => {
                blocked(reads, partials) == Blocking::Free
            }
            CompletionBehavior::DropAfterCollect(partials) => files_for(partials, reads).is_empty(),
        };
        if ready {
            ObservationAdmission::Ready
        } else {
            ObservationAdmission::Incomplete
        }
    }
    pub(crate) fn row_retention(&self) -> RowRetention {
        match self.behavior {
            CompletionBehavior::DropAfterCollect(_) => RowRetention::Required,
            CompletionBehavior::BlockBeforeRun(_) => RowRetention::Unnecessary,
        }
    }
    pub(crate) fn filter_rows(
        &self,
        operation: &Operation,
        needs: &[RelationRead],
        rows: &mut Table,
    ) {
        if let CompletionBehavior::DropAfterCollect(partials) = &self.behavior {
            let kinds = rows.kinds.clone();
            drop_uncertain(operation, &kinds, rows, &partial_files_for(partials, needs));
            // A skipped file is out of scope for a count not keyed by file; it withholds only its own rows otherwise.
            drop_inapplicable(operation, &kinds, rows, &skipped_files(partials));
        }
    }
    pub(crate) fn region_admission(&self, needs: &[RelationRead]) -> RegionAdmission {
        let allowed = match &self.behavior {
            CompletionBehavior::BlockBeforeRun(partials) => {
                blocked(needs, partials) == Blocking::Free
            }
            CompletionBehavior::DropAfterCollect(_) => true,
        };
        if allowed {
            RegionAdmission::Allowed
        } else {
            RegionAdmission::Blocked
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
