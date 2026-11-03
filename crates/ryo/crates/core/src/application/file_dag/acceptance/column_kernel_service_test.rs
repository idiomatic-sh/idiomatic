//! Same-binary differential proof for a real selected-cut/join/filter/project shape.

use crate::application::file_dag::derived::selection::prepare_request;
use crate::application::file_dag::pipeline::service::evaluate_file_with_executor;
use crate::domain::column_kernel::types::KernelStrategy;
use crate::domain::coverage::types::{Lacking, Partial};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::service::pool_for;
use crate::domain::operator::types::{Pool, PreparedPool};
use crate::domain::table_model::service::take_read_row_counts;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::service::{node_id, to_bits};
use ryo_language::domain::relations::types::{
    ColumnsView, FactColumn, FileId, Integer, Offset, Position, Relation, Span, Value as Cell,
};
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input, JoinColumn, LiteralText, Operand, QueryLiteral, RepeatedRows,
    Side, StepId, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use ryo_values::domain::stored_facts::types::PartialCause;

fn column(number: u32) -> ColumnIndex {
    ColumnIndex(number)
}

/// The same compiled family observed at PYI034 cut 116 and PLC0206 cut 233.
fn plan() -> Plan {
    let steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Relation(Relation::Function),
                columns: vec![column(0), column(2)],
                distinct: RepeatedRows::Dropped,
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Relation(Relation::SemanticChild),
                left_on: vec![column(0), column(1)],
                right_on: vec![column(0), column(4)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: column(0),
                    },
                    JoinColumn {
                        side: Side::Left,
                        column: column(1),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: column(1),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: column(3),
                    },
                ],
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Step(StepId(1)),
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(column(3)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("body".into()))),
                    origin: None,
                }],
                kept: Vec::new(),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Step(StepId(2)),
                columns: vec![column(0), column(1), column(2)],
                distinct: RepeatedRows::Dropped,
            },
        },
    ];
    Plan {
        parameters: Vec::new(),
        steps,
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(3),
            subject: column(0),
            key: vec![column(0)],
            headings: Vec::new(),
        },
        // The shared cut is also handed outward, exactly as a real kept cut is.
        frontier: vec![Input::Step(StepId(0))],
    }
}

fn run<'f>(
    plan: &Plan,
    read: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    symbols: &mut Symbols,
    partial: &[Partial],
    strategy: KernelStrategy,
) -> Result<(Pool, PreparedPool, (u64, u64)), OperatingError> {
    let mut pool = pool_for(std::slice::from_ref(plan));
    pool.partial = partial.to_vec();
    let mut prepared = PreparedPool {
        kernel_strategy: strategy,
        ..PreparedPool::default()
    };
    let request = crate::domain::operator::contract::types::RequestOperators::prepare_with_schemas(
        std::slice::from_ref(plan),
        &[vec![]],
        &[],
        &|relation| {
            read(relation).map_or_else(
                || ryo_language::domain::relations::service::kinds(relation),
                |view| view.kinds,
            )
        },
    )?;
    prepare_request(&mut prepared, &request);
    let frame = (strategy == KernelStrategy::Column).then(|| {
        std::cell::RefCell::new(
            crate::application::file_dag::derived::owner::resource_fixture::residual_frame(
                std::slice::from_ref(plan),
                &prepared,
                &[Relation::Function, Relation::SemanticChild],
                partial,
            ),
        )
    });
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
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
    };
    let _previous = take_read_row_counts();
    let result = evaluate_file_with_executor(
        plan,
        read,
        None,
        &mut context,
        &mut pool,
        &mut prepared,
        &[],
        None,
        0,
        frame
            .as_ref()
            .map(|frame| frame as &dyn crate::application::file_dag::interface::DagExecution),
        &crate::application::file_dag::execution::CommonOperatorEngine,
        &request.rules[0],
    );
    let reads = take_read_row_counts();
    result?;
    Ok((pool, prepared, reads))
}

