//! The prepared native pass keeps independent class results and never publishes.
use std::borrow::Cow;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::types::OperatorContext;
use crate::domain::shared_walk::service::evaluate;
use crate::domain::shared_walk::types::WalkWork;
use crate::domain::shared_walk::types::native_batch::value::NativeBatch;
use crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk;
use crate::domain::step_classes::types::ClassPlace;
use crate::domain::table_cells::fixtures::{SMALL_SOURCE, SmallTree, small_tree};
use crate::domain::table_model::types::Rows;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::table_scratch::types::OperatorScratch;
use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_relations::domain::relations::types::{FileId, Integer, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, LiteralText, Operand, ParameterId, PlanComparison,
    QueryLiteral, StepId, StepScope, Test, WholeNumber,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_bindings::types::ParameterBinding;

pub(crate) fn plan(name: &str) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(if name.is_empty() { 2 } else { 1 })),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(name.into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

pub(crate) fn plans() -> [Plan; 4] {
    [plan("word"), plan("other"), plan("word"), plan("")]
}

pub(crate) fn evaluate_plans(
    prepared: &PreparedWalk,
    bindings: &[Vec<ParameterBinding>],
    ready: &[ClassPlace],
) -> NativeBatch {
    with_small_tree(|elements, context| {
        evaluate(
            prepared,
            Relation::SyntaxTree,
            &tree_of(elements, FileId(0)),
            &bound(bindings),
            context,
            ready,
            &mut WalkWork::default(),
        )
    })
}

/// The bindings of each rule, as the walk takes them.
pub(crate) fn bound(bindings: &[Vec<ParameterBinding>]) -> Vec<Box<[ParameterBinding]>> {
    bindings
        .iter()
        .cloned()
        .map(Vec::into_boxed_slice)
        .collect()
}

/// The rows of a tree of these elements, as the file of this number.
pub(crate) fn tree_of(elements: &Elements, file: FileId) -> Rows<'_> {
    Rows::Tree(TreeRows {
        elements,
        file,
        form: TreeForm::Elements,
    })
}

/// Runs with the elements of the small tree and an operator context over its source and names.
pub(crate) fn with_small_tree<T>(run: impl FnOnce(&Elements, &mut OperatorContext<'_>) -> T) -> T {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut operator_scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: ryo_operators::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut operator_scratch,
    };
    run(&elements, &mut context)
}

/// The raw evaluator, reached by the other walk proofs only through this unit proof.
pub(crate) fn evaluate_rows(
    prepared: &PreparedWalk,
    relation: Relation,
    rows: &Rows<'_>,
    bound: &[Box<[ParameterBinding]>],
    context: &mut OperatorContext<'_>,
    classes: &[ClassPlace],
    work: &mut WalkWork,
) -> NativeBatch {
    evaluate(prepared, relation, rows, bound, context, classes, work)
}

/// A walk of the small tree for rules that each select the `syntax_tree` rows named so (the empty name: whose kind is empty),
/// the ready classes being those of the rules at these places.
#[derive(Clone, Copy, Debug)]
struct Walk {
    names: &'static [&'static str],
    ready: &'static [usize],
}

/// The class place of each rule's first step, the members of each class, and per class the positions it selected (none when it
/// was not ready) and the rows it visited.
#[derive(Debug, PartialEq)]
struct Walked {
    classes: Cow<'static, [usize]>,
    members: Cow<'static, [usize]>,
    positions: Cow<'static, [Option<Cow<'static, [u32]>>]>,
    visits: Cow<'static, [u64]>,
}

type Input = Walk;
type Value = Walked;
type Failure = OperatingError;

fn subject(walk: Input) -> Result<Value, Failure> {
    let plans: Vec<Plan> = walk.names.iter().map(|name| plan(name)).collect();
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let class_of = |rule: usize| -> Result<ClassPlace, Failure> {
        prepared
            .class_of(crate::domain::coverage::types::RuleIndex(rule))
            .and_then(|classes| classes.first().copied().flatten())
            .ok_or(OperatingError::NotRun)
    };
    let classes = (0..plans.len())
        .map(|rule| class_of(rule).map(|class| class.0))
        .collect::<Result<Vec<_>, _>>()?;
    let ready = walk
        .ready
        .iter()
        .map(|rule| class_of(*rule))
        .collect::<Result<Vec<_>, _>>()?;
    let batch = evaluate_plans(&prepared, &bindings, &ready);
    let places = 0..prepared.classes().len();
    Ok(Walked {
        classes: classes.into(),
        members: prepared
            .classes()
            .iter()
            .map(|class| class.members.len())
            .collect(),
        positions: places
            .clone()
            .map(|at| {
                batch
                    .class(ClassPlace(at))
                    .map(|result| Cow::Owned(result.positions.0.clone()))
            })
            .collect(),
        visits: places.map(|at| batch.visits(ClassPlace(at)).0).collect(),
    })
}

