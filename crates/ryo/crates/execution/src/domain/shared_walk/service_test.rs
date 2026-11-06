//! The prepared native pass keeps independent class results and never publishes.

use ryo_language::domain::{
    completion::service::{completion, name_kind},
    elements::service::{enter_node, leave_node, table_builder, token},
    elements::types::Elements,
    kind_index::service::index_kinds,
    symbols::types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::types::{FileId, KindBits, Offset, Relation},
    terms::types::NameText,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Comparison, Input as PlanInput, LiteralText, Operand, ParameterId,
        QueryLiteral, StepId, Test,
    },
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
};
use {
    crate::domain::operating::types::OperatorContext,
    crate::domain::positions::service_test::SMALL_SOURCE,
    crate::domain::positions::service_test::SmallTree,
    crate::domain::positions::service_test::small_tree,
    crate::domain::positions::types::ClassPlace, crate::domain::shared_walk::service::evaluate,
    crate::domain::shared_walk::types::WalkWork,
    crate::domain::shared_walk::types::prepared_walk::value::NativeBatch,
    crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk,
    crate::domain::table_cells::types::TreeForm, crate::domain::table_cells::types::TreeRows,
    crate::domain::table_scratch::types::OperatorScratch,
    ryo_values::domain::files::types::SourceView,
};

fn plan(name: &str) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(if name.is_empty() { 2 } else { 1 })),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(name.into()))),
                    origin: None,
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

fn predicate_order_preserves_errors_and_short_circuits() {
    let mut first = plan("absent-kind");
    let Operation::Select { comparisons, .. } = &mut first.steps[0].operation else {
        unreachable!()
    };
    let invalid = Comparison {
        test: Test::Matches,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("[".into()))),
        origin: None,
    };
    comparisons.insert(0, invalid.clone());
    let mut second = plan("absent-kind");
    let Operation::Select { comparisons, .. } = &mut second.steps[0].operation else {
        unreachable!()
    };
    comparisons.push(invalid);
    let plans = [first, second];
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let batch = evaluate_plans(&prepared, &bindings, &[ClassPlace(0), ClassPlace(1)]);
    assert!(matches!(
        batch.class(ClassPlace(0)).map(|result| &result.completion),
        Some(Err(
            crate::domain::operating::errors::OperatingError::InvalidRegex { .. }
        ))
    ));
    assert!(batch.class(ClassPlace(1)).unwrap().positions.0.is_empty());
}

pub(crate) fn add_pattern(plans: &mut [Plan; 4], parameter: u32) {
    for rule in [0, 2] {
        let Operation::Select { comparisons, .. } = &mut plans[rule].steps[0].operation else {
            unreachable!();
        };
        comparisons.push(Comparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(2)),
            right: Operand::Parameter(ParameterId(parameter)),
            origin: None,
        });
    }
}

fn a_reached_semantic_error_is_kept_for_its_class() {
    let mut plans = plans();
    add_pattern(&mut plans, 0);
    let mut bindings = vec![Vec::new(); plans.len()];
    bindings[0] = vec![ParameterBinding::Text(LiteralText("[".into()))];
    bindings[2] = vec![ParameterBinding::Text(LiteralText("[".into()))];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    assert_eq!(prepared.classes()[0].members.len(), 2);
    let batch = evaluate_plans(
        &prepared,
        &bindings,
        &[ClassPlace(0), ClassPlace(1), ClassPlace(2)],
    );
    assert!(matches!(
        batch.class(ClassPlace(0)).map(|result| &result.completion),
        Some(Err(_))
    ));
    assert_eq!(batch.visits(ClassPlace(0)).0, 1);
    assert_eq!(batch.class(ClassPlace(1)).unwrap().positions.0, [2]);
}

fn different_kind_errors_keep_the_first_source_row() {
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
            unreachable!();
        };
        *comparisons = vec![
            Comparison {
                test: Test::NotEqual,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("file".into()))),
                origin: None,
            },
            Comparison {
                test: Test::Matches,
                left: Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
                right: Operand::Column(ColumnIndex(2)),
                origin: None,
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
    let batch = evaluate(
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
    );
    let Some(Err(crate::domain::operating::errors::OperatingError::InvalidRegex { reason })) =
        batch.class(ClassPlace(0)).map(|result| &result.completion)
    else {
        panic!("first source row must retain its regex error");
    };
    assert!(reason.contains('['), "{reason}");
    assert_eq!(batch.visits(ClassPlace(0)).0, 1);
}

