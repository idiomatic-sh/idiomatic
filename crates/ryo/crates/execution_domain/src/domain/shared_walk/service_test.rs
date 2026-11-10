//! The prepared native pass keeps independent class results and never publishes.
use crate::domain::operator::types::OperatorContext;
use crate::domain::shared_walk::service::evaluate;
use crate::domain::shared_walk::types::WalkWork;
use crate::domain::shared_walk::types::native_batch::value::NativeBatch;
use crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk;
use crate::domain::step_classes::types::ClassPlace;
use crate::domain::table_cells::fixtures::{SMALL_SOURCE, SmallTree, small_tree};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, LiteralText, Operand, ParameterId, PlanComparison,
    QueryLiteral, StepId, StepScope, Test,
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
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let tree = TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    let bound = bindings
        .iter()
        .cloned()
        .map(Vec::into_boxed_slice)
        .collect::<Vec<_>>();
    let mut operator_scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: crate::domain::matching::service_test::matcher(),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut operator_scratch,
    };
    evaluate(
        prepared,
        Relation::SyntaxTree,
        &crate::domain::table_model::types::Rows::Tree(tree),
        &bound,
        &mut context,
        ready,
        &mut WalkWork::default(),
    )
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

fn any_equivalent_consumer_can_name_the_ready_class() {
    let mut plans = plans();
    plans[1] = plan("word");
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let representative = prepared
        .class_of(crate::domain::coverage::types::RuleIndex(0))
        .unwrap()[0]
        .unwrap();
    let other = prepared
        .class_of(crate::domain::coverage::types::RuleIndex(1))
        .unwrap()[0]
        .unwrap();
    assert_eq!(representative, other);
    assert_eq!(prepared.classes()[representative.0].members.len(), 3);
    let from_other = evaluate_plans(&prepared, &bindings, &[other]);
    assert_eq!(
        from_other.class(representative).unwrap().positions.0,
        [1, 3]
    );
    let from_first = evaluate_plans(&prepared, &bindings, &[representative]);
    assert_eq!(
        from_first.class(representative).unwrap().positions.0,
        [1, 3]
    );
    assert!((1..prepared.classes().len()).all(|at| from_first.class(ClassPlace(at)).is_none()));
}

fn distinct_single_consumer_predicates_share_the_walk() {
    let plans = [plan("word"), plan("other")];
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    assert_eq!(prepared.classes().len(), 2);
    assert!(
        prepared
            .classes()
            .iter()
            .all(|class| class.members.len() == 1)
    );
    let batch = evaluate_plans(&prepared, &bindings, &[ClassPlace(0), ClassPlace(1)]);
    assert_eq!(batch.class(ClassPlace(0)).unwrap().positions.0, [1, 3]);
    assert_eq!(batch.class(ClassPlace(1)).unwrap().positions.0, [2]);
    assert_eq!(batch.visits(ClassPlace(0)).0, 2);
    assert_eq!(batch.visits(ClassPlace(1)).0, 1);
}

#[derive(Clone, Copy)]
enum Scenario {
    Distinct,
    Ready,
}

type Input = Scenario;
type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("distinct single-consumer predicates share source traversal"),
        word: Word::Succeeds,
        input: || Scenario::Distinct,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("any equivalent consumer names the same ready class"),
        word: Word::Succeeds,
        input: || Scenario::Ready,
        expect: Expect::Value(1),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::Distinct => distinct_single_consumer_predicates_share_the_walk(),
        Scenario::Ready => any_equivalent_consumer_can_name_the_ready_class(),
    }
    Ok(1)
}

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
        matcher: crate::domain::matching::service_test::matcher(),
        bindings: &[],
        symbols: symbols.into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    })
}

#[cfg(test)]
mod errors_test;
