//! The `language` service's cases.

use super::service::{dialect, reads};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::types::Dialect;
use ryo::languages;

/// What a row asks of a path.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Whether the language reads the file at the path.
    Reads(&'static str),
    /// The dialect the file at the path is read in.
    Dialect(&'static str),
}

/// What the service answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// Whether the language reads the file.
    Reads(bool),
    /// The dialect the file is read in.
    Dialect(Dialect),
}

type Input = Scenario;
type Value = Answer;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_typescript_file_is_read"),
        word: Word::Succeeds,
        input: || Scenario::Reads("src/main.ts"),
        expect: Expect::Value(Answer::Reads(true)),
    },
    Case {
        name: CaseName("a_file_of_another_language_is_declined"),
        word: Word::Succeeds,
        input: || Scenario::Reads("README.md"),
        expect: Expect::Value(Answer::Reads(false)),
    },
    Case {
        name: CaseName("a_ts_file_is_plain_typescript"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/main.ts"),
        expect: Expect::Value(Answer::Dialect(Dialect::TypeScript)),
    },
    Case {
        name: CaseName("a_tsx_file_has_jsx"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/view.tsx"),
        expect: Expect::Value(Answer::Dialect(Dialect::Tsx)),
    },
    Case {
        name: CaseName("an_mts_file_is_a_module"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/main.mts"),
        expect: Expect::Value(Answer::Dialect(Dialect::Module)),
    },
    Case {
        name: CaseName("a_cts_file_is_commonjs"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/main.cts"),
        expect: Expect::Value(Answer::Dialect(Dialect::CommonJs)),
    },
    Case {
        name: CaseName("a_d_ts_file_is_declarations"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("types/index.d.ts"),
        expect: Expect::Value(Answer::Dialect(Dialect::Declarations)),
    },
    Case {
        name: CaseName("a_declaration_of_an_arbitrary_extension_is_declarations"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/styles.d.css.ts"),
        expect: Expect::Value(Answer::Dialect(Dialect::Declarations)),
    },
    Case {
        name: CaseName("a_d_mts_file_is_the_declarations_of_a_module"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("types/index.d.mts"),
        expect: Expect::Value(Answer::Dialect(Dialect::ModuleDeclarations)),
    },
    Case {
        name: CaseName("a_d_cts_file_is_the_declarations_of_commonjs"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("types/index.d.cts"),
        expect: Expect::Value(Answer::Dialect(Dialect::CommonJsDeclarations)),
    },
    Case {
        name: CaseName("a_file_named_d_ts_alone_is_plain_typescript"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("src/d.ts"),
        expect: Expect::Value(Answer::Dialect(Dialect::TypeScript)),
    },
    Case {
        name: CaseName("a_file_of_another_extension_is_plain_typescript"),
        word: Word::Succeeds,
        input: || Scenario::Dialect("component.vue"),
        expect: Expect::Value(Answer::Dialect(Dialect::TypeScript)),
    },
];

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::Reads(path) => {
            Answer::Reads(reads(languages::SourcePath(path)) == languages::Recognition::Reads)
        }
        Scenario::Dialect(path) => Answer::Dialect(dialect(languages::SourcePath(path))),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
