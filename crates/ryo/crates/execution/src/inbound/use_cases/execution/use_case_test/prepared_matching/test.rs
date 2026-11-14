//! Literal metadata and dynamic patterns are exercised through real prepared file owners; the runtime proves which of them
//! enter the invocation's local caches.
use super::super::super::fixtures;
use super::super::super::fixtures::requests::run_prepared;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_operators::domain::table_texts::types::TextView;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, Function, Input as Source, LiteralText, Operand, ParameterId,
    PlanComparison, QueryLiteral, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::{service::push_row, types::Table};
use std::borrow::Cow;
use std::sync::atomic::Ordering;
/// How the probed select compares its one text column.
#[derive(Clone, Copy, Debug)]
enum Pattern {
    /// With `test` against a literal pattern.
    Literal(Test, &'static str),
    /// A replace of the literal pattern by the literal replacement.
    Replace(&'static str, &'static str),
    /// Equal to `never`, then matching `[`.
    ShortCircuit,
    /// Matching a bound text.
    Bound(&'static str),
    /// Glob-any against a bound list of texts.
    BoundList(&'static [&'static str]),
    /// Matching the literal concatenation of two texts.
    Concat(&'static str, &'static str),
    /// Matching the row's own text.
    Row,
}

/// A probe: the pattern, the subject row's text (or no row), the literal that shares the request
/// undemanded and whose matcher is asked for (or, unasked, the short circuit's `[` or the probed
/// operation itself), and the files it runs over.
#[derive(Clone, Copy, Debug)]
struct Probe {
    pattern: Pattern,
    text: Option<&'static str>,
    shadow: Option<(Test, &'static str)>,
    files: usize,
}

/// What one file answered: the rows' cells as text, or the invalid regex's reason.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Answer {
    Rows(Cow<'static, [Cow<'static, str>]>),
    /// The regex compiler's reason.
    InvalidRegex(Cow<'static, str>),
}

/// The matchers built before any file, whether the shadow's literal matcher is ready, each file's
/// answer, and the matchers built after.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Probed {
    builds: (usize, usize),
    shadow_ready: Option<bool>,
    answers: Cow<'static, [Answer]>,
}

/// The probes run in turn, each over its own prepared program.
type Input = &'static [Probe];
/// What each probe showed.
type Value = Cow<'static, [Probed]>;
type Failure = OperatingError;

/// The literal pattern over `text`, run over `files` files.
const fn probe(pattern: Pattern, text: Option<&'static str>, files: usize) -> Probe {
    Probe {
        pattern,
        text,
        shadow: None,
        files,
    }
}

/// The dynamic pattern over `text` beside the undemanded literal `shadow`, run over one file.
const fn dynamic(pattern: Pattern, text: &'static str, shadow: (Test, &'static str)) -> Probe {
    Probe {
        pattern,
        text: Some(text),
        shadow: Some(shadow),
        files: 1,
    }
}

/// No matcher before and `after` after, whether the shadow is ready, and the answers.
const fn probed(after: usize, shadow_ready: Option<bool>, answers: &'static [Answer]) -> Probed {
    Probed {
        builds: (0, after),
        shadow_ready,
        answers: Cow::Borrowed(answers),
    }
}

