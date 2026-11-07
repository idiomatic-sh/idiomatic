//! The part kind selects the directories of one spec's instances, directly beneath the place or at any depth.

use super::super::interface::SelectorsItem as _;
use super::item::PartItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::Interpolations;
use crate::domain::packages::types::DefinitionName;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::types::{
    PartInstance, PartSelector, PathTest, PatternFailure, Recursion, Resolution, Resolved,
    SelectionContext, Selector, Spelling,
};

/// The spec selected and how deep.
type Input = (&'static str, Recursion);
/// The directories selected, joined by `,`.
type Value = String;
type Failure = ();

fn compile(_: &Spelling) -> Result<std::sync::Arc<dyn PathTest>, PatternFailure> {
    Err(PatternFailure("a part selector compiles nothing".into()))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_direct_part_skips_an_instance_nested_in_another"),
        word: Word::Succeeds,
        input: || ("use-case", Recursion::Direct),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_recursive_part_reaches_any_depth"),
        word: Word::Succeeds,
        input: || ("use-case", Recursion::Recursive),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_part_no_instance_has_selects_nothing"),
        word: Word::Refuses,
        input: || ("item", Recursion::Recursive),
        expect: Expect::error(()),
    },
];

fn instance(directory: &str, spec: &str) -> PartInstance {
    PartInstance {
        directory: RepoPath(directory.into()),
        spec: DefinitionName(spec.into()),
    }
}

fn subject((part, recursion): Input) -> Result<Value, Failure> {
    let instances = [
        instance("core/src/inbound/use_cases/check", "use-case"),
        instance("core/src/domain/x", "concept"),
        instance("core/src/domain/x/inner", "use-case"),
    ];
    let base = RepoPath("core".into());
    let context = SelectionContext {
        base: &base,
        values: &Interpolations::default(),
        instances: &instances,
        placed: &[],
        compile: &compile,
    };
    let selector = Selector::Part(PartSelector {
        part: DefinitionName(part.into()),
        recursion,
    });
    let Ok(Resolution::Resolved(Resolved::Paths(found))) = PartItem.resolve(&selector, &context)
    else {
        return Err(());
    };
    if found.is_empty() {
        return Err(());
    }
    let answer = found
        .into_iter()
        .map(|path| path.0.into_string())
        .collect::<Vec<_>>()
        .join(",");
    let expected = match recursion {
        Recursion::Direct => "core/src/inbound/use_cases/check",
        Recursion::Recursive => "core/src/domain/x/inner,core/src/inbound/use_cases/check",
    };
    Ok(if answer == expected {
        String::new()
    } else {
        answer
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
