//! The `targets` service's cases: a target is resolved against the file that writes it.

use super::service::{reach_of, resolve_target};
use super::types::{TargetReach, WrittenTarget};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::SourcePath;

/// What a row asks: where a target is resolved to, or what it reaches.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The file, and the target written in it.
    Resolve(&'static str, &'static str),
    /// A target alone.
    Reach(&'static str),
}

/// What a row asks.
type Input = Question;
/// What the target names.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_path_is_resolved_against_the_directory_of_the_file"),
        word: Word::Succeeds,
        input: || Question::Resolve("docs/a.txt", "../guide.txt#setup"),
        expect: Expect::Value("guide.txt#setup"),
    },
    Case {
        name: CaseName("a_path_beside_the_file_stays_in_its_directory"),
        word: Word::Succeeds,
        input: || Question::Resolve("docs/a.txt", "./b/c.txt"),
        expect: Expect::Value("docs/b/c.txt"),
    },
    Case {
        name: CaseName("a_path_from_the_root_is_the_root_of_the_run"),
        word: Word::Succeeds,
        input: || Question::Resolve("docs/a.txt", "/README.txt"),
        expect: Expect::Value("README.txt"),
    },
    Case {
        name: CaseName("a_file_at_the_root_resolves_beside_itself"),
        word: Word::Succeeds,
        input: || Question::Resolve("a.txt", "b.txt"),
        expect: Expect::Value("b.txt"),
    },
    Case {
        name: CaseName("a_fragment_alone_names_a_place_in_the_file"),
        word: Word::Succeeds,
        input: || Question::Resolve("docs/a.txt", "#top"),
        expect: Expect::Value("docs/a.txt#top"),
    },
    Case {
        name: CaseName("a_target_with_a_scheme_stays_as_written"),
        word: Word::Succeeds,
        input: || Question::Resolve("docs/a.txt", "https://example.org/x"),
        expect: Expect::Value("https://example.org/x"),
    },
    Case {
        name: CaseName("a_target_with_a_host_stays_as_written"),
        word: Word::Succeeds,
        input: || Question::Resolve("docs/a.txt", "//cdn.example/x"),
        expect: Expect::Value("//cdn.example/x"),
    },
    Case {
        name: CaseName("a_colon_after_a_digit_first_is_no_scheme"),
        word: Word::Succeeds,
        input: || Question::Resolve("docs/a.txt", "1a:b"),
        expect: Expect::Value("docs/1a:b"),
    },
    Case {
        name: CaseName("a_path_that_leaves_the_run_says_so"),
        word: Word::Succeeds,
        input: || Question::Resolve("a.txt", "../x.txt"),
        expect: Expect::Value("../x.txt"),
    },
    Case {
        name: CaseName("a_query_stays_as_written"),
        word: Word::Succeeds,
        input: || Question::Resolve("a.txt", "b.txt?raw=1"),
        expect: Expect::Value("b.txt?raw=1"),
    },
    Case {
        name: CaseName("an_empty_target_stays_empty"),
        word: Word::Succeeds,
        input: || Question::Resolve("a.txt", ""),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_path_reaches_inside_the_run"),
        word: Word::Succeeds,
        input: || Question::Reach("docs/a.txt#setup"),
        expect: Expect::Value("inside"),
    },
    Case {
        name: CaseName("a_fragment_alone_reaches_inside_the_run"),
        word: Word::Succeeds,
        input: || Question::Reach("#top"),
        expect: Expect::Value("inside"),
    },
    Case {
        name: CaseName("a_scheme_reaches_outside_the_run"),
        word: Word::Succeeds,
        input: || Question::Reach("https://cdn.example/"),
        expect: Expect::Value("outside"),
    },
    Case {
        name: CaseName("a_host_reaches_outside_the_run"),
        word: Word::Succeeds,
        input: || Question::Reach("//cdn.example/x.js"),
        expect: Expect::Value("outside"),
    },
];

fn subject(question: Input) -> Result<Value, Failure> {
    Ok(match question {
        Question::Resolve(located, written) => Box::leak(
            resolve_target(SourcePath(located), WrittenTarget(written))
                .0
                .into_boxed_str(),
        ),
        Question::Reach(written) => match reach_of(WrittenTarget(written)) {
            TargetReach::Inside => "inside",
            TargetReach::Outside => "outside",
        },
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
