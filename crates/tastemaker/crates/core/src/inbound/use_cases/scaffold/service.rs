//! The `scaffold` service: local free-function helpers, private to this use case.

use super::errors::ScaffoldError;
use super::types::{ScaffoldRequest, Workspace};
use crate::domain::captures::types::{CaptureName, CaptureValue, Captures, Template};
use crate::domain::definitions::types::LoadedWorkspace;
use crate::domain::entries::service::{identity, reference, release};
use crate::domain::entries::types::{Count, EntryHeader, EntryIdentity};
use crate::domain::packages::service::release_id;
use crate::domain::packages::types::{DefinitionKind, DefinitionName};
use crate::domain::paths::service::join;
use crate::domain::paths::types::{PathText, RepoPath};
use crate::domain::placement::types::Placement;
use crate::domain::scaffolding::types::{Reads, Staging};
use crate::domain::scopes::service::composition_of;

/// Turns a manifest failure into the error that names `file`.
pub(super) fn manifest_error(
    file: PathText<'_>,
) -> impl Fn(crate::outbound::manifest::errors::ManifestError) -> ScaffoldError + '_ {
    move |source| ScaffoldError::Manifest {
        file: file.0.to_owned(),
        source,
    }
}

/// Where the instance `--at` names is, relative to the repository: refused when it is absolute or climbs out.
pub(super) fn instance_at(
    workspace: &Workspace,
    request: &ScaffoldRequest,
) -> Result<RepoPath, ScaffoldError> {
    if std::path::Path::new(&*request.at.0).is_absolute() {
        return Err(ScaffoldError::InvalidPath {
            path: request.at.0.to_string(),
        });
    }
    join(workspace.found.start(), PathText(&request.at.0)).ok_or_else(|| {
        ScaffoldError::InvalidPath {
            path: request.at.0.to_string(),
        }
    })
}

/// Checks that every condition needed by a draft has all of its rule parameters.
pub(super) fn condition_parameters(
    loaded: &LoadedWorkspace,
    placement: &Placement,
    staged: &Staging,
) -> Result<(), ScaffoldError> {
    for scope in &placement.scopes {
        let composition = composition_of(loaded, &scope.composition);
        for (name, condition) in &composition.conditions {
            let needed = staged.drafts.values().any(|draft| {
            draft.directory == scope.dir
                && matches!(&draft.entry.count, Count::When { condition: waited, .. } if waited == name)
        });
            if !needed {
                continue;
            }
            let release = crate::domain::entries::service::rule_release(&condition.entry);
            if let Some(rule) = loaded.rules.get(&release) {
                let mut problems = Vec::new();
                if crate::domain::applications::service::rule_params(
                    &mut problems,
                    scope,
                    &condition.entry,
                    &rule.params,
                )
                .is_none()
                {
                    return Err(ScaffoldError::Undecided {
                        reason: format!("condition `{}` has unresolved rule parameters", name.0),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Refuses the first name `--set` gives that nothing the entry places reads: a typo would otherwise be dropped.
pub(super) fn unread(request: &ScaffoldRequest, reads: &Reads) -> Result<(), ScaffoldError> {
    match request
        .set
        .keys()
        .find(|name| !reads.0.contains(&CaptureName(name.0.clone())))
    {
        Some(name) => Err(ScaffoldError::Unread {
            name: name.0.to_string(),
            entry: request.entry.0.to_string(),
        }),
        None => Ok(()),
    }
}

/// The names `explain` writes for the entry of `header`, of `kind` and definition `name`: its alias or its definition's
/// name, its identity (`package/name`, `package/spec/name`, with `(alias)` after it), and its definition at its
/// version (`package/name@version`). A name asked for is the entry's when it is one of them.
pub(super) fn spellings(
    header: &EntryHeader,
    kind: DefinitionKind,
    name: &DefinitionName,
) -> [EntryIdentity; 4] {
    let id = release_id(&release(header, kind, name)).0;
    [
        EntryIdentity(reference(header, name).0.to_string()),
        identity(header, kind, name),
        EntryIdentity(format!("{id}@{}", header.version.0)),
        EntryIdentity(id),
    ]
}

/// Gives `values` each capture of `more` it does not hold yet: what is already there wins.
pub(super) fn unset(values: &mut Captures, more: Captures) {
    for (name, value) in more.0 {
        let _: &mut CaptureValue = values.0.entry(name).or_insert(value);
    }
}

/// The first capture `at` leaves unresolved: `--at` names one instance's directory, so a capture left in it is no
/// directory, and writing it would make one named for the capture.
pub(super) fn capture_left(at: Template<'_>) -> Option<CaptureName> {
    crate::domain::captures::service::pieces(at)
        .into_iter()
        .find_map(|piece| match piece {
            crate::domain::captures::types::PatternPiece::Capture(name)
            | crate::domain::captures::types::PatternPiece::Span(name) => {
                Some(CaptureName(name.0.into()))
            }
            crate::domain::captures::types::PatternPiece::Literal(_)
            | crate::domain::captures::types::PatternPiece::Star
            | crate::domain::captures::types::PatternPiece::Question => None,
        })
}
