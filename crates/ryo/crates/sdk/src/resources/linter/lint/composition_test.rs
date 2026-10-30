//! Proves the real wiring of `linter lint` over fake connections, as rows of cases.

use super::composition::store_skip;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// How a row spells where the store is.
#[derive(Clone, Copy, Debug)]
enum Store {
    /// As written.
    Written(&'static str),
    /// As written, made absolute from the current directory.
    Absolute(&'static str),
}

/// What a row gives the wiring: where the store is kept and where the root is, as the caller spelled them.
type Input = (Store, &'static str);
/// What the wiring builds: the skip the walk is given for the store, or none.
type Value = Option<&'static str>;
/// Why the wiring ends in error.
type Failure = ();

/// The rows of this composition's cases: the store and the root as spelled, and the skip that results.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_store_beneath_the_current_directory_root_is_skipped"),
        word: Word::Succeeds,
        input: || (Store::Written(".ryo_cache"), "."),
        expect: Expect::Value(Some("./.ryo_cache")),
    },
    Case {
        name: CaseName("a_store_and_a_root_spelled_differently_name_the_same_directory"),
        word: Word::Succeeds,
        input: || (Store::Written("proj/.ryo_cache"), "./proj"),
        expect: Expect::Value(Some("./.ryo_cache")),
    },
    Case {
        name: CaseName("a_store_with_a_parent_component_names_the_directory_it_reaches"),
        word: Word::Succeeds,
        input: || (Store::Written("proj/x/../.ryo_cache"), "proj/"),
        expect: Expect::Value(Some("./.ryo_cache")),
    },
    Case {
        name: CaseName("an_absolute_store_beneath_a_relative_root_is_skipped"),
        word: Word::Succeeds,
        input: || (Store::Absolute("proj/.ryo_cache"), "proj"),
        expect: Expect::Value(Some("./.ryo_cache")),
    },
    Case {
        name: CaseName("a_store_deeper_beneath_the_root_is_skipped_by_its_whole_path"),
        word: Word::Succeeds,
        input: || (Store::Written("src/.cache/ryo"), "."),
        expect: Expect::Value(Some("./src/.cache/ryo")),
    },
    Case {
        name: CaseName("a_store_outside_the_root_is_skipped_from_nothing"),
        word: Word::Succeeds,
        input: || (Store::Written("elsewhere/cache"), "proj"),
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("a_store_above_the_root_is_skipped_from_nothing"),
        word: Word::Succeeds,
        input: || (Store::Written("../cache"), "."),
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("a_store_that_is_the_root_is_skipped_from_nothing"),
        word: Word::Succeeds,
        input: || (Store::Written("proj"), "proj/"),
        expect: Expect::Value(None),
    },
];

/// Calls the real derivation the composition gives the walk, over one row's store and root.
fn subject(input: Input) -> Result<Value, Failure> {
    let (store, root) = input;
    let store = match store {
        Store::Written(text) => std::path::PathBuf::from(text),
        Store::Absolute(text) => std::path::absolute(text).map_err(|_refused| ())?,
    };
    Ok(
        store_skip(&store, std::path::Path::new(root)).and_then(|skip| {
            skip.to_str()
                .map(|text| &*Box::leak(Box::<str>::from(text)))
        }),
    )
}

#[test]
fn succeeds() {
    assert!(!CASES.is_empty(), "the composition has cases");
    run(CASES, Word::Succeeds, subject);
}
