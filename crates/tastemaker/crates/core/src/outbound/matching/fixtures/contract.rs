//! What every `matching` adapter must pass.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::selectors::types::{Binding, Coverage, GlobIndex, PathRest, Spelling};
use crate::outbound::matching::interface::MatchingPort;

/// A glob, with no expression, and a path it is asked about.
type Input = (&'static str, &'static str);
/// Whether the path passes.
type Value = Coverage;
/// Why a row could not answer: its glob did not compile.
type Failure = ();

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_literal_glob_passes_its_own_path"),
        word: Word::Succeeds,
        input: || ("src/lib.txt", "src/lib.txt"),
        expect: Expect::Value(Coverage::Covered),
    },
    Case {
        name: CaseName("a_star_stays_within_one_segment"),
        word: Word::Succeeds,
        input: || ("src/*", "src/a/b.txt"),
        expect: Expect::Value(Coverage::Uncovered),
    },
    Case {
        name: CaseName("a_double_star_crosses_segments"),
        word: Word::Succeeds,
        input: || ("src/**", "src/a/b.txt"),
        expect: Expect::Value(Coverage::Covered),
    },
];

/// Whether `path` passes `glob`, compiled alone and in a set, which must agree.
fn answer(adapter: &impl MatchingPort, (glob, path): Input) -> Result<Value, Failure> {
    let spelling = Spelling {
        glob: glob.into(),
        binder: Binding::GlobOnly,
        groups: Vec::new(),
    };
    let alone = adapter
        .compile(&spelling)
        .map_err(|_refused| ())?
        .passes(PathRest(path));
    // The glob in a set with another passes the paths it passes alone.
    let set = adapter
        .compile_set(&[
            crate::domain::selectors::types::SetGlob("**/never-a-file".into()),
            crate::domain::selectors::types::SetGlob(glob.into()),
        ])
        .map_err(|_refused| ())?;
    let mut passing = Vec::new();
    set.passing(PathRest(path), &mut passing);
    let together = if passing == [GlobIndex(1)] {
        Coverage::Covered
    } else {
        Coverage::Uncovered
    };
    if together == alone {
        Ok(alone)
    } else {
        Err(())
    }
}

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl MatchingPort) {
    run(CASES, Word::Succeeds, |input| answer(adapter, input));
}