fn an_empty_next_file_has_no_stale_native_publication() {
    let plans = plans();
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let mut work = WalkWork::default();
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
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut operator_scratch,
    };
    let bound = bindings
        .iter()
        .cloned()
        .map(Vec::into_boxed_slice)
        .collect::<Vec<_>>();
    let first = evaluate(
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
        &mut work,
    );
    assert_eq!(first.class(ClassPlace(0)).unwrap().positions.0, [1, 3]);
    let empty = ryo_language::domain::elements::types::Elements::default();
    let next = evaluate(
        &prepared,
        Relation::SyntaxTree,
        &crate::domain::table_model::types::Rows::Tree(TreeRows {
            elements: &empty,
            file: FileId(1),
            form: TreeForm::Elements,
        }),
        &bound,
        &mut context,
        &[ClassPlace(0), ClassPlace(1), ClassPlace(2)],
        &mut work,
    );
    assert!((0..prepared.classes().len()).all(|at| {
        next.class(ClassPlace(at))
            .is_some_and(|result| result.positions.0.is_empty() && result.completion.is_ok())
    }));
    assert_eq!(first.class(ClassPlace(0)).unwrap().positions.0, [1, 3]);
}

#[derive(Clone, Copy)]
enum Scenario {
    PredicateOrder,
    Distinct,
    Ready,
    Failure,
    SourceOrder,
    NextFile,
}

type Input = Scenario;
type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("predicates preserve authored error and short-circuit order"),
        word: Word::Succeeds,
        input: || Scenario::PredicateOrder,
        expect: Expect::Value(1),
    },
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
    Case {
        name: CaseName("reached semantic error retained"),
        word: Word::Succeeds,
        input: || Scenario::Failure,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("first source row owns first error"),
        word: Word::Succeeds,
        input: || Scenario::SourceOrder,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("next file drops previous native publication"),
        word: Word::Succeeds,
        input: || Scenario::NextFile,
        expect: Expect::Value(1),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::PredicateOrder => predicate_order_preserves_errors_and_short_circuits(),
        Scenario::Distinct => distinct_single_consumer_predicates_share_the_walk(),
        Scenario::Ready => any_equivalent_consumer_can_name_the_ready_class(),
        Scenario::Failure => a_reached_semantic_error_is_kept_for_its_class(),
        Scenario::SourceOrder => different_kind_errors_keep_the_first_source_row(),
        Scenario::NextFile => an_empty_next_file_has_no_stale_native_publication(),
    }
    Ok(1)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_values::domain::query_bindings::types::ParameterBinding;

fn relation_plan(relation: Relation, comparisons: Vec<Comparison>) -> Plan {
    let mut selected = plan("word");
    selected.steps[0].operation = Operation::Select {
        from: PlanInput::Relation(relation),
        comparisons,
        kept: Vec::new(),
    };
    selected
}

fn numbered(column: u32, value: i64) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Number(Integer(value))),
        origin: None,
    }
}

fn with_context<T>(symbols: &mut Symbols, run: impl FnOnce(&mut OperatorContext<'_>) -> T) -> T {
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    run(&mut OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: symbols.into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    })
}