/// Three rules naming `word` and one the empty kind, by places 0 to 3.
const THREE_WORDS: &[&str] = &["word", "word", "word", ""];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("distinct single-consumer predicates share source traversal"),
        word: Word::Succeeds,
        input: || Walk {
            names: &["word", "other"],
            ready: &[0, 1],
        },
        expect: Expect::Value(Walked {
            classes: Cow::Borrowed(&[0, 1]),
            members: Cow::Borrowed(&[1, 1]),
            positions: Cow::Borrowed(&[Some(Cow::Borrowed(&[1, 3])), Some(Cow::Borrowed(&[2]))]),
            visits: Cow::Borrowed(&[2, 1]),
        }),
    },
    Case {
        name: CaseName("any equivalent consumer names the same ready class"),
        word: Word::Succeeds,
        input: || Walk {
            names: THREE_WORDS,
            ready: &[1],
        },
        expect: Expect::Value(Walked {
            classes: Cow::Borrowed(&[0, 0, 0, 1]),
            members: Cow::Borrowed(&[3, 1]),
            positions: Cow::Borrowed(&[Some(Cow::Borrowed(&[1, 3])), None]),
            visits: Cow::Borrowed(&[2, 0]),
        }),
    },
    Case {
        name: CaseName("the first consumer names the same ready class and readies no other"),
        word: Word::Succeeds,
        input: || Walk {
            names: THREE_WORDS,
            ready: &[0],
        },
        expect: Expect::Value(Walked {
            classes: Cow::Borrowed(&[0, 0, 0, 1]),
            members: Cow::Borrowed(&[3, 1]),
            positions: Cow::Borrowed(&[Some(Cow::Borrowed(&[1, 3])), None]),
            visits: Cow::Borrowed(&[2, 0]),
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

pub(crate) fn add_pattern(plans: &mut [Plan; 4], parameter: u32) {
    for rule in [0, 2] {
        let Operation::Select { comparisons, .. } = &mut plans[rule].steps[0].operation else {
            panic!("the fixture builds a Select");
        };
        comparisons.push(PlanComparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(2)),
            right: Operand::Parameter(ParameterId(parameter)),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        });
    }
}

pub(crate) fn relation_plan(relation: Relation, comparisons: Vec<PlanComparison>) -> Plan {
    let mut selected = plan("word");
    selected.steps[0].operation = Operation::Select {
        from: PlanInput::Relation(relation),
        comparisons,
        kept: Vec::new(),
    };
    selected
}

pub(crate) fn with_context<T>(
    symbols: &mut Symbols,
    run: impl FnOnce(&mut OperatorContext<'_>) -> T,
) -> T {
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    run(&mut OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: ryo_operators::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        bindings: &[],
        symbols: symbols.into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    })
}

/// An equality of a column with a number.
pub(crate) fn numbered(column: u32, value: i64) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Number(Integer(value))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

/// Three `source_location` rules testing column 8 against a parameter bound to 1, 2 and the text `1`, and a `semantic_syntax`
/// rule testing its first column against 0, with their bindings.
pub(crate) fn generic_plans() -> ([Plan; 4], Vec<Vec<ParameterBinding>>) {
    let mut by_parameter = numbered(8, 0);
    by_parameter.right = Operand::Parameter(ParameterId(0));
    let plans = [
        relation_plan(Relation::SourceLocation, vec![by_parameter.clone()]),
        relation_plan(Relation::SourceLocation, vec![by_parameter.clone()]),
        relation_plan(Relation::SourceLocation, vec![by_parameter]),
        relation_plan(Relation::SemanticSyntax, vec![numbered(0, 0)]),
    ];
    let bindings = vec![
        vec![ParameterBinding::Number(WholeNumber(1))],
        vec![ParameterBinding::Number(WholeNumber(2))],
        vec![ParameterBinding::Text(LiteralText("1".into()))],
        vec![],
    ];
    (plans, bindings)
}

/// The plan selecting `word` whose second step matches the kinds of the tree against the invalid regex `[`.
pub(crate) fn invalid_second_step() -> Plan {
    let mut selected = plan("word");
    selected.steps.push(PlanStep {
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::SyntaxTree),
            comparisons: vec![PlanComparison {
                test: Test::Matches,
                left: Operand::Column(ColumnIndex(2)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("[".into()))),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            }],
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    });
    selected
}

#[cfg(test)]
mod errors_test;
