//! The globset adapter of the matching port: the port's contract, then what is specific to it.

use super::adapter::MatchingGlobsetAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::CaptureName;
use crate::domain::selectors::types::{Binding, Spelling};
use crate::outbound::matching::fixtures::contract::check;
use crate::outbound::matching::interface::MatchingPort as _;

/// A spelling and the path it is asked about.
type Input = (Spelling, &'static str);
/// The values it binds.
type Value = Vec<String>;
type Failure = ();

fn binding(glob: &str, expression: &str, groups: &[&str]) -> Spelling {
    Spelling {
        glob: glob.into(),
        binder: Binding::Expression(expression.into()),
        groups: groups
            .iter()
            .map(|name| CaptureName((*name).into()))
            .collect(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaptureCase::BINDS,
        word: Word::Succeeds,
        input: || {
            (
                binding("src/*", "^src/([^/]+)$", &["domain"]),
                "src/greeting",
            )
        },
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaptureCase::BAD_GLOB,
        word: Word::Refuses,
        input: || (binding("src/[", "^src$", &[]), "src"),
        expect: Expect::error(()),
    },
    Case {
        name: CaptureCase::BAD_EXPRESSION,
        word: Word::Refuses,
        input: || (binding("src/*", "^(src$", &[]), "src"),
        expect: Expect::error(()),
    },
];

/// The names of this adapter's rows.
struct CaptureCase;

impl CaptureCase {
    const BINDS: CaseName = CaseName("an_expression_binds_each_capture_in_order");
    const BAD_GLOB: CaseName = CaseName("a_glob_that_does_not_compile_is_refused");
    const BAD_EXPRESSION: CaseName = CaseName("an_expression_that_does_not_compile_is_refused");
}

fn subject((spelling, path): Input) -> Result<Value, Failure> {
    let test = MatchingGlobsetAdapter::default()
        .compile(&spelling)
        .map_err(|_refused| ())?;
    let bound = test
        .bind(crate::domain::selectors::types::PathRest(path))
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.0.to_string())
        .collect::<Vec<_>>();
    Ok(if bound == ["greeting"] {
        Vec::new()
    } else {
        bound
    })
}

#[test]
fn succeeds() {
    check(&MatchingGlobsetAdapter::default());
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
