//! The observation describes the standard mini-language, not a custom value's `__format__`.
use super::{StandardSpecOutcome as Outcome, recognize};
use crate::domain::standard_format_spec::types::{
    FormatSpecText, PresentationCharacter, StandardFormatTarget,
};
use crate::domain::standard_library::types::{PythonMajor, PythonMinor, TargetVersion};

/// Python 3 at the given minor release, as the recognizer is asked about it.
fn target(minor: i64) -> StandardFormatTarget {
    TargetVersion {
        major: PythonMajor(3),
        minor: PythonMinor(minor),
    }
    .into()
}

fn static_clauses_and_unicode_fill_are_recognized() {
    for spec in [
        "",
        "s",
        "*^30s",
        "🦀>8.2f",
        "0=+10,.2f",
        "#x",
        ".3g",
        "٣d",
        ".٢f",
    ] {
        assert_eq!(
            recognize(FormatSpecText(spec), false.into(), target(10)),
            Outcome::Recognized,
            "{spec}"
        );
    }
}

fn unsupported_type_is_distinct_from_other_structural_error() {
    for (spec, expected) in [
        ("z", Outcome::InvalidType(PresentationCharacter('z'))),
        ("*^30y", Outcome::InvalidType(PresentationCharacter('y'))),
        ("}", Outcome::InvalidType(PresentationCharacter('}'))),
        (".2fa", Outcome::OtherError),
        ("ds", Outcome::OtherError),
        (".q", Outcome::OtherError),
        ("٣q", Outcome::InvalidType(PresentationCharacter('q'))),
        ("²d", Outcome::InvalidType(PresentationCharacter('²'))),
    ] {
        assert_eq!(
            recognize(FormatSpecText(spec), false.into(), target(10)),
            expected,
            "{spec}"
        );
    }
}

fn dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately() {
    assert_eq!(
        recognize(FormatSpecText(".{width}g"), true.into(), target(10)),
        Outcome::Dynamic
    );
    assert_eq!(
        recognize(FormatSpecText("y"), false.into(), target(10)),
        Outcome::InvalidType(PresentationCharacter('y'))
    );
}

fn zero_coercion_is_version_bound() {
    assert_eq!(
        recognize(FormatSpecText("z.2f"), false.into(), target(10)),
        Outcome::InvalidType(PresentationCharacter('z'))
    );
    assert_eq!(
        recognize(FormatSpecText("z.2f"), false.into(), target(11)),
        Outcome::Recognized
    );
}

use crate::cases::{Case, CaseName, Expect, Word, run};
type Scenario = fn();
type Failure = std::convert::Infallible;
const CASES: &[Case<Scenario, (), Failure>] = &[
    Case {
        name: CaseName("static_clauses_and_unicode_fill_are_recognized"),
        word: Word::Succeeds,
        input: || static_clauses_and_unicode_fill_are_recognized as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unsupported_type_is_distinct_from_other_structural_error"),
        word: Word::Succeeds,
        input: || unsupported_type_is_distinct_from_other_structural_error as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately"),
        word: Word::Succeeds,
        input: || dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("zero_coercion_is_version_bound"),
        word: Word::Succeeds,
        input: || zero_coercion_is_version_bound as fn(),
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Scenario) -> Result<(), Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
