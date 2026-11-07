//! A walk publishes nothing for an empty next file, declines invalid generic rows, and does no semantic work for unready classes.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::positions::service_test::{SMALL_SOURCE, SmallTree, small_tree};
use crate::domain::positions::types::ClassPlace;
use crate::domain::shared_walk::service_test::{
    add_pattern, evaluate_plans, evaluate_rows, plan, plans, relation_plan, with_context,
};
use crate::domain::shared_walk::types::WalkWork;
use crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn, FileId, Integer, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral, Test, WholeNumber,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::RowCount;

#[derive(Clone, Copy)]
enum Scenario {
    ReachedError,
    NextFile,
    GenericRows,
    InvalidGeneric,
    UnreadyClass,
}

type Input = Scenario;
type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reached semantic error retained"),
        word: Word::Succeeds,
        input: || Scenario::ReachedError,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("next file drops previous native publication"),
        word: Word::Succeeds,
        input: || Scenario::NextFile,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("generic rows share one walk under exact typed bindings and relation"),
        word: Word::Succeeds,
        input: || Scenario::GenericRows,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(
            "an invalid generic representation or relation schema declines without semantic work",
        ),
        word: Word::Succeeds,
        input: || Scenario::InvalidGeneric,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an unready class of a demanded rule performs no semantic work"),
        word: Word::Succeeds,
        input: || Scenario::UnreadyClass,
        expect: Expect::Value(1),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ReachedError => a_reached_semantic_error_is_kept_for_its_class(),
        Scenario::NextFile => an_empty_next_file_has_no_stale_native_publication(),
        Scenario::GenericRows => {
            generic_rows_share_one_walk_under_exact_typed_bindings_and_relation()
        }
        Scenario::InvalidGeneric => {
            invalid_generic_representation_or_relation_schema_declines_without_semantic_work()
        }
        Scenario::UnreadyClass => an_unready_class_of_a_demanded_rule_performs_no_semantic_work(),
    }
    Ok(1)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
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
        matcher: crate::domain::matching::service_test::matcher(),
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
    let first = evaluate_rows(
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
    let next = evaluate_rows(
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
            .is_some_and(|result| result.positions.0.is_empty() && result.completion == Ok(()))
    }));
    assert_eq!(first.class(ClassPlace(0)).unwrap().positions.0, [1, 3]);
}
fn numbered(column: u32, value: i64) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Number(Integer(value))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
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
        evaluate_rows(
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
        evaluate_rows(
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
        let invalid = evaluate_rows(
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
        let oversized = evaluate_rows(
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
        let wrong_schema = evaluate_rows(
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
#[test]
fn an_unready_class_of_a_demanded_rule_performs_no_semantic_work() {
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
    assert_eq!(batch.class(ready).unwrap().completion, Ok(()));
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