#[test]
fn selected_stored_chain_matches_executor_and_reduces_full_row_reads() -> Result<(), OperatingError>
{
    let mut symbols = Symbols::default();
    let body = intern_derived(&mut symbols, ByteSlice(b""), NameText("body"));
    let other = intern_derived(&mut symbols, ByteSlice(b""), NameText("other"));
    let file = FileId(0);
    let functions = [
        FactColumn(
            (0..64)
                .map(|row| to_bits(Cell::Node(node_id(file, Position(row)))).0)
                .collect(),
        ),
        FactColumn(vec![to_bits(Cell::Symbol(other)).0; 64]),
        FactColumn(vec![to_bits(Cell::File(file)).0; 64]),
        FactColumn(
            (0..64)
                .map(|row| {
                    to_bits(Cell::Span(Span {
                        start: Offset(row),
                        end: Offset(row + 1),
                    }))
                    .0
                })
                .collect(),
        ),
    ];
    let mut parent = Vec::new();
    let mut child = Vec::new();
    let mut ordinal = Vec::new();
    let mut field = Vec::new();
    let mut files = Vec::new();
    for row in 0..64_u32 {
        for repeated in 0..10 {
            parent.push(to_bits(Cell::Node(node_id(file, Position(row)))).0);
            child.push(to_bits(Cell::Node(node_id(file, Position(row + 1000)))).0);
            ordinal.push(to_bits(Cell::Number(Integer(repeated))).0);
            field.push(to_bits(Cell::Symbol(if repeated == 9 { other } else { body })).0);
            files.push(to_bits(Cell::File(file)).0);
        }
    }
    let edges = [
        FactColumn(parent),
        FactColumn(child),
        FactColumn(ordinal),
        FactColumn(field),
        FactColumn(files),
    ];
    let function_view = ColumnsView {
        kinds: ryo_language::domain::relations::service::kinds(Relation::Function),
        columns: &functions,
    };
    let edge_view = ColumnsView {
        kinds: ryo_language::domain::relations::service::kinds(Relation::SemanticChild),
        columns: &edges,
    };
    let read = |relation| match relation {
        Relation::Function => Some(function_view),
        Relation::SemanticChild => Some(edge_view),
        _ => None,
    };
    let plan = plan();
    let (baseline, _, old_reads) = run(&plan, &read, &mut symbols, &[], KernelStrategy::Existing)?;
    let (candidate, prepared, new_reads) =
        run(&plan, &read, &mut symbols, &[], KernelStrategy::Column)?;
    assert_eq!(candidate.steps[3], baseline.steps[3]);
    assert_eq!(candidate.frontier, baseline.frontier);
    assert_eq!(
        candidate.steps[3].count.0, 64,
        "stable first representative of nine duplicate hits"
    );
    assert_eq!(prepared.kernel_counts.compiled_chains, 1);
    assert_eq!(prepared.kernel_counts.eligible_chains, 1);
    assert_eq!(prepared.kernel_counts.covered_rows, 576);
    assert_eq!(prepared.kernel_counts.joined_hits, 640);
    assert!(prepared.kernel_counts.decoded_cells > 0);
    assert!(
        old_reads.0 > 0 && old_reads.1 > 0,
        "independent executor was exercised"
    );
    assert!(
        new_reads.0 * 5 <= old_reads.0,
        "at least 80% fewer actual full-row reads: {old_reads:?} -> {new_reads:?}"
    );
    assert!(
        new_reads.1 * 5 <= old_reads.1,
        "at least 80% fewer full-row cells read: {old_reads:?} -> {new_reads:?}"
    );
    Ok(())
}