/// The literal regex and glob the shadows compile.
const A_ANY: (Test, &str) = (Test::Matches, "a.*");
/// The literal glob the list shadows compile.
const A_GLOB: (Test, &str) = (Test::GlobAny, "a*");
/// One row of the spaced text.
const SPACED: Answer = Answer::Rows(Cow::Borrowed(&[Cow::Borrowed("é\u{2003}雪")]));
/// One row of `src/lib.rs`.
const LIB: Answer = Answer::Rows(Cow::Borrowed(&[Cow::Borrowed("src/lib.rs")]));
/// The row kept, then the literal replacement.
const REPLACED: Answer = Answer::Rows(Cow::Borrowed(&[
    Cow::Borrowed("a \t b"),
    Cow::Borrowed("a$1b"),
]));
/// The refusal of the unclosed class `[`.
const UNCLOSED: Answer = Answer::InvalidRegex(Cow::Borrowed(
    "regex parse error:\n    [\n    ^\nerror: unclosed character class",
));
/// One row of `alpha`.
const ALPHA: Answer = Answer::Rows(Cow::Borrowed(&[Cow::Borrowed("alpha")]));
/// One row of the text `a.*`.
const ROW_TEXT: Answer = Answer::Rows(Cow::Borrowed(&[Cow::Borrowed("a.*")]));
/// The refusal of a repetition with nothing to repeat.
const NO_REPEATED: Answer = Answer::InvalidRegex(Cow::Borrowed(
    "regex parse error:\n    *\n    ^\nerror: repetition operator missing expression",
));
/// No row.
const NONE: Answer = Answer::Rows(Cow::Borrowed(&[]));
/// One row of `αβ`.
const ALPHA_BETA: Answer = Answer::Rows(Cow::Borrowed(&[Cow::Borrowed("αβ")]));

/// A literal text operand.
fn text(text: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(text.into())))
}
/// A select of the function relation's rows whose column 0 passes each `(test, right)` comparison.
fn comparing(comparisons: Vec<(Test, Operand)>) -> Operation {
    Operation::Select {
        from: Source::Relation(Relation::Function),
        kept: vec![],
        comparisons: comparisons
            .into_iter()
            .map(|(test, right)| PlanComparison {
                left: Operand::Column(ColumnIndex(0)),
                right,
                test,
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            })
            .collect(),
    }
}
/// A select of the rows whose column 0 passes `test` against the literal `pattern`.
fn select(test: Test, pattern: &str) -> Operation {
    comparing(vec![(test, text(pattern))])
}
fn prepare(
    operation: Operation,
    subject: Option<&str>,
    bindings: &[ParameterBinding],
    shadow: &Operation,
) -> Result<
    (
        fixtures::requests::Preparation,
        PreparedExecutionProgram,
        Symbols,
    ),
    Failure,
