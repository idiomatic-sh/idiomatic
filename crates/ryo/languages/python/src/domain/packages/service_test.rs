//! The `packages` service's cases: names and requirements.

use super::service::{normalized, parsed, specifier_of};
use super::types::{PackageName, Specifier, Written};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dependencies::service::{pep440_exactness, poetry_exactness};

/// What a row asks: which of the service's questions, and its text.
type Input = (Question, &'static str);
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The questions the service answers.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The name, normalized.
    Normalized,
    /// The requirement: its name, as written, normalized, and its marker.
    Parsed,
    /// The version a PEP 508 requirement asks for, and whether it admits one version only.
    Pep440,
    /// The same of a Poetry constraint.
    Poetry,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_is_lower_case_with_each_run_of_separators_one_dash"),
        word: Word::Succeeds,
        input: || (Question::Normalized, "Friendly__Bard.-Two"),
        expect: Expect::Value("friendly-bard-two"),
    },
    Case {
        name: CaseName("a_requirement_is_read_for_its_name_and_its_marker"),
        word: Word::Succeeds,
        input: || {
            (
                Question::Parsed,
                "Requests[security] (>=2.8.1) ; python_version >= '3.9'",
            )
        },
        expect: Expect::Value("Requests | requests | python_version >= '3.9'"),
    },
    Case {
        name: CaseName("a_requirement_with_a_version_and_no_marker_has_none"),
        word: Word::Succeeds,
        input: || (Question::Parsed, "lib.two>=1.0,<2"),
        expect: Expect::Value("lib.two | lib-two | "),
    },
    Case {
        name: CaseName("a_pin_is_its_version_and_exact"),
        word: Word::Succeeds,
        input: || (Question::Pep440, "fastapi==0.115.0"),
        expect: Expect::Value("==0.115.0 | exact"),
    },
    Case {
        name: CaseName(
            "extras_parentheses_and_a_marker_are_not_the_version_and_a_pin_in_a_conjunction_is_exact",
        ),
        word: Word::Succeeds,
        input: || {
            (
                Question::Pep440,
                "Lib_One[fast] (>=1, ==1.4.2) ; python_version < '3.12'",
            )
        },
        expect: Expect::Value(">=1, ==1.4.2 | exact"),
    },
    Case {
        name: CaseName("a_wildcard_pin_is_a_family"),
        word: Word::Succeeds,
        input: || (Question::Pep440, "requests==2.*"),
        expect: Expect::Value("==2.* | a range"),
    },
    Case {
        name: CaseName("arbitrary_equality_is_exact"),
        word: Word::Succeeds,
        input: || (Question::Pep440, "legacy===1.0-custom"),
        expect: Expect::Value("===1.0-custom | exact"),
    },
    Case {
        name: CaseName("a_name_alone_asks_for_no_version"),
        word: Word::Succeeds,
        input: || (Question::Pep440, "httpx"),
        expect: Expect::Value(" | a range"),
    },
    Case {
        name: CaseName("a_direct_reference_asks_for_no_version"),
        word: Word::Succeeds,
        input: || (Question::Pep440, "pkg @ https://example.com/pkg.whl"),
        expect: Expect::Value(" | a range"),
    },
    Case {
        name: CaseName("a_poetry_bare_version_is_exact"),
        word: Word::Succeeds,
        input: || (Question::Poetry, "1.2.3"),
        expect: Expect::Value("1.2.3 | exact"),
    },
    Case {
        name: CaseName("a_poetry_caret_is_a_range"),
        word: Word::Succeeds,
        input: || (Question::Poetry, "^1.2"),
        expect: Expect::Value("^1.2 | a range"),
    },
    Case {
        name: CaseName("poetry_alternatives_are_a_range"),
        word: Word::Succeeds,
        input: || (Question::Poetry, "1.2.3 || 2.0.0"),
        expect: Expect::Value("1.2.3 || 2.0.0 | a range"),
    },
];

fn subject((question, first): Input) -> Value {
    let text = match question {
        Question::Normalized => normalized(&PackageName(first.into())).0.into_string(),
        Question::Parsed => {
            let read = parsed(&Written(first.into()));
            format!("{} | {} | {}", read.written.0, read.name.0, read.marker.0)
        }
        Question::Pep440 => {
            let specifier = specifier_of(&Written(first.into()));
            let exact = pep440_exactness(&specifier);
            shown_version(&specifier, exact)
        }
        Question::Poetry => {
            let specifier = Specifier(first.into());
            let exact = poetry_exactness(&specifier);
            shown_version(&specifier, exact)
        }
    };
    Box::leak(text.into_boxed_str())
}

/// A version and whether it is exact, as a row states them.
fn shown_version(specifier: &Specifier, exact: ryo::languages::Flag) -> String {
    let admits = if exact == ryo::languages::Flag::Raised {
        "exact"
    } else {
        "a range"
    };
    format!("{} | {admits}", specifier.0)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