#[test]
fn no_hit_and_ragged_right_keep_exact_fallback() -> Result<(), OperatingError> {
    let mut symbols = Symbols::default();
    let other = intern_derived(&mut symbols, ByteSlice(b""), NameText("other"));
    let body = intern_derived(&mut symbols, ByteSlice(b""), NameText("body"));
    let file = FileId(0);
    let parent = node_id(file, Position(1));
    let child = node_id(file, Position(2));
    let functions = [
        FactColumn(vec![to_bits(Cell::Node(parent)).0]),
        FactColumn(vec![to_bits(Cell::Symbol(other)).0]),
        FactColumn(vec![to_bits(Cell::File(file)).0]),
        FactColumn(vec![
            to_bits(Cell::Span(Span {
                start: Offset(0),
                end: Offset(1),
            }))
            .0,
        ]),
    ];
    let full_edges = [
        FactColumn(vec![to_bits(Cell::Node(parent)).0; 2]),
        FactColumn(vec![to_bits(Cell::Node(child)).0; 2]),
        FactColumn(vec![
            to_bits(Cell::Number(Integer(0))).0,
            to_bits(Cell::Number(Integer(1))).0,
        ]),
        FactColumn(vec![to_bits(Cell::Symbol(other)).0; 2]),
        FactColumn(vec![to_bits(Cell::File(file)).0; 2]),
    ];
    let function_view = ColumnsView {
        kinds: ryo_language::domain::relations::service::kinds(Relation::Function),
        columns: &functions,
    };
    let full = |relation| match relation {
        Relation::Function => Some(function_view),
        Relation::SemanticChild => Some(ColumnsView {
            kinds: ryo_language::domain::relations::service::kinds(relation),
            columns: &full_edges,
        }),
        _ => None,
    };
    let plan = plan();
    let (baseline, _, _) = run(&plan, &full, &mut symbols, &[], KernelStrategy::Existing)?;
    let (candidate, prepared, _) = run(&plan, &full, &mut symbols, &[], KernelStrategy::Column)?;
    assert_eq!(baseline.steps[3], candidate.steps[3]);
    assert_eq!(baseline.frontier, candidate.frontier);
    assert_eq!(candidate.steps[3].count.0, 0);
    assert_eq!(prepared.kernel_counts.eligible_chains, 1);

    let ragged_edges = [
        full_edges[0].clone(),
        full_edges[1].clone(),
        FactColumn(vec![to_bits(Cell::Number(Integer(0))).0]),
        FactColumn(vec![to_bits(Cell::Symbol(body)).0; 2]),
        full_edges[4].clone(),
    ];
    let ragged = |relation| match relation {
        Relation::Function => Some(function_view),
        Relation::SemanticChild => Some(ColumnsView {
            kinds: ryo_language::domain::relations::service::kinds(relation),
            columns: &ragged_edges,
        }),
        _ => None,
    };
    let (baseline, _, _) = run(&plan, &ragged, &mut symbols, &[], KernelStrategy::Existing)?;
    let (candidate, prepared, _) = run(&plan, &ragged, &mut symbols, &[], KernelStrategy::Column)?;
    assert_eq!(baseline.steps[3], candidate.steps[3]);
    assert_eq!(baseline.frontier, candidate.frontier);
    assert_eq!(
        prepared.kernel_counts.eligible_chains, 0,
        "ragged provider declines kernel"
    );
    Ok(())
}

#[test]
fn missing_provider_and_partial_file_keep_executor_failure_boundary() -> Result<(), OperatingError>
{
    let mut symbols = Symbols::default();
    let plan = plan();
    let read = |_relation| None;
    assert!(matches!(
        run(&plan, &read, &mut symbols, &[], KernelStrategy::Existing),
        Err(OperatingError::NotRead)
    ));
    assert!(matches!(
        run(&plan, &read, &mut symbols, &[], KernelStrategy::Column),
        Err(OperatingError::NotRead)
    ));
    let partial = [Partial {
        relation: Relation::SemanticChild,
        cause: PartialCause("incomplete".into()),
        lacking: Lacking::Any,
        at: None,
    }];
    // The candidate's complete-source gate refuses before considering an index.
    let empty_function = [
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
    ];
    let empty_edges = [
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
    ];
    let read_empty = |relation| match relation {
        Relation::Function => Some(ColumnsView {
            kinds: ryo_language::domain::relations::service::kinds(relation),
            columns: &empty_function,
        }),
        Relation::SemanticChild => Some(ColumnsView {
            kinds: ryo_language::domain::relations::service::kinds(relation),
            columns: &empty_edges,
        }),
        _ => None,
    };
    let (baseline, _, _) = run(
        &plan,
        &read_empty,
        &mut symbols,
        &partial,
        KernelStrategy::Existing,
    )?;
    let (candidate, prepared, _) = run(
        &plan,
        &read_empty,
        &mut symbols,
        &partial,
        KernelStrategy::Column,
    )?;
    assert_eq!(candidate.steps[3], baseline.steps[3]);
    assert_eq!(candidate.frontier, baseline.frontier);
    assert_eq!(prepared.kernel_counts.eligible_chains, 0);
    Ok(())
}
