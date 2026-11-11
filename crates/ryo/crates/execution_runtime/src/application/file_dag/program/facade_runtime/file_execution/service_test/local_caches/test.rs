//! Finite literal patterns are prepared once for the program and never enter an invocation's local caches; dynamic
//! patterns (bound, computed or read from a row) do, once.
use super::fixtures::{FROM, comparing, local_caches, select};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, Function, LiteralText, Operand, ParameterId, QueryLiteral, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The pattern an invocation's one operation compares its text row with.
#[derive(Clone, Copy, Debug)]
enum Pattern {
    /// A literal pattern under this test, over this text.
    Literal {
        test: Test,
        pattern: &'static str,
        text: &'static str,
    },
    /// A literal replacement pattern over this text.
    Replacement {
        pattern: &'static str,
        text: &'static str,
    },
    /// A regex bound to this text, over `alpha`, beside a literal `a.*` of the same bytes.
    Bound(&'static str),
    /// A regex computed as `"a" + ".*"`, over `alpha`, beside a literal `a.*`.
    Computed,
    /// A regex read from the row `a.*`, beside a literal `a.*`.
    ReadFromRow,
    /// Globs bound to these texts, over `alpha`, beside a literal `a*`.
    BoundGlobs(&'static [&'static str]),
}

/// The rows the invocation answered and the regexes and globs its local caches compiled.
#[derive(Debug, PartialEq)]
struct Caches {
    rows: usize,
    regexes: usize,
    globs: usize,
}

type Input = Pattern;
type Value = Caches;
type Failure = OperatingError;

const fn caches(rows: usize, regexes: usize, globs: usize) -> Caches {
    Caches {
        rows,
        regexes,
        globs,
    }
}

const fn literal(test: Test, pattern: &'static str, text: &'static str) -> Pattern {
    Pattern::Literal {
        test,
        pattern,
        text,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("only_dynamic_patterns_enter_local_caches"),
        word: Word::Succeeds,
        input: || literal(Test::Matches, r"\s+", "é\u{2003}雪"),
        expect: Expect::Value(caches(1, 0, 0)),
    },
    Case {
        name: CaseName("a_literal_glob_never_enters_local_caches"),
        word: Word::Succeeds,
        input: || literal(Test::GlobAny, "*.rs", "src/lib.rs"),
        expect: Expect::Value(caches(1, 0, 0)),
    },
    Case {
        name: CaseName("a_literal_wildcard_glob_never_enters_local_caches"),
        word: Word::Succeeds,
        input: || literal(Test::GlobAny, "*", "αβ"),
        expect: Expect::Value(caches(1, 0, 0)),
    },
    Case {
        name: CaseName("a_literal_negated_glob_never_enters_local_caches"),
        word: Word::Succeeds,
        input: || literal(Test::NotGlobAny, "*", "αβ"),
        expect: Expect::Value(caches(0, 0, 0)),
    },
    Case {
        name: CaseName("a_literal_single_character_glob_never_enters_local_caches"),
        word: Word::Succeeds,
        input: || literal(Test::GlobAny, "α?", "αβ"),
        expect: Expect::Value(caches(1, 0, 0)),
    },
    Case {
        name: CaseName("a_literal_replacement_pattern_never_enters_local_caches"),
        word: Word::Succeeds,
        input: || Pattern::Replacement {
            pattern: r"\s+",
            text: "a \t b",
        },
        expect: Expect::Value(caches(1, 0, 0)),
    },
    Case {
        name: CaseName("a_bound_regex_cannot_reuse_the_same_byte_literal_slot"),
        word: Word::Succeeds,
        input: || Pattern::Bound("a.*"),
        expect: Expect::Value(caches(1, 1, 0)),
    },
    Case {
        name: CaseName("a_bound_regex_that_matches_nothing_enters_local_caches_once"),
        word: Word::Succeeds,
        input: || Pattern::Bound("b.*"),
        expect: Expect::Value(caches(0, 1, 0)),
    },
    Case {
        name: CaseName("a_computed_regex_enters_local_caches_once"),
        word: Word::Succeeds,
        input: || Pattern::Computed,
        expect: Expect::Value(caches(1, 1, 0)),
    },
    Case {
        name: CaseName("a_regex_read_from_the_row_enters_local_caches_once"),
        word: Word::Succeeds,
        input: || Pattern::ReadFromRow,
        expect: Expect::Value(caches(1, 1, 0)),
    },
    Case {
        name: CaseName("a_bound_glob_list_enters_local_caches_once"),
        word: Word::Succeeds,
        input: || Pattern::BoundGlobs(&["a*", "α*"]),
        expect: Expect::Value(caches(1, 0, 1)),
    },
    Case {
        name: CaseName("a_bound_glob_list_that_matches_nothing_enters_local_caches_once"),
        word: Word::Succeeds,
        input: || Pattern::BoundGlobs(&["b*"]),
        expect: Expect::Value(caches(0, 0, 1)),
    },
    Case {
        name: CaseName("an_empty_bound_glob_list_enters_local_caches_once"),
        word: Word::Succeeds,
        input: || Pattern::BoundGlobs(&[]),
        expect: Expect::Value(caches(0, 0, 1)),
    },
];

/// Runs the pattern's operation over its one text row, beside a literal shadow step, and reads
/// what the invocation's local caches compiled.
fn subject(input: Input) -> Result<Value, Failure> {
    let text = |text: &str| LiteralText(text.into());
    let regex = select(Test::Matches, "a.*");
    let glob = select(Test::GlobAny, "a*");
    let parameter = Operand::Parameter(ParameterId(0));
    let (operation, row, bindings, shadow) = match input {
        Pattern::Literal {
            test,
            pattern,
            text: row,
        } => {
            let operation = select(test, pattern);
            (operation.clone(), row, vec![], operation)
        }
        Pattern::Replacement { pattern, text: row } => {
            let operation = Operation::Replace {
                from: FROM,
                text: ColumnIndex(0),
                pattern: text(pattern),
                replacement: text("$1"),
            };
            (operation.clone(), row, vec![], operation)
        }
        Pattern::Bound(pattern) => (
            comparing(regex.clone(), parameter),
            "alpha",
            vec![ParameterBinding::Text(text(pattern))],
            regex,
        ),
        Pattern::Computed => (
            comparing(
                regex.clone(),
                Operand::Function(Applied {
                    function: Function::Concat,
                    arguments: vec![
                        Operand::Literal(QueryLiteral::Text(text("a"))),
                        Operand::Literal(QueryLiteral::Text(text(".*"))),
                    ],
                }),
            ),
            "alpha",
            vec![],
            regex,
        ),
        Pattern::ReadFromRow => (
            comparing(regex.clone(), Operand::Column(ColumnIndex(0))),
            "a.*",
            vec![],
            regex,
        ),
        Pattern::BoundGlobs(patterns) => (
            comparing(glob.clone(), parameter),
            "alpha",
            vec![ParameterBinding::Texts(
                patterns.iter().map(|pattern| text(pattern)).collect(),
            )],
            glob,
        ),
    };
    let (rows, regexes, globs) = local_caches(operation, row, &bindings, &shadow)?;
    Ok(caches(rows, regexes, globs))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
