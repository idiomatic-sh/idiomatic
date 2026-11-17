//! What placement finds over a workspace that composes nothing, and what it refuses of one the compiler refuses, on
//! values alone.

use super::errors::PlacementError;
use super::service::place;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::fixtures::bare;
use crate::domain::entries::fixtures::unwritten;
use crate::domain::findings::service::printed;
use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::fixtures::refusing;
use crate::domain::selectors::types::{PathPattern, Selector, Selectors};

/// What a row places.
#[derive(Clone, Copy, Debug)]
enum Workspace {
    /// A workspace that composes nothing.
    Bare,
    /// The same workspace, its `rules-exclude` of `docs/**` a glob the compiler refuses.
    RefusedExclude,
    /// The same workspace, applying the spec `p/language`, which it did not load.
    UnloadedSpec,
}

type Input = Workspace;
/// The scopes, the placed files and the findings, in words, empty when they are what the row expects.
type Value = String;
type Failure = PlacementError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_workspace_that_composes_nothing_is_one_scope_that_places_nothing"),
        word: Word::Succeeds,
        input: || Workspace::Bare,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rules_exclude_the_compiler_refuses_refuses_the_placement"),
        word: Word::Refuses,
        input: || Workspace::RefusedExclude,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(PlacementError::Selector {
                    entry: "workspace".into(),
                    source: SelectorsError::Pattern {
                        pattern: "docs/**".into(),
                        reason: "refused".into(),
                    },
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_spec_the_workspace_did_not_load_refuses_the_placement"),
        word: Word::Refuses,
        input: || Workspace::UnloadedSpec,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(PlacementError::Unloaded {
                    entry: "p/language".into(),
                    release: "p/language".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn subject(asked: Input) -> Result<Value, Failure> {
    let mut workspace = bare(&["tastemaker.toml", "src/a/lib.txt", "docs/a.md"]);
    match asked {
        Workspace::Bare => {}
        Workspace::RefusedExclude => {
            workspace.section.rules_exclude =
                Selectors(vec![Selector::Glob(PathPattern("docs/**".into()))]);
        }
        Workspace::UnloadedSpec => {
            workspace
                .composition
                .specs
                .push(crate::domain::instance_params::fixtures::entry(
                    std::collections::BTreeMap::new(),
                ))
        }
    }
    let placement = place(&workspace, &refusing, &unwritten)?;
    let said = format!(
        "{} scopes, {} placed, findings: {}",
        placement.scopes.len(),
        placement.placed.len(),
        placement
            .findings
            .iter()
            .map(|finding| printed(&finding.rule).0.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(if said == "1 scopes, 0 placed, findings: " {
        String::new()
    } else {
        said
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
