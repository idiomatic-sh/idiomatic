//! The `files` service's cases, on values alone.

use super::service::with_separator;
use super::types::{NativeSeparator, PlatformPath};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::files::types::FilePath;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A path written with the platform's separator.
    Native,
    /// A path that already uses the engine's separator.
    Engine,
    /// A backslash in a name on a platform that does not split on it.
    Odd,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// Whether the path came out as the engine writes it.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_platforms_separator_becomes_the_engines"),
        word: Word::Succeeds,
        input: || Scenario::Native,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_path_in_the_engines_separator_is_unchanged"),
        word: Word::Succeeds,
        input: || Scenario::Engine,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_backslash_stays_where_the_platform_does_not_split_on_it"),
        word: Word::Succeeds,
        input: || Scenario::Odd,
        expect: Expect::Value(1),
    },
];

fn written(text: &'static str, native: char) -> FilePath {
    with_separator(PlatformPath(text), NativeSeparator(native))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let answered = match scenario {
        Scenario::Native => {
            written("src\\inbound\\lib.rs", '\\') == FilePath("src/inbound/lib.rs".into())
        }
        Scenario::Engine => {
            written("src/inbound/lib.rs", '/') == FilePath("src/inbound/lib.rs".into())
        }
        Scenario::Odd => written("src/odd\\name.rs", '/') == FilePath("src/odd\\name.rs".into()),
    };
    Ok(usize::from(answered))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