#[test]
fn generic_rows_share_one_walk_under_exact_typed_bindings_and_relation() {
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
    let prepared = PreparedWalk::compile(&plans, &bindings);
    assert_eq!(prepared.classes().len(), 4);
    // The fourth class belongs to another source and receives no visits.
    let columns = (0..9)
        .map(|at| FactColumn(if at == 8 { vec![1, 2, 1] } else { vec![0; 3] }))
        .collect::<Vec<_>>();
    let rows = Rows::Stored(ColumnsView {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::SourceLocation),
        columns: &columns,
    });
    let bound = bindings
        .iter()
        .cloned()
        .map(Vec::into_boxed_slice)
        .collect::<Vec<_>>();
    let batch = with_context(&mut Symbols::default(), |context| {
        evaluate(
            &prepared,
            Relation::SourceLocation,
            &rows,
            &bound,
            context,
            &[ClassPlace(0), ClassPlace(1), ClassPlace(2)],
            &mut WalkWork::default(),
        )
    });
    assert_eq!(batch.counts().passes.0, 1);
    assert_eq!(batch.counts().source_rows.0, 3);
    assert_eq!(batch.class(ClassPlace(0)).unwrap().positions.0, [0, 2]);
    assert_eq!(batch.class(ClassPlace(1)).unwrap().positions.0, [1]);
    assert!(batch.class(ClassPlace(3)).is_none());
    assert_eq!(batch.visits(ClassPlace(3)).0, 0);
    // Reuse the exact program for a second schema; no SourceLocation class runs.
    let other = ryo_values::domain::rows::types::Table {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::SemanticSyntax).to_vec(),
        cells: Vec::new(),
        count: RowCount(0),
    };
    let empty = with_context(&mut Symbols::default(), |context| {
        evaluate(
            &prepared,
            Relation::SemanticSyntax,
            &Rows::Computed(&other),
            &bound,
            context,
            &[ClassPlace(3)],
            &mut WalkWork::default(),
        )
    });
    assert!(empty.class(ClassPlace(0)).is_none());
    assert!(empty.class(ClassPlace(3)).unwrap().positions.0.is_empty());
}

#[derive(Default)]
struct Collected(Vec<Vec<CellValue>>);
impl crate::domain::operating::types::Emit for Collected {
    fn row(
        &mut self,
        _: &mut OperatorContext<'_>,
        row: &[CellValue],
    ) -> Result<(), OperatingError> {
        self.0.push(row.to_vec());
        Ok(())
    }
}

#[test]
fn generic_late_error_replays_exact_accepted_prefix_and_sink_refusal() {
    let comparison = Comparison {
        test: Test::Matches,
        left: Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
        right: Operand::Column(ColumnIndex(2)),
        origin: None,
    };
    let plans = [relation_plan(Relation::SemanticSyntax, vec![comparison])];
    let prepared = PreparedWalk::compile(&plans, &[vec![]]);
    let mut symbols = Symbols::default();
    let valid = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ByteSlice(b""),
        NameText("x"),
    );
    let invalid = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ByteSlice(b""),
        NameText("["),
    );
    let table = ryo_values::domain::rows::types::Table {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::SemanticSyntax).to_vec(),
        cells: [valid, invalid, valid]
            .iter()
            .enumerate()
            .flat_map(|(at, text)| {
                [
                    CellValue::Node(NodeId(at as u64)),
                    CellValue::Symbol(valid),
                    CellValue::Symbol(*text),
                    CellValue::File(FileId(0)),
                    CellValue::Span(Span {
                        start: Offset(0),
                        end: Offset(1),
                    }),
                ]
            })
            .collect(),
        count: RowCount(3),
    };
    let source = Rows::Computed(&table);
    with_context(&mut symbols, |context| {
        let batch = evaluate(
            &prepared,
            Relation::SemanticSyntax,
            &source,
            &[Vec::new().into_boxed_slice()],
            context,
            &[ClassPlace(0)],
            &mut WalkWork::default(),
        );
        let result = batch.class(ClassPlace(0)).unwrap();
        assert_eq!(result.positions.0, [0]);
        assert_eq!(batch.visits(ClassPlace(0)).0, 2);
        assert!(matches!(
            &result.completion,
            Err(OperatingError::InvalidRegex { .. })
        ));
        let mut collected = Collected::default();
        let replayed = crate::domain::replay::service::selected(
            result,
            &source,
            crate::domain::replay::types::SelectionColumns::Columns(&[
                ColumnIndex(2),
                ColumnIndex(0),
                ColumnIndex(2),
            ]),
            context,
            &mut collected,
        );
        assert!(matches!(replayed, Err(OperatingError::InvalidRegex { .. })));
        assert_eq!(
            collected.0,
            vec![vec![
                CellValue::Symbol(valid),
                CellValue::Node(NodeId(0)),
                CellValue::Symbol(valid)
            ]]
        );
        struct Refuse;
        impl crate::domain::operating::types::Emit for Refuse {
            fn row(
                &mut self,
                _: &mut OperatorContext<'_>,
                _: &[CellValue],
            ) -> Result<(), OperatingError> {
                Err(OperatingError::NotRead)
            }
        }
        assert!(matches!(
            crate::domain::replay::service::selected(
                result,
                &source,
                crate::domain::replay::types::SelectionColumns::All,
                context,
                &mut Refuse
            ),
            Err(OperatingError::NotRead)
        ));
    });
}

