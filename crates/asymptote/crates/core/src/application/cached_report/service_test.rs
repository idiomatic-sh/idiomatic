//! Freshness follows the recorded files, the manifest among them, and shares the retained report without copying it.

use super::errors::CachedReportError;
use super::service::CachedReport;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::judgement::types::Failure as ToolFailure;
use crate::domain::operations::types::Revision;
use crate::domain::renames::types::Head;
use crate::domain::reports::types::{Digest, FileRecord, Recorded, Report};
use crate::domain::violations::types::{RepoPath, ToolId};
use crate::outbound::check_store::types::{KeptReport, Stored};
use crate::outbound::sources::fixtures::fake::SourcesFake;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
enum Scene {
    Fresh,
    Changed,
    Missing,
    HeadMoved,
    Reconfigured,
    ToolFailed,
}
type Input = Scene;
type Value = bool;
type Failure = CachedReportError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("fresh_inputs_return_the_same_report_allocation"),
        word: Word::Succeeds,
        input: || Scene::Fresh,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("substituting_changed_sources_refuses_the_report"),
        word: Word::Refuses,
        input: || Scene::Changed,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(CachedReportError::Moved {
                    first: "a.rs".into(),
                    count: 1,
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_absent_report_cannot_authorize_a_ratchet"),
        word: Word::Refuses,
        input: || Scene::Missing,
        expect: Expect::error(CachedReportError::NoReport),
    },
    Case {
        name: CaseName("a_changed_head_refuses_the_previous_report"),
        word: Word::Refuses,
        input: || Scene::HeadMoved,
        expect: Expect::error(CachedReportError::HeadMoved),
    },
    Case {
        name: CaseName("a_substituted_manifest_refuses_the_previous_report"),
        word: Word::Refuses,
        input: || Scene::Reconfigured,
        expect: Expect::error(CachedReportError::ManifestMoved),
    },
    Case {
        name: CaseName("a_failed_tool_cannot_authorize_a_ratchet"),
        word: Word::Refuses,
        input: || Scene::ToolFailed,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(CachedReportError::ToolsFailed {
                    first: "lint".into(),
                    count: 1,
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];
/// The records of `a.rs` and the manifest, as the check that kept the report stamped them.
fn recorded() -> Recorded {
    Recorded {
        files: [("a.rs", "original"), ("asymptote.toml", "tools")]
            .into_iter()
            .map(|(path, stamp)| {
                (
                    RepoPath(path.into()),
                    FileRecord {
                        digest: Digest(stamp.into()),
                        ..FileRecord::default()
                    },
                )
            })
            .collect(),
        ..Recorded::default()
    }
}

fn subject(scene: Input) -> Result<Value, Failure> {
    let report = Arc::new(Report {
        head: Head::Detached,
        scanned: if matches!(scene, Scene::ToolFailed) {
            vec![crate::domain::reports::service::failed(
                &ToolId("lint".into()),
                ToolFailure::Signalled {
                    stderr: crate::domain::judgement::types::Tail::default(),
                },
            )]
        } else {
            Vec::new()
        },
    });
    let stored = Stored {
        report: if matches!(scene, Scene::Missing) {
            KeptReport::Nothing
        } else {
            KeptReport::Kept(Arc::clone(&report))
        },
        recorded: recorded(),
    };
    let sources = match scene {
        Scene::Changed => SourcesFake::holding(&[("a.rs", "edited"), ("asymptote.toml", "tools")]),
        Scene::Reconfigured => {
            SourcesFake::holding(&[("a.rs", "original"), ("asymptote.toml", "new-tool")])
        }
        Scene::Fresh | Scene::Missing | Scene::HeadMoved | Scene::ToolFailed => {
            SourcesFake::holding(&[("a.rs", "original"), ("asymptote.toml", "tools")])
        }
    };
    let head = if matches!(scene, Scene::HeadMoved) {
        Head::Commit(Revision("new-head".into()))
    } else {
        Head::Detached
    };
    CachedReport::new(Arc::new(sources))
        .current(&stored, &head)
        .map(|current| Arc::ptr_eq(&current, &report))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
