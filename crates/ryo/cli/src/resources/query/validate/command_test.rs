//! What `query validate` promises its caller, one row of `CASES` for each case it has.

use super::command::{ValidateArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;

/// What a row gives the command: the command line, one argument at a time.
type Input = Vec<&'static str>;
/// How many lines the command prints when it succeeds.
type Value = usize;
/// How the command ends when it does not succeed.
type Failure = Outcome;

const VALID: &str =
    "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");";
const MISSPELLED: &str =
    "let functions = selekt * from \"function\"; emit functions subject \"file\" key(\"span\");";

/// A comparison of the kind of a node with a word that is no kind of any language.
const MISTYPED_KIND: &str = "let nodes = select * from \"syntax-tree\" where kind = \"functoin\"; emit nodes subject \"file\" key(\"span\");";

/// A query of a relation only a host declares: the command line declares none, so it refuses it where the SDK, given the declaration,
/// accepts it; both ask the one validate use case (its rows `a_relation_the_host_declared_is_read_as_ryos_own_are` and
/// `a_relation_the_host_did_not_declare_is_refused`).
const HOSTED: &str =
    "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"spec\");";

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: ValidateArguments,
}

/// The rows of this command's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("prints_ok_for_valid_text"),
        word: Word::Succeeds,
        input: || vec!["--text", VALID],
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("prints_the_error_with_its_span"),
        word: Word::Refuses,
        input: || vec!["--text", MISSPELLED],
        expect: Expect::Error(Outcome::Error),
    },
    Case {
        name: CaseName("a_kind_no_linked_language_has_is_refused"),
        word: Word::Refuses,
        input: || vec!["--text", MISTYPED_KIND],
        expect: Expect::Error(Outcome::Error),
    },
    Case {
        name: CaseName("a_relation_only_a_host_declares_is_refused_as_the_core_refuses_it"),
        word: Word::Refuses,
        input: || vec!["--text", HOSTED],
        expect: Expect::Error(Outcome::Error),
    },
    Case {
        name: CaseName("a_blank_text_is_refused"),
        word: Word::Refuses,
        input: || vec!["--text", " "],
        expect: Expect::Error(Outcome::Failure),
    },
];

/// Runs `query validate` on one command line and returns what it printed or the outcome it ended with. The SDK, asked the same text with
/// no host declaring anything, accepts it exactly when the command does: both ask the one validate use case.
fn subject(line: Input) -> Result<Value, Failure> {
    let text = line
        .iter()
        .position(|word| *word == "--text")
        .and_then(|place| line.get(place + 1))
        .copied();
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line))
        .map_err(|_refused| Outcome::Error)?;
    let answered = answer(parsed.arguments).map(|answer| answer.to_string().lines().count());
    // A kind is checked against the languages linked, and the SDK here is given none: that row is the command line's alone.
    if let Some(text) = text.filter(|text| !text.trim().is_empty() && *text != MISTYPED_KIND) {
        let sdk = ryo_sdk::query::validate::entry::validate(
            &ryo_sdk::linter::Languages::default(),
            ryo_sdk::query::ValidateRequest {
                observations: Vec::new(),
                query: ryo_sdk::query::RyoQlQuery::try_from(text).expect("a query"),
                hosted: Vec::new(),
            },
        );
        assert!(
            matches!((&answered, &sdk), (Ok(_), Ok(_)) | (Err(_), Err(_))),
            "the command line and the SDK disagree about `{text}`"
        );
    }
    answered
}

/// The `succeeds` rows: the command line is accepted and the printed answer is the expected one.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The `refuses` rows: the command line is refused with the expected outcome.
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
