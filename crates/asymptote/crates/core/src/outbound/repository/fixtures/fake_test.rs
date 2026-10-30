//! The `repository` port's fake runs the port's contract, given the contract's repository as its answers.

use super::super::contract::check;
use super::RepositoryFake;
use crate::domain::operations::types::{OperationText, Revision};
use crate::domain::renames::types::{Introduction, Parent, Rename, RenameEvent};
use crate::domain::violations::types::RepoPath;
use crate::outbound::repository::types::{Renames, TrackedFile};

fn rename(from: &str, to: &str) -> Rename {
    Rename {
        from: RepoPath(from.into()),
        to: RepoPath(to.into()),
    }
}

#[test]
fn succeeds() {
    let lock = RepoPath(format!(".asymptote/operations/{}.lock", "a".repeat(64)).into());
    let fake = RepositoryFake {
        worktree: Ok(Renames(vec![rename("b.rs", "c.rs")])),
        history: vec![
            RenameEvent {
                revision: Revision("first".into()),
                renames: Vec::new(),
            },
            RenameEvent {
                revision: Revision("head".into()),
                renames: vec![rename("a.rs", "b.rs")],
            },
        ],
        introduced: vec![Introduction {
            path: lock.clone(),
            revision: Revision("first".into()),
            parent: Parent::Root,
        }],
        at_other: vec![TrackedFile {
            path: lock,
            text: OperationText(Box::default()),
        }],
        files: vec![RepoPath("c.rs".into())],
        ..RepositoryFake::default()
    };
    check(&fake);
}
