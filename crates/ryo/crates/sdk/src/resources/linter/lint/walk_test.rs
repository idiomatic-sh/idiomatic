//! The walk of a request that says `Corpus::Walked` skips the directory that holds the store when it is kept beneath the root: where the
//! skip is, however the store and the root are spelled. The expected skips are the old path's, from the trunk before the executor.

use super::walk::store_skip;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// How a row spells where the store is.
#[derive(Clone, Copy, Debug)]
enum Store {
    /// As written.
    Written(&'static str),
    /// As written, made absolute from the current directory.
    Absolute(&'static str),
}

type Input = (Store, &'static str);
/// The skip the walk is given for the store, or `no skip`.
type Value = Cow<'static, str>;
/// Why the row ends in error.
type Failure = Cow<'static, str>;

/// The rows: the store and the root as spelled, and the skip that results.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_store_beneath_the_current_directory_root_is_skipped"),
        word: Word::Succeeds,
        input: || (Store::Written(".ryo_cache"), "."),
        expect: Expect::Value(Cow::Borrowed("./.ryo_cache")),
    },
    Case {
        name: CaseName("a_store_and_a_root_spelled_differently_name_the_same_directory"),
        word: Word::Succeeds,
        input: || (Store::Written("proj/.ryo_cache"), "./proj"),
        expect: Expect::Value(Cow::Borrowed("./.ryo_cache")),
    },
    Case {
        name: CaseName("a_store_with_a_parent_component_names_the_directory_it_reaches"),
        word: Word::Succeeds,
        input: || (Store::Written("proj/x/../.ryo_cache"), "proj/"),
        expect: Expect::Value(Cow::Borrowed("./.ryo_cache")),
    },
    Case {
        name: CaseName("an_absolute_store_beneath_a_relative_root_is_skipped"),
        word: Word::Succeeds,
        input: || (Store::Absolute("proj/.ryo_cache"), "proj"),
        expect: Expect::Value(Cow::Borrowed("./.ryo_cache")),
    },
    Case {
        name: CaseName("a_store_deeper_beneath_the_root_is_skipped_by_its_whole_path"),
        word: Word::Succeeds,
        input: || (Store::Written("src/.cache/ryo"), "."),
        expect: Expect::Value(Cow::Borrowed("./src/.cache/ryo")),
    },
    Case {
        name: CaseName("a_store_outside_the_root_is_skipped_from_nothing"),
        word: Word::Succeeds,
        input: || (Store::Written("elsewhere/cache"), "proj"),
        expect: Expect::Value(Cow::Borrowed("no skip")),
    },
    Case {
        name: CaseName("a_store_above_the_root_is_skipped_from_nothing"),
        word: Word::Succeeds,
        input: || (Store::Written("../cache"), "."),
        expect: Expect::Value(Cow::Borrowed("no skip")),
    },
    Case {
        name: CaseName("a_store_that_is_the_root_is_skipped_from_nothing"),
        word: Word::Succeeds,
        input: || (Store::Written("proj"), "proj/"),
        expect: Expect::Value(Cow::Borrowed("no skip")),
    },
];

/// Calls the walk's skip over one row.
fn subject((store, root): Input) -> Result<Value, Failure> {
    let store = match store {
        Store::Written(text) => std::path::PathBuf::from(text),
        Store::Absolute(text) => std::path::absolute(text).map_err(|error| Cow::Owned(error.to_string()))?,
    };
    Ok(store_skip(&store, std::path::Path::new(root))
        .and_then(|skip| skip.to_str().map(|text| Cow::Owned(text.to_owned())))
        .unwrap_or(Cow::Borrowed("no skip")))
}

#[test]
fn succeeds() {
    assert!(!CASES.is_empty(), "the walk has cases");
    run(CASES, Word::Succeeds, subject);
}
