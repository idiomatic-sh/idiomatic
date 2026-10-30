//! The shape of what git prints, read into the core's types: `-z` output, so a path with a newline, a quote or a byte
//! that is no UTF-8 is never unescaped wrongly. Each wire shape is a `<Name>Schema` and its conversion an `impl From`
//! or `impl TryFrom`.

use super::constants::{BEFORE_PATH, FIELD, NEGATED, RECORD, UNMATCHED};
use crate::domain::operations::types::Revision;
use crate::domain::renames::types::{Introduction, Parent, Rename, RenameEvent};
use crate::domain::violations::service::normalize;
use crate::domain::violations::types::RepoPath;
use crate::outbound::repository::errors::RepositoryError;

/// Why git's output was refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum GitSchemaError {
    /// A section has no separator after its commit.
    #[error("git log printed a commit without its separator")]
    Separator,
    /// A commit is no full object id.
    #[error("git log printed {0:?}, which is no object id")]
    ObjectId(String),
}

/// What `git diff --name-status -z` printed.
pub(super) struct NameStatusSchema<'a>(pub &'a str);

/// What `git log --format=%x1e%H%x00 --name-status -z` printed.
pub(super) struct HistorySchema<'a>(pub &'a str);

/// What `git log --format=%x1e%H%x00%P%x00 --name-only --diff-filter=A -z` printed.
pub(super) struct AddedSchema<'a>(pub &'a str);

/// Whether `value` is a full object id.
pub(super) fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// A path git printed, in the normalized form findings are keyed by.
fn path(raw: &str) -> RepoPath {
    normalize(&RepoPath(raw.into()))
}

/// A rename is its status, the old path and the new; every other status carries one path, which is consumed so the
/// fields stay in step.
impl From<NameStatusSchema<'_>> for Vec<Rename> {
    fn from(schema: NameStatusSchema<'_>) -> Self {
        let mut fields = schema.0.split('\0').filter(|field| !field.is_empty());
        let mut found = Self::new();
        while let Some(status) = fields.next() {
            if status.starts_with('R') {
                if let (Some(from), Some(to)) = (fields.next(), fields.next()) {
                    found.push(Rename {
                        from: path(from),
                        to: path(to),
                    });
                }
            } else {
                let _: Option<&str> = fields.next();
            }
        }
        found
    }
}

impl TryFrom<HistorySchema<'_>> for Vec<RenameEvent> {
    type Error = GitSchemaError;

    fn try_from(schema: HistorySchema<'_>) -> Result<Self, Self::Error> {
        schema
            .0
            .split(RECORD)
            .filter(|section| !section.is_empty())
            .map(|section| {
                let (revision, changes) =
                    section.split_once('\0').ok_or(GitSchemaError::Separator)?;
                let revision = revision.trim();
                if !is_object_id(revision) {
                    return Err(GitSchemaError::ObjectId(revision.to_owned()));
                }
                Ok(RenameEvent {
                    revision: Revision(revision.into()),
                    renames: NameStatusSchema(changes.trim_start_matches('\n')).into(),
                })
            })
            .collect()
    }
}

impl TryFrom<AddedSchema<'_>> for Vec<Introduction> {
    type Error = GitSchemaError;

    fn try_from(schema: AddedSchema<'_>) -> Result<Self, Self::Error> {
        let mut found = Self::new();
        for section in schema.0.split(RECORD).filter(|section| !section.is_empty()) {
            let (revision, rest) = section.split_once('\0').ok_or(GitSchemaError::Separator)?;
            let revision = revision.trim();
            if !is_object_id(revision) {
                return Err(GitSchemaError::ObjectId(revision.to_owned()));
            }
            let (parents, paths) = rest.split_once('\0').ok_or(GitSchemaError::Separator)?;
            let parent = parents
                .split_whitespace()
                .next()
                .map_or(Parent::Root, |parent| {
                    Parent::Commit(Revision(parent.into()))
                });
            for added in paths
                .trim_start_matches('\n')
                .split('\0')
                .filter(|added| !added.is_empty())
            {
                found.push(Introduction {
                    path: path(added),
                    revision: Revision(revision.into()),
                    parent: parent.clone(),
                });
            }
        }
        Ok(found)
    }
}

/// Git's output refused, as the port's failure.
fn failed(error: &GitSchemaError) -> RepositoryError {
    RepositoryError::Failed {
        reason: error.to_string(),
    }
}

/// The renames `git diff --name-status -z` printed.
pub(super) fn renames(raw: &str) -> Vec<Rename> {
    NameStatusSchema(raw).into()
}

/// The commits and renames `git log` printed.
///
/// # Errors
///
/// `Failed` when it is not the shape asked for.
pub(super) fn history(raw: &str) -> Result<Vec<RenameEvent>, RepositoryError> {
    HistorySchema(raw)
        .try_into()
        .map_err(|error| failed(&error))
}

/// The files each commit added, as `git log` printed them.
///
/// # Errors
///
/// `Failed` when it is not the shape asked for.
pub(super) fn introductions(raw: &str) -> Result<Vec<Introduction>, RepositoryError> {
    AddedSchema(raw).try_into().map_err(|error| failed(&error))
}

/// Each path of a `-z` list, normalized.
pub(super) fn paths(raw: &str) -> Vec<RepoPath> {
    raw.split('\0')
        .filter(|listed| !listed.is_empty())
        .map(path)
        .collect()
}

/// The paths `git check-ignore --verbose --non-matching` says are ignored: a pattern matched each, and it is no
/// negation.
pub(super) fn ignored(raw: &str) -> Vec<&str> {
    raw.lines()
        .filter_map(|line| line.split_once(BEFORE_PATH))
        .filter(|(pattern, _)| {
            !pattern.starts_with(UNMATCHED)
                && pattern
                    .rsplit_once(FIELD)
                    .is_none_or(|(_, pattern)| !pattern.starts_with(NEGATED))
        })
        .map(|(_, path)| path)
        .collect()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