#[test]
fn invalid_generic_representation_or_relation_schema_declines_without_semantic_work() {
    let plans = [relation_plan(
        Relation::SourceLocation,
        vec![numbered(8, 1)],
    )];
    let prepared = PreparedWalk::compile(&plans, &[vec![]]);
    let mut table = ryo_values::domain::rows::types::Table {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::SourceLocation).to_vec(),
        cells: vec![],
        count: RowCount(1),
    };
    with_context(&mut Symbols::default(), |context| {
        let invalid = evaluate(
            &prepared,
            Relation::SourceLocation,
            &Rows::Computed(&table),
            &[Vec::new().into_boxed_slice()],
            context,
            &[ClassPlace(0)],
            &mut WalkWork::default(),
        );
        assert!(invalid.class(ClassPlace(0)).is_none());
        assert_eq!(
            invalid.refusal(),
            Some(crate::domain::shared_walk::types::WalkRefusal::Representation)
        );
        assert_eq!(invalid.counts().passes.0, 0);
        table.count = RowCount(usize::MAX);
        let oversized = evaluate(
            &prepared,
            Relation::SourceLocation,
            &Rows::Computed(&table),
            &[Vec::new().into_boxed_slice()],
            context,
            &[ClassPlace(0)],
            &mut WalkWork::default(),
        );
        assert!(oversized.class(ClassPlace(0)).is_none());
        assert_eq!(oversized.counts().passes.0, 0);
        table.count = RowCount(0);
        let wrong_schema = evaluate(
            &prepared,
            Relation::SemanticSyntax,
            &Rows::Computed(&table),
            &[Vec::new().into_boxed_slice()],
            context,
            &[ClassPlace(0)],
            &mut WalkWork::default(),
        );
        assert!(wrong_schema.class(ClassPlace(0)).is_none());
        assert!(
            wrong_schema.refusal().is_none(),
            "a foreign ready class is not a representation refusal"
        );
        assert_eq!(wrong_schema.counts().passes.0, 0);
    });
}

use crate::domain::{operating::errors::OperatingError, table_model::types::Rows};
use ryo_relations::domain::relations::types::{
    ColumnsView, FactColumn, Integer, NodeId, Span, Value as CellValue,
};
use ryo_values::domain::{plan_parts::types::WholeNumber, rows::types::RowCount};

#[test]
fn an_unready_class_of_a_demanded_rule_performs_no_semantic_work() {
    let mut selected = plan("word");
    selected.steps.push(PlanStep {
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::SyntaxTree),
            comparisons: vec![Comparison {
                test: Test::Matches,
                left: Operand::Column(ColumnIndex(2)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("[".into()))),
                origin: None,
            }],
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    });
    let bindings = vec![Vec::new()];
    let prepared = PreparedWalk::compile(&[selected], &bindings);
    let classes = prepared
        .class_of(crate::domain::coverage::types::RuleIndex(0))
        .unwrap();
    let ready = classes[0].unwrap();
    let unready = classes[1].unwrap();
    assert_ne!(ready, unready);
    assert_eq!(
        prepared.classes()[ready.0].first.rule,
        prepared.classes()[unready.0].first.rule
    );
    let idle = evaluate_plans(&prepared, &bindings, &[]);
    assert!(idle.class(ready).is_none());
    assert!(
        idle.refusal().is_none(),
        "no demand cannot invent a representation refusal"
    );
    assert!(idle.class(unready).is_none());
    assert_eq!(idle.counts().passes.0, 0);
    assert_eq!(idle.counts().source_rows.0, 0);
    let batch = evaluate_plans(&prepared, &bindings, &[ready]);
    assert_eq!(batch.class(ready).unwrap().positions.0, [1, 3]);
    assert!(batch.class(ready).unwrap().completion.is_ok());
    assert!(batch.class(unready).is_none());
    assert_eq!(batch.visits(unready).0, 0);
    // The skipped predicate really errors when its own continuation becomes ready.
    let later = evaluate_plans(&prepared, &bindings, &[unready]);
    assert!(later.class(ready).is_none());
    assert_eq!(later.visits(ready).0, 0);
    assert!(matches!(
        later.class(unready).map(|result| &result.completion),
        Some(Err(OperatingError::InvalidRegex { .. }))
    ));
}
