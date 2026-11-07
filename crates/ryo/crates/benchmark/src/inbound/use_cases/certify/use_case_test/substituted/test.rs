//! Substituted invocations, profiles and measured sources refuse before any launch.
use super::super::registered::fixtures::{Fixture, copy_measured_profile, digest, failed};
use super::fixtures::{Route, foreign_profile, identical, substitution};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

const UNKNOWN_FIELD: &str = "unknown field";
const NAMESPACE: &str = "registered context namespace differs";
const AUTHORITY: &str = "registered authority differs from measured canonical profile";
const ROOT: &str = "selected rule root differs from measured profile";
const QUERY_CONTEXT: &str = "measured query differs from registered context";
const MODULE_CONTEXT: &str = "measured helper differs from tested context";

/// One scenario of a substituted authority or measured source.
#[derive(Clone, Copy)]
enum Scenario {
    /// A byte-identical staged copy of the measured profile.
    Staged,
    /// The document or its context substitutes an invocation or the canonical profile.
    Substitution { change: &'static str, reason: &'static str },
    /// The measured profile identity is substituted via the query directory.
    ForeignProfile,
    /// A profile whose `benchmark.toml` is identical but whose source bytes differ.
    Identical { changed: &'static str, route: Route, reason: &'static str },
}

type Input = Scenario;
type Value = usize;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a byte-identical staged profile keeps registered source authority"),
        word: Word::Succeeds,
        input: || Scenario::Staged,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a substituted executable refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "executable", reason: UNKNOWN_FIELD },
        expect: Expect::Error(Cow::Borrowed(UNKNOWN_FIELD)),
    },
    Case {
        name: CaseName("substituted arguments refuse before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "arguments", reason: UNKNOWN_FIELD },
        expect: Expect::Error(Cow::Borrowed(UNKNOWN_FIELD)),
    },
    Case {
        name: CaseName("a substituted directory refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "directory", reason: UNKNOWN_FIELD },
        expect: Expect::Error(Cow::Borrowed(UNKNOWN_FIELD)),
    },
    Case {
        name: CaseName("a substituted environment refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "environment", reason: UNKNOWN_FIELD },
        expect: Expect::Error(Cow::Borrowed(UNKNOWN_FIELD)),
    },
    Case {
        name: CaseName("a substituted context refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "context", reason: NAMESPACE },
        expect: Expect::Error(Cow::Borrowed(NAMESPACE)),
    },
    Case {
        name: CaseName("an extra context file refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "extra-context-file", reason: NAMESPACE },
        expect: Expect::Error(Cow::Borrowed(NAMESPACE)),
    },
    Case {
        name: CaseName("a foreign canonical profile in the registration refuses before launch"),
        word: Word::Refuses,
        input: || Scenario::Substitution { change: "foreign-profile", reason: AUTHORITY },
        expect: Expect::Error(Cow::Borrowed(AUTHORITY)),
    },
    Case {
        name: CaseName("the measured profile identity cannot be substituted via the query directory"),
        word: Word::Refuses,
        input: || Scenario::ForeignProfile,
        expect: Expect::Error(Cow::Borrowed(AUTHORITY)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured query bytes in certification"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "rules/A/query.ryoql", route: Route::Certify, reason: ROOT },
        expect: Expect::Error(Cow::Borrowed(ROOT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured query bytes in preparation"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "rules/A/query.ryoql", route: Route::Prepare, reason: ROOT },
        expect: Expect::Error(Cow::Borrowed(ROOT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured query bytes with foreign rules"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "rules/A/query.ryoql", route: Route::Rules, reason: QUERY_CONTEXT },
        expect: Expect::Error(Cow::Borrowed(QUERY_CONTEXT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured helper bytes in certification"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "helper.ryoql", route: Route::Certify, reason: ROOT },
        expect: Expect::Error(Cow::Borrowed(ROOT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured helper bytes in preparation"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "helper.ryoql", route: Route::Prepare, reason: ROOT },
        expect: Expect::Error(Cow::Borrowed(ROOT)),
    },
    Case {
        name: CaseName("identical profile toml cannot substitute measured helper bytes with foreign rules"),
        word: Word::Refuses,
        input: || Scenario::Identical { changed: "helper.ryoql", route: Route::Rules, reason: MODULE_CONTEXT },
        expect: Expect::Error(Cow::Borrowed(MODULE_CONTEXT)),
    },
];

fn staged() -> Result<Value, Failure> {
    let f = Fixture::new();
    let staged = f._root.path().join("measured-staged");
    copy_measured_profile(&f, &staged);
    assert_eq!(
        digest(&staged.join("benchmark.toml")),
        digest(&f.profile.join("benchmark.toml"))
    );
    assert_ne!(staged, f.profile);
    let certificate = f
        .certify_profile_rules(&staged, &staged.join("rules"))
        .map_err(|error| failed(&error))?;
    assert_eq!(*f.execution.launches.lock().unwrap(), 2);
    Ok(certificate.cases().0)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Staged => staged(),
        Scenario::Substitution { change, reason } => substitution(change, reason),
        Scenario::ForeignProfile => foreign_profile(AUTHORITY),
        Scenario::Identical { changed, route, reason } => identical(changed, route, reason),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
