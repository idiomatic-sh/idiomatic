//! Native heads keep their projected columns and source row order, and unused intents evaluate nothing.
use super::super::super::fixtures::*;
use super::super::super::no_catalog;
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::inbound::use_cases::execution::interface::{
    ExecutionRequest, ExecutionUseCase, FileExecution,
};
use ryo_execution::inbound::use_cases::execution::types::*;
use ryo_language::domain::elements::service::text_at;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Offset, Position, Relation, Span, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, Test};
use ryo_values::domain::plans::types::{Operation, PlanStep, StepScope};
use ryo_values::domain::rows::types::{RowCount, Table};
#[test]
fn distinct_native_heads_keep_reordered_duplicate_columns_and_source_row_order() {
    let (engine, semantics) = engine();
    let mut authored = authored_request(2);
    for (index, kind, kept) in [
        (
            0,
            "word",
            vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(2)],
        ),
        (
            1,
            "other",
            vec![
                ColumnIndex(4),
                ColumnIndex(2),
                ColumnIndex(4),
                ColumnIndex(0),
            ],
        ),
    ] {
        authored.plans[index].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: selection(Selector::Kind(kind)),
            kept,
        };
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }; 2];
    let mut file = request
        .open_file(
            input(FileId(0), b"abc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    let a = Value::Symbol(text_at(&elements, Position(1)));
    let b = Value::Symbol(text_at(&elements, Position(2)));
    let c = Value::Symbol(text_at(&elements, Position(3)));
    let first = Value::Node(node_id(FileId(0), Position(1)));
    let last = Value::Node(node_id(FileId(0), Position(3)));
    assert_eq!(
        complete(file.rule(RuleIndex(0), demand(), &mut symbols).unwrap()),
        Table {
            kinds: vec![CellKind::Symbol, CellKind::Node, CellKind::Symbol],
            count: RowCount(2),
            cells: vec![a, first, a, c, last, c]
        }
    );
    let span = Value::Span(Span {
        start: Offset(1),
        end: Offset(2),
    });
    assert_eq!(
        complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
        Table {
            kinds: vec![
                CellKind::Span,
                CellKind::Symbol,
                CellKind::Span,
                CellKind::Node
            ],
            count: RowCount(1),
            cells: vec![span, b, span, Value::Node(node_id(FileId(0), Position(2)))]
        }
    );
    counts(&file, 1, 4);
    assert_eq!(
        semantics.selects(),
        0,
        "both projected heads use shared replay, without ordinary selector rescans"
    );
    file.finish().unwrap();
    request.finish();
}
#[test]
fn fused_select_reads_projected_ordinals_and_forwards_exact_rows_in_source_order() {
    for (pattern, positions) in [("{a,c}", &[1_u32, 3][..]), ("c", &[3_u32][..])] {
        let (engine, _semantics) = engine();
        let mut authored = authored_request(1);
        authored.plans[0].steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::SyntaxTree),
                    comparisons: selection(Selector::Kind("word")),
                    kept: vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(2)],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Step(StepId(0)),
                    comparisons: vec![compared(Test::GlobAny, 0, text(pattern))],
                    kept: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
                },
            },
        ];
        authored.plans[0].emit.step = StepId(1);
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = engine.begin(&program, &corpus);
        let (elements, mut symbols) = tree(FileId(0), b"abc");
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }];
        let mut file = request
            .open_file(
                input(FileId(0), b"abc", &elements, &intents, &no_catalog),
                &mut symbols,
            )
            .unwrap();
        let mut cells = vec![];
        for position in positions {
            let node = Value::Node(node_id(FileId(0), Position(*position)));
            cells.extend([
                node,
                Value::Symbol(text_at(&elements, Position(*position))),
                node,
            ]);
        }
        assert_eq!(
            complete(file.rule(RuleIndex(0), demand(), &mut symbols).unwrap()),
            Table {
                kinds: vec![CellKind::Node, CellKind::Symbol, CellKind::Node],
                count: RowCount(positions.len()),
                cells
            }
        );
        counts(&file, 1, 4);
        file.finish().unwrap();
        request.finish();
    }
}
#[test]
fn mixed_intents_keep_excluded_errors_unreachable_while_an_active_neighbor_walks() {
    let (engine, semantics) = engine();
    let program = prepared(
        &engine,
        &[
            Selector::BadRegex,
            Selector::RegexBeforeAbsentKind,
            Selector::Kind("other"),
        ],
    );
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"[bc");
    let intents = [
        FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected),
        FileRuleIntent::Undemanded,
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        },
    ];
    let mut file = request
        .open_file(
            input(FileId(0), b"[bc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    assert_eq!(
        complete(file.rule(RuleIndex(2), demand(), &mut symbols).unwrap()),
        expected(FileId(0), &elements, &[2])
    );
    counts(&file, 1, 4);
    for rule in [RuleIndex(0), RuleIndex(1)] {
        assert!(matches!(
            file.rule(rule, demand(), &mut symbols),
            Err(OperatingError::NotRun)
        ));
        assert!(matches!(
            file.emit_rows(rule, &mut symbols, &mut |_| panic!(
                "excluded root cannot deliver rows"
            )),
            Err(OperatingError::NotRun)
        ));
        counts(&file, 1, 4);
    }
    assert_eq!(semantics.selects(), 0);
    assert_eq!(
        file.walk_counts().predicate_rows.0,
        1,
        "only the active other-kind row reaches a predicate"
    );
    file.finish().unwrap();
    request.finish();
}
#[test]
fn a_run_intent_with_no_enabled_roots_does_not_evaluate_its_predicates() {
    let (engine, _) = engine();
    let program = prepared(&engine, &[Selector::BadRegex, Selector::Kind("other")]);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let intents = [
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Disabled,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        },
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        },
    ];
    let mut file = request
        .open_file(
            input(FileId(0), b"abc", &elements, &intents, &no_catalog),
            &mut symbols,
        )
        .unwrap();
    assert_eq!(
        complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
        expected(FileId(0), &elements, &[2]),
    );
    counts(&file, 1, 4);
    assert_eq!(
        file.walk_counts().predicate_rows.0,
        1,
        "an enabled rule is not evidence that any of its roots were demanded"
    );
    file.finish().unwrap();
    request.finish();
}
