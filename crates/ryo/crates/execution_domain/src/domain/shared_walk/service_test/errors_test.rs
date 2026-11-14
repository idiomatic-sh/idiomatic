//! The shared walk keeps errors in their authored order: predicates short-circuit as written, a memoised predicate builds its error
//! once, and the first source row owns the first error.
use std::borrow::Cow;

use super::{evaluate_plans, plan, plans};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::types::OperatorContext;
use crate::domain::shared_walk::types::WalkWork;
use crate::domain::shared_walk::types::native_batch::value::NativeBatch;
use crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk;
use crate::domain::step_classes::types::ClassPlace;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use ryo_operators::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset, Relation};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::plans::types::{Operation, Plan};

/// A plan with an invalid regular expression before or after its name test.
fn with_invalid(name: &str, first: bool) -> Plan {
    let mut each = plan(name);
    let Operation::Select { comparisons, .. } = &mut each.steps[0].operation else {
        panic!("the fixture builds a Select")
    };
    let at = if first { 0 } else { comparisons.len() };
    comparisons.insert(
        at,
        PlanComparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(2)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText("[".into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        },
    );
    each
}

/// The batch of the walk of the tree of `[(a` (a file node over the tokens `first`, `second`, `second`) by the four fixture rules,
/// rules 0 and 2 selecting rows whose kind is not `file` and that the text `x` matches as a regex.
fn first_source_row() -> NativeBatch {
    let source = b"[(a";
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(source));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(3), Offset(0), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    token(&mut builder, KindBits(2), Offset(2), Offset(3));
    leave_node(&mut builder, Offset(3));
    for (kind, name) in [(1, "file"), (2, "second"), (3, "first")] {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(source), FileId(0)),
            KindBits(kind),
            NameText(name),
        );
    }
    index_kinds(&mut elements);
    let mut plans = plans();
    for rule in [0, 2] {
        let Operation::Select { comparisons, .. } = &mut plans[rule].steps[0].operation else {
            panic!("the fixture builds a Select");
        };
        *comparisons = vec![
            PlanComparison {
                test: Test::NotEqual,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("file".into()))),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            },
            PlanComparison {
                test: Test::Matches,
                left: Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
                right: Operand::Column(ColumnIndex(2)),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            },
        ];
    }
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: ryo_operators::domain::patterns::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let bound = bindings
        .iter()
        .cloned()
        .map(Vec::into_boxed_slice)
        .collect::<Vec<_>>();
    super::evaluate_rows(
        &prepared,
        Relation::SyntaxTree,
        &crate::domain::table_model::types::Rows::Tree(TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Elements,
        }),
        &bound,
        &mut context,
        &[ClassPlace(0), ClassPlace(1), ClassPlace(2)],
        &mut WalkWork::default(),
    )
}
/// What a row walks.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The small tree, by rules that each select the rows named so, an invalid regex test before (true) or after its name test,
    /// every class ready.
    Invalid(&'static [(&'static str, bool)]),
    /// The tree of `[(a`, the first three classes ready.
    FirstSourceRow,
}

/// How a class's selection ended.
#[derive(Clone, Debug, PartialEq)]
enum Completion {
    Complete,
    /// With an invalid regex, for this reason.
    InvalidRegex(Cow<'static, str>),
    Other(Cow<'static, str>),
}

/// What a class answered: how it ended, the positions it selected and the rows it visited.
#[derive(Clone, Debug, PartialEq)]
struct Class {
    completion: Completion,
    positions: Cow<'static, [u32]>,
    visits: u64,
}

/// Each class's answer (none when it was not ready), and how many predicates were tested and reused.
#[derive(Debug, PartialEq)]
struct Walked {
    classes: Cow<'static, [Option<Class>]>,
    tests: u64,
    reuses: u64,
}

type Input = Ask;
type Value = Walked;
type Failure = std::convert::Infallible;

fn walked(batch: &NativeBatch, classes: usize) -> Walked {
    Walked {
        classes: (0..classes)
            .map(|at| {
                batch.class(ClassPlace(at)).map(|result| Class {
                    completion: match result.completion.as_ref().err() {
                        None => Completion::Complete,
                        Some(OperatingError::InvalidRegex { reason }) => {
                            Completion::InvalidRegex(Cow::Owned(reason.to_string()))
                        }
                        Some(other) => Completion::Other(Cow::Owned(format!("{other:?}"))),
                    },
                    positions: Cow::Owned(result.positions.0.clone()),
                    visits: batch.visits(ClassPlace(at)).0,
                })
            })
            .collect(),
        tests: batch.counts().predicate_tests.0,
        reuses: batch.counts().predicate_reuses.0,
    }
}

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::Invalid(rules) => {
            let plans: Vec<Plan> = rules
                .iter()
                .map(|(name, first)| with_invalid(name, *first))
                .collect();
            let bindings = vec![Vec::new(); plans.len()];
            let prepared = PreparedWalk::compile(&plans, &bindings);
            let ready: Vec<ClassPlace> = (0..prepared.classes().len()).map(ClassPlace).collect();
            walked(&evaluate_plans(&prepared, &bindings, &ready), ready.len())
        }
        Ask::FirstSourceRow => walked(&first_source_row(), 3),
    })
}

/// A class that ended at the invalid regex `[`.
const INVALID: Completion = Completion::InvalidRegex(Cow::Borrowed(
    "regex parse error:\n    [\n    ^\nerror: unclosed character class",
));

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("predicates preserve authored error and short-circuit order"),
        word: Word::Succeeds,
        input: || Ask::Invalid(&[("absent-kind", true), ("absent-kind", false)]),
        expect: Expect::Value(Walked {
            classes: Cow::Borrowed(&[
                Some(Class {
                    completion: INVALID,
                    positions: Cow::Borrowed(&[]),
                    visits: 1,
                }),
                Some(Class {
                    completion: Completion::Complete,
                    positions: Cow::Borrowed(&[]),
                    visits: 0,
                }),
            ]),
            tests: 1,
            reuses: 0,
        }),
    },
    Case {
        name: CaseName("memoised predicates reuse hits and build an error once"),
        word: Word::Succeeds,
        input: || Ask::Invalid(&[("word", true), ("other", true)]),
        expect: Expect::Value(Walked {
            classes: Cow::Borrowed(&[
                Some(Class {
                    completion: INVALID,
                    positions: Cow::Borrowed(&[]),
                    visits: 1,
                }),
                Some(Class {
                    completion: INVALID,
                    positions: Cow::Borrowed(&[]),
                    visits: 1,
                }),
            ]),
            tests: 1,
            reuses: 1,
        }),
    },
    Case {
        name: CaseName("first source row owns first error"),
        word: Word::Succeeds,
        input: || Ask::FirstSourceRow,
        expect: Expect::Value(Walked {
            classes: Cow::Borrowed(&[
                Some(Class {
                    completion: INVALID,
                    positions: Cow::Borrowed(&[]),
                    visits: 1,
                }),
                Some(Class {
                    completion: Completion::Complete,
                    positions: Cow::Borrowed(&[]),
                    visits: 0,
                }),
                Some(Class {
                    completion: Completion::Complete,
                    positions: Cow::Borrowed(&[0]),
                    visits: 4,
                }),
            ]),
            tests: 6,
            reuses: 0,
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
