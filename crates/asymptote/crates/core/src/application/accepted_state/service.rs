//! Accepted operations and their projections, read through injected ports.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::errors::AcceptedStateError;
use crate::domain::inputs::constants::ROOT;
use crate::domain::manifest::types::Pathspec;
use crate::domain::operations::service::fold;
use crate::domain::operations::types::{Folded, Operation, OperationId, Record, Revision};
use crate::domain::renames::service::{
    history_start, introduced_by, on_branch, operation_of, project,
};
use crate::domain::renames::types::{Basis, Head, HistoryStart, Projection, Rename};
use crate::domain::violations::types::RepoPath;
use crate::outbound::lock::interface::LockPort;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::RepositoryPort;

/// Loads accepted state and prepares rename projections without publishing anything.
#[derive(Debug)]
pub struct AcceptedState {
    /// Where the accepted operations are read and their keys' content addresses come from.
    lock: Arc<dyn LockPort>,
    /// Git: the renames a projection follows and the lock as an earlier revision held it.
    repository: Arc<dyn RepositoryPort>,
}

impl AcceptedState {
    /// Bind the operations and history contracts selected by composition.
    #[must_use]
    pub const fn new(lock: Arc<dyn LockPort>, repository: Arc<dyn RepositoryPort>) -> Self {
        Self { lock, repository }
    }

    /// The lock, folded: a fold is a few milliseconds, so none is kept between runs.
    pub(crate) fn folded(&self) -> Result<Arc<Folded>, AcceptedStateError> {
        Ok(Arc::new(fold(&self.lock.read()?.0)?))
    }

    /// `folded`'s records where they are, when every one is at a path the working tree holds: then no rename moved
    /// one, and git's rename detection is not asked. None when one's file is gone, or the tree cannot be listed.
    fn unmoved(&self, folded: &Folded) -> Option<Projection> {
        let files = self.repository.files(&[Pathspec(ROOT.into())]).ok()?; // ignore: idiomatic.arch/no-ok-on-result - a tree that cannot be listed is the answer that records may have moved: the caller asks git for the renames instead
        let held: BTreeSet<&RepoPath> = files.0.iter().collect();
        folded
            .records
            .iter()
            .all(|record| held.contains(&record.key.path))
            .then(|| Projection {
                records: folded.records.clone(),
                migrations: Vec::new(),
            })
    }

    /// Prepare the check's accepted records, consulting worktree history before reading the lock; when no record's
    /// file moved, the records as the lock holds them, with no rename asked.
    pub(crate) fn current(&self, head: &Head) -> Result<Vec<Record>, AcceptedStateError> {
        let folded = self.folded();
        if let Ok(kept) = &folded
            && let Some(unmoved) = self.unmoved(kept)
        {
            return Ok(unmoved.records);
        }
        let worktree = match head {
            Head::Commit(head) => self.repository.renames_to_worktree(head)?.0,
            Head::Detached => Vec::new(),
        };
        Ok(self.projected_from(head, &*folded?, &worktree)?.records)
    }

    /// Carry the lock's records through the history and worktree renames that apply to them; when no record's file
    /// moved, they stay where they are, with no rename asked.
    pub(crate) fn projected(
        &self,
        head: &Head,
        folded: &Folded,
    ) -> Result<Projection, AcceptedStateError> {
        if let Some(unmoved) = self.unmoved(folded) {
            return Ok(unmoved);
        }
        let worktree = match head {
            Head::Commit(head) => self.repository.renames_to_worktree(head)?.0,
            Head::Detached => Vec::new(),
        };
        self.projected_from(head, folded, &worktree)
    }

    /// Carries `folded`'s records from where they were accepted to `head` and on through `worktree`'s renames, once
    /// those renames are read.
    fn projected_from(
        &self,
        head: &Head,
        folded: &Folded,
        worktree: &[Rename],
    ) -> Result<Projection, AcceptedStateError> {
        let Head::Commit(head) = head else {
            return Ok(Projection {
                records: folded.records.clone(),
                migrations: Vec::new(),
            });
        };
        let introductions =
            absent_is_empty(self.repository.introductions(head, &self.lock.directory()))?.0;
        let introduced = introduced_by(&introductions);
        let history = match history_start(&introductions, &folded.origins, &introduced) {
            Some(start) => absent_is_empty(self.repository.rename_history(&start, head))?.0,
            None => Vec::new(),
        };
        let basis = Basis::Operations {
            origins: &folded.origins,
            introduced: &introduced,
        };
        Ok(project(
            folded.records.clone(),
            head,
            &history,
            worktree,
            basis,
            &|key| self.lock.id(key),
        ))
    }

    /// Accepted records of a comparison branch, carried to the current working tree.
    pub(crate) fn since(
        &self,
        head: &Revision,
        revision: &Revision,
    ) -> Result<Vec<Record>, AcceptedStateError> {
        let worktree = self.repository.renames_to_worktree(head)?.0;
        let base = self.repository.merge_base(head, revision)?;
        let records = on_branch(self.held_at(&base)?, self.held_at(revision)?);
        let history = absent_is_empty(
            self.repository
                .rename_history(&HistoryStart::After(base), head),
        )?
        .0;
        Ok(
            project(records, head, &history, &worktree, Basis::Branch, &|key| {
                self.lock.id(key)
            })
            .records,
        )
    }

    /// The records the lock held at `revision`, read from git's object store rather than the working tree.
    fn held_at(&self, revision: &Revision) -> Result<Vec<Record>, AcceptedStateError> {
        let files = self
            .repository
            .read_all_at(revision, &self.lock.directory())?
            .0;
        let operations = files
            .iter()
            .filter_map(|file| operation_of(&file.path).map(|id| (id, &file.text)))
            .map(|(id, text)| {
                let operation = self.lock.parse(&id, text)?;
                Ok((id, operation))
            })
            .collect::<Result<BTreeMap<OperationId, Operation>, AcceptedStateError>>()?;
        Ok(fold(&operations)?.records)
    }
}

/// A shallow or pruned clone's missing history is absent, not a failed operation.
fn absent_is_empty<T>(answer: Result<T, RepositoryError>) -> Result<T, RepositoryError>
where
    T: Default,
{
    match answer {
        Err(RepositoryError::UnknownRevision { .. }) => Ok(T::default()),
        other => other,
    }
}
