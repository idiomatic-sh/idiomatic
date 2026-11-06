//! Reading policy controls at their actual core owner. Execution/lifetime subjects
//! run through the compiled FileRun controls, not a second reading executor.
use super::{
    service::{begin, engine_asked, first_buckets, settle, start_partials},
    types::EngineRows,
};
use crate::domain::{
    finish::types::{ColumnNumber, FileLine},
    render::types::{RowNumber, RowOrder},
    workers::service::tables_for,
};
use ryo_execution::inbound::use_cases::metadata::interface::Recovery;
use ryo_relations::domain::relations::types::{Relation, WorkerNumber};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, InputNumber, StepId, StepScope},
    plans::types::{Emit, Operation, Plan, PlanStep},
    runs::types::RuleNumber,
    records::{
        service::keep,
        types::{Cell as Typed, CellText, FrontierCoverage, FrontierProof, Kept},
    },
    stored_facts::{
        service::frontier_bucket,
        types::{ArenaBytes, Bucket, Digest, KeptWords, PackedWords},
    },
};
#[test]
fn an_input_of_the_frontier_waits_in_the_bucket_after_the_ones_before_it() {
    assert_eq!(frontier_bucket(Bucket(3), InputNumber(2)), Bucket(5));
}
#[test]
fn a_file_is_counted_and_its_text_begins_where_the_worker_left_off() {
    let mut tables = tables_for(WorkerNumber(0), &[]);
    tables.arena = ArenaBytes(vec![0; 6]);
    tables.captured.push(Kept {
        bucket: Bucket(1),
        proof: FrontierProof {
            rule: ryo_values::domain::records::types::ProducerRule(0),
            input: ryo_values::domain::records::types::FrontierOrdinal(0),
            rule_digest: Digest([0; 16]),
            coverage: FrontierCoverage::Complete,
        },
        words: KeptWords(PackedWords(vec![1])),
    });
    let base = begin(&mut tables);
    assert_eq!(u64::from(base.0) * 10 + u64::from(tables.reads.0), 61);
    assert!(tables.captured.is_empty());
}
#[test]
fn a_rule_that_asks_for_the_line_count_asks_for_the_engine_rows() {
    let engine = engine_asked(&[Relation::FilesystemLineCount]);
    assert_eq!(
        u64::from(engine.file == EngineRows::Asked) * 100
            + u64::from(engine.lines == EngineRows::Asked) * 10
            + u64::from(engine.any == EngineRows::Asked),
        11
    );
}
#[test]
fn a_recovery_from_syntax_errors_is_partial_in_the_relations_of_code() {
    let mut tables = tables_for(WorkerNumber(0), &[]);
    start_partials(
        &mut tables,
        &[],
        Recovery::SyntaxErrors,
        &[Relation::Defines, Relation::FilesystemFile],
    );
    assert_eq!(tables.partials.len(), 1);
    assert_eq!(tables.partials[0].relation, Relation::Defines);
}
#[test]
fn findings_are_put_in_the_order_of_their_keys() {
    let mut tables = tables_for(WorkerNumber(0), &[]);
    for path in ["z", "a"] {
        let rendered = &mut tables.rendered;
        let id = keep(&mut rendered.text, Typed::Text(CellText("r")));
        rendered.cells.push(Some(id));
        let file = keep(&mut rendered.text, Typed::File(CellText(path)));
        rendered.cells.push(Some(file));
        rendered
            .cells
            .push(Some(crate::domain::render::service::key_count(
                crate::domain::render::types::KeyCount(1),
            )));
        let key = keep(&mut rendered.text, Typed::Text(CellText("k")));
        rendered.cells.extend([Some(key), None, None]);
        crate::domain::render::service::end_row(rendered);
    }
    tables.rendered.order = [0, 1]
        .map(|row| RowOrder {
            rule: RuleNumber(0),
            line: FileLine(0),
            column: ColumnNumber(0),
            row: RowNumber(row),
        })
        .to_vec();
    settle(
        &mut tables,
        &crate::domain::render::types::Answering::Findings,
    );

    assert_eq!(tables.rendered.order[0].row.0, 1);
}
fn plan(frontiers: usize) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: vec![],
                kept: vec![],
            },
        }],
        frontier: vec![Input::Step(StepId(0)); frontiers],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}
#[test]
fn the_findings_have_bucket_zero_and_each_frontier_the_buckets_after() {
    let first = first_buckets(&[plan(2), plan(1)]);
    assert_eq!(first, [Bucket(1), Bucket(3)]);
}