> {
    let mut symbols = Symbols::default();
    let mut input = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    if let Some(text) = subject {
        let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText(text));
        push_row(&mut input, &[Cell::Symbol(symbol)]);
    }
    let mut authored = inputs::table_inputs(operation, &[&input], bindings, &mut symbols);
    // A real but undemanded authored literal remains in the same immutable request.
    // Dynamic callers must not acquire its already-compiled slot merely by equal bytes.
    let mut shadow = shadow.clone();
    ryo_values::domain::plans::service::each_input(&mut shadow, &mut |input| {
        *input = Source::Step(StepId(0))
    });
    authored.plan.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: shadow,
    });
    let (engine, program) =
        fixtures::requests::prepared(authored.plan, bindings.to_vec(), authored.hosted)?;
    Ok((engine, program, symbols))
}
/// The matchers the program has built.
fn builds(program: &PreparedExecutionProgram) -> usize {
    program.matchers().builds.load(Ordering::Relaxed)
}
/// A literal regex, glob and replace (which appends one literal replacement cell and keeps its
/// input), each compiled once across three files.
const LITERALS: &[Probe] = &[
    probe(
        Pattern::Literal(Test::Matches, r"\s+"),
        Some("é\u{2003}雪"),
        3,
    ),
    probe(
        Pattern::Literal(Test::GlobAny, "*.rs"),
        Some("src/lib.rs"),
        3,
    ),
    probe(Pattern::Replace(r"\s+", "$1"), Some("a \t b"), 3),
];
/// What each probe shows.
const LITERAL_LIFETIMES: &[Probed] = &[
    probed(1, None, &[SPACED, SPACED, SPACED]),
    probed(1, None, &[LIB, LIB, LIB]),
    probed(1, None, &[REPLACED, REPLACED, REPLACED]),
];
/// An invalid regex and replace over no row and over a row, and an invalid regex behind a false
/// earlier predicate: nothing is built until a row reaches it.
const INVALIDS: &[Probe] = &[
    probe(Pattern::Literal(Test::Matches, "["), None, 1),
    probe(Pattern::Literal(Test::Matches, "["), Some("actual"), 2),
    probe(Pattern::Replace("[", "x"), None, 1),
    probe(Pattern::Replace("[", "x"), Some("actual"), 2),
    probe(Pattern::ShortCircuit, Some("actual"), 1),
];
/// What each probe shows.
const INVALID_LITERALS: &[Probed] = &[
    probed(0, None, &[NONE]),
    probed(1, None, &[UNCLOSED, UNCLOSED]),
    probed(0, None, &[NONE]),
    probed(1, None, &[UNCLOSED, UNCLOSED]),
    probed(0, None, &[NONE]),
];
/// Bound, concatenated, row and bound-list patterns beside an undemanded literal of the same bytes:
/// the literal's matcher is ready and the dynamic pattern never reuses it.
const DYNAMICS: &[Probe] = &[
    dynamic(Pattern::Bound("a.*"), "alpha", A_ANY),
    dynamic(Pattern::Bound("b.*"), "alpha", A_ANY),
    dynamic(Pattern::Concat("a", ".*"), "alpha", A_ANY),
    dynamic(Pattern::Row, "a.*", A_ANY),
    dynamic(Pattern::BoundList(&["a*", "α*"]), "alpha", A_GLOB),
    dynamic(Pattern::BoundList(&["b*"]), "alpha", A_GLOB),
    dynamic(Pattern::BoundList(&[]), "alpha", A_GLOB),
];
/// What each probe shows.
const DYNAMIC_PATTERNS: &[Probed] = &[
    probed(1, Some(true), &[ALPHA]),
    probed(1, Some(true), &[NONE]),
    probed(1, Some(true), &[ALPHA]),
    probed(1, Some(true), &[ROW_TEXT]),
    probed(1, Some(true), &[ALPHA]),
    probed(1, Some(true), &[NONE]),
    probed(1, Some(true), &[NONE]),
];
/// Glob, negated glob and one-character glob over `αβ` across two files, and a glob written as a regex.
const GLOBS: &[Probe] = &[
    probe(Pattern::Literal(Test::GlobAny, "*"), Some("αβ"), 2),
    probe(Pattern::Literal(Test::NotGlobAny, "*"), Some("αβ"), 2),
    probe(Pattern::Literal(Test::GlobAny, "α?"), Some("αβ"), 2),
    probe(Pattern::Literal(Test::Matches, "*"), Some("αβ"), 1),
];
/// What each probe shows.
const GLOB_FAMILIES: &[Probed] = &[
    probed(1, None, &[ALPHA_BETA, ALPHA_BETA]),
    probed(1, None, &[NONE, NONE]),
    probed(1, None, &[ALPHA_BETA, ALPHA_BETA]),
    probed(1, None, &[NO_REPEATED]),
];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("literal_matchers_survive_real_rule_and_file_workspaces"),
        word: Word::Succeeds,
        input: || LITERALS,
        expect: Expect::Value(Cow::Borrowed(LITERAL_LIFETIMES)),
    },
    Case {
        name: CaseName("invalid_literals_keep_errors_empty_and_short_circuit_laziness"),
        word: Word::Succeeds,
        input: || INVALIDS,
        expect: Expect::Value(Cow::Borrowed(INVALID_LITERALS)),
    },
    Case {
        name: CaseName("bound_lists_functions_and_row_patterns_remain_invocation_owned"),
        word: Word::Succeeds,
        input: || DYNAMICS,
        expect: Expect::Value(Cow::Borrowed(DYNAMIC_PATTERNS)),
    },
    Case {
        name: CaseName("literal_glob_fallback_family_and_polarity_match_real_output"),
        word: Word::Succeeds,
        input: || GLOBS,
        expect: Expect::Value(Cow::Borrowed(GLOB_FAMILIES)),
    },
];

