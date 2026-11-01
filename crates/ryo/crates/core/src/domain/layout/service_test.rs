//! The `layout` service's cases, on the plans of the `pipeline`'s cases.

use ryo_language::domain::relations::types::Relation;
use ryo_language::domain::symbols::types::Symbols;

use super::service::{layout_of, reads_of, shaped};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::types::{PlanIdentity, StepPlace};
use crate::domain::pipeline::service_test::{KINDS, SOURCE, counting_plan, functions, plan};
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::ColumnsView;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Lays out a plan whose last step is emitted.
    LaysOutAnEmittingPlan,
    /// Lays out a plan whose first step is handed to the cross-file steps.
    LaysOutAHandingPlan,
    /// Counts the steps that read each step of a plan.
    CountsReads,
    /// Lays out a plan over a relation nothing provides.
    LaysOutWithoutTheRelation,
    /// Stored selection, extension and graph layout must leave execution storage untouched.
    MetadataOnly,
    /// A refusal keeps its original relation probe order.
    ProbeOrder,
    /// A native graph's frontier still has kinds when no file populated it.
    EmptyNativeFrontier,
}

/// What a row of the cases does.
type Input = Scenario;
/// The steps that keep their rows, times ten, plus the columns of the widest step, or the reads counted.
type Value = usize;
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::MetadataOnly => return metadata_only(),
        Scenario::ProbeOrder => return probe_order(),
        Scenario::EmptyNativeFrontier => return native_frontier(),
        _ => {}
    }
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    let provided = |relation: Relation| {
        if relation == Relation::Function {
            Ok(Rows::Stored(ColumnsView {
                kinds: KINDS,
                columns: &columns,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let nothing = |_: Relation| Err(OperatingError::NotRead);
    match scenario {
        Scenario::LaysOutAnEmittingPlan | Scenario::LaysOutAHandingPlan => {
            let plan = if matches!(scenario, Scenario::LaysOutAnEmittingPlan) {
                plan()
            } else {
                counting_plan()
            };
            let layout = layout_of(&plan, PlanIdentity(0), &provided, &mut context)?;
            let kept = layout.kept.0.iter().filter(|kept| **kept).count();
            Ok(kept * 10 + layout.kinds.iter().map(Vec::len).max().unwrap_or(0))
        }
        Scenario::CountsReads => {
            let plan = plan();
            Ok(reads_of(&plan, StepPlace(0)).0 * 10 + reads_of(&plan, StepPlace(1)).0)
        }
        Scenario::LaysOutWithoutTheRelation => {
            layout_of(&plan(), PlanIdentity(0), &nothing, &mut context)
                .map(|layout| layout.kinds.len())
        }
        Scenario::MetadataOnly | Scenario::ProbeOrder | Scenario::EmptyNativeFrontier => {
            unreachable!()
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("layout_never_reads_stored_payload_or_mutates_execution_room"),
        word: Word::Succeeds,
        input: || Scenario::MetadataOnly,
        expect: Expect::Value(7),
    },
    Case {
        name: CaseName("provider_refusal_preserves_left_then_right_probe_order"),
        word: Word::Succeeds,
        input: || Scenario::ProbeOrder,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("empty_frontier_preserves_native_deep_walk_schema"),
        word: Word::Succeeds,
        input: || Scenario::EmptyNativeFrontier,
        expect: Expect::Value(6),
    },
    Case {
        name: CaseName("only_the_emitted_step_keeps_its_rows_when_each_is_read_once"),
        word: Word::Succeeds,
        input: || Scenario::LaysOutAnEmittingPlan,
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("the_step_handed_to_the_cross_file_steps_keeps_its_rows"),
        word: Word::Succeeds,
        input: || Scenario::LaysOutAHandingPlan,
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("a_step_read_by_a_step_and_emitted_is_read_once_each_way"),
        word: Word::Succeeds,
        input: || Scenario::CountsReads,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_plan_over_a_relation_nobody_read_is_refused"),
        word: Word::Refuses,
        input: || Scenario::LaysOutWithoutTheRelation,
        expect: Expect::Error(OperatingError::NotRead),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// Probe real nonempty Stored views without filtering, interning text or traversing a graph.
fn metadata_only() -> Result<Value, Failure> {
    use ryo_language::domain::relations::types::{CellKind, Integer, Value as Cell};
    use ryo_language::domain::symbols::service::{derived_of, name_count};
    use ryo_values::domain::plan_parts::types::{
        ColumnIndex, Input as Source, LiteralText, Operand, QueryLiteral, StepId,
    };
    use ryo_values::domain::plans::types::{Operation, PlanStep, StepScope};
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let names_before = name_count(&symbols);
    let derived_before = derived_of(&symbols).0.clone();
    let mut scratch = OperatorScratch::default();
    let sentinel = Cell::Number(Integer(99));
    scratch.row.push(sentinel);
    scratch.right.kinds.push(CellKind::Number);
    scratch.right.cells.push(sentinel);
    scratch.right.count.0 = 1;
    let _inserted = scratch.walk.visited.insert(sentinel);
    let row_before = scratch.row.clone();
    let right_before = scratch.right.clone();
    let row_capacity = scratch.row.capacity();
    let mut memo = vec![crate::domain::operating::types::Memo::Unknown];
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut plan = plan();
    plan.steps[1].operation = Operation::Extend {
        from: Source::Step(StepId(0)),
        values: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
            "do-not-intern-layout".into(),
        )))],
    };
    plan.steps.push(PlanStep {
        operation: Operation::Closure {
            edges: Source::Relation(Relation::Function),
            keys: Vec::new(),
            source: ColumnIndex(0),
            destination: ColumnIndex(0),
        },
        scope: StepScope::PerFile,
    });
    plan.emit.subject = ColumnIndex(1);
    let probes = std::cell::RefCell::new(Vec::new());
    let provided = |relation| {
        probes.borrow_mut().push(relation);
        Ok(Rows::Stored(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        }))
    };
    let layout = layout_of(&plan, PlanIdentity(0), &provided, &mut context)?;
    assert_eq!(*probes.borrow(), [Relation::Function, Relation::Function]);
    assert_eq!(name_count(context.symbols), names_before);
    assert_eq!(derived_of(context.symbols).0, derived_before);
    assert_eq!(context.source.0, SOURCE);
    assert_eq!(context.scratch.row, row_before);
    assert_eq!(context.scratch.row.capacity(), row_capacity);
    assert_eq!(context.scratch.right, right_before);
    assert_eq!(context.scratch.walk.visited.len(), 1);
    assert!(context.scratch.walk.visited.contains(&sentinel));
    assert_eq!(context.memo.len(), 1);
    assert_eq!(
        layout.kinds,
        [
            KINDS.to_vec(),
            vec![CellKind::Symbol, CellKind::File, CellKind::Symbol],
            vec![CellKind::Symbol; 2]
        ]
    );
    Ok(layout.kinds.iter().map(Vec::len).sum())
}

/// Availability is checked on both inputs in original order before schema algebra.
fn probe_order() -> Result<Value, Failure> {
    use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Source, JoinColumn, Side};
    use ryo_values::domain::plans::types::Operation;
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        scratch: &mut scratch,
        memo: &mut memo,
    };
    let mut plan = plan();
    plan.steps.truncate(1);
    plan.emit.step = ryo_values::domain::plan_parts::types::StepId(0);
    plan.steps[0].operation = Operation::Join {
        left: Source::Relation(Relation::Function),
        right: Source::Relation(Relation::Comment),
        left_on: Vec::new(),
        right_on: Vec::new(),
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        }],
        filter: Vec::new(),
        view: Vec::new(),
    };
    let probes = std::cell::RefCell::new(Vec::new());
    let provided = |relation| {
        probes.borrow_mut().push(relation);
        if relation == Relation::Comment {
            Err(OperatingError::NotRead)
        } else {
            Ok(Rows::Stored(ColumnsView {
                kinds: KINDS,
                columns: &columns,
            }))
        }
    };
    assert!(matches!(
        layout_of(&plan, PlanIdentity(0), &provided, &mut context),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(*probes.borrow(), [Relation::Function, Relation::Comment]);
    let count = probes.borrow().len();
    Ok(count)
}

/// Empty cross-file handoff uses declared native representation, not a computed graph placeholder.
fn native_frontier() -> Result<Value, Failure> {
    use crate::domain::table_model::types::{Frontier, Table};
    use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, Input as Source, StepId};
    use ryo_values::domain::plans::types::Operation;
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        scratch: &mut scratch,
        memo: &mut memo,
    };
    let mut plan = plan();
    plan.steps.truncate(1);
    plan.emit.step = StepId(0);
    plan.steps[0].operation = Operation::Axis {
        from: Source::Relation(Relation::SyntaxTree),
        edges: Source::Relation(Relation::SyntaxChild),
        domain: Source::Relation(Relation::SyntaxTree),
        direction: Direction::Deep,
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        domain_filter: Vec::new(),
        domain_view: Vec::new(),
        output: Vec::new(),
    };
    let frontier = [Frontier {
        input: Source::Step(StepId(0)),
        table: Table::default(),
    }];
    let answer = shaped(&plan, &frontier, &mut context)?;
    let expected = ryo_language::domain::relations::service::kinds(Relation::SyntaxTree);
    assert_eq!(answer[0].table.kinds[..expected.len()], *expected);
    assert_eq!(
        answer[0].table.kinds.last(),
        Some(&ryo_language::domain::relations::types::CellKind::Node)
    );
    Ok(answer[0].table.kinds.len())
}