/// The probed operation.
fn operation(pattern: Pattern) -> Operation {
    let bound = || Operand::Parameter(ParameterId(0));
    comparing(match pattern {
        Pattern::Literal(test, pattern) => vec![(test, text(pattern))],
        Pattern::Replace(pattern, replacement) => {
            return Operation::Replace {
                from: Source::Relation(Relation::Function),
                text: ColumnIndex(0),
                pattern: LiteralText(pattern.into()),
                replacement: LiteralText(replacement.into()),
            };
        }
        Pattern::ShortCircuit => vec![(Test::Equal, text("never")), (Test::Matches, text("["))],
        Pattern::Bound(_) => vec![(Test::Matches, bound())],
        Pattern::BoundList(_) => vec![(Test::GlobAny, bound())],
        Pattern::Concat(first, second) => vec![(
            Test::Matches,
            Operand::Function(Applied {
                function: Function::Concat,
                arguments: vec![text(first), text(second)],
            }),
        )],
        Pattern::Row => vec![(Test::Matches, Operand::Column(ColumnIndex(0)))],
    })
}

/// Runs each probe.
fn subject(probes: Input) -> Result<Value, Failure> {
    let probed = probes
        .iter()
        .map(|&probe| run_probe(probe))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Cow::Owned(probed))
}

/// Prepares the probe and runs it over its files.
fn run_probe(probe: Probe) -> Result<Probed, Failure> {
    let probed = operation(probe.pattern);
    let shadow = match (probe.shadow, probe.pattern) {
        (Some((test, text)), _) => select(test, text),
        (None, Pattern::ShortCircuit) => select(Test::Matches, "["),
        (None, _) => probed.clone(),
    };
    let texts = |texts: &[&str]| {
        texts
            .iter()
            .map(|text| LiteralText((*text).into()))
            .collect()
    };
    let bindings: Vec<ParameterBinding> = match probe.pattern {
        Pattern::Bound(text) => vec![ParameterBinding::Text(LiteralText(text.into()))],
        Pattern::BoundList(list) => vec![ParameterBinding::Texts(texts(list))],
        Pattern::Literal(..) | Pattern::Replace(..) => vec![],
        Pattern::ShortCircuit | Pattern::Concat(..) | Pattern::Row => vec![],
    };
    let (engine, program, mut symbols) = prepare(probed, probe.text, &bindings, &shadow)?;
    let before = builds(&program);
    let matcher = ryo_execution_runtime::application::file_dag::fixtures::pattern_matcher();
    let matchers = program.matchers();
    let shadow_ready = probe.shadow.map(|(test, text)| {
        if test == Test::Matches {
            matchers
                .regex(TextView(text), matcher)
                .and_then(Result::ok)
                .is_some()
        } else {
            matchers.glob(test, TextView(text), matcher).is_some()
        }
    });
    let mut read = fixtures::TableFacts::new(&[]);
    let elements = Elements::default();
    let mut answers = Vec::new();
    for _file in 0..probe.files {
        let answer = run_prepared(&engine, &program, b"", &elements, &mut symbols, &mut read);
        answers.push(match answer {
            Ok(table) => Answer::Rows(Cow::Owned(
                table
                    .cells
                    .iter()
                    .map(|cell| Cow::Owned(inputs::text(*cell, &symbols).to_owned()))
                    .collect(),
            )),
            Err(OperatingError::InvalidRegex { reason }) => {
                Answer::InvalidRegex(Cow::Owned(reason))
            }
            Err(error) => return Err(error),
        });
    }
    Ok(Probed {
        builds: (before, builds(&program)),
        shadow_ready,
        answers: Cow::Owned(answers),
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
