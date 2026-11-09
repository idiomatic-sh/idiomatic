//! The `reading` service's cases, on values alone: the policy of reading one file, at its actual core owner. Execution and lifetime
//! subjects run through the compiled FileRun controls, not a second reading executor.

use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use std::convert::Infallible;

use super::service::{begin, engine_asked, first_buckets, settle, start_partials};
use super::types::EngineRows;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    finish::types::{ColumnNumber, FileLine},
    render::types::{RowNumber, RowOrder},
    workers::service::tables_for,
};
use ryo_execution::inbound::use_cases::metadata::interface::Recovery;
use ryo_relations::domain::relations::types::{Relation, WorkerNumber};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, InputNumber, StepId, StepScope},
    plans::types::{Emit, Operation, Plan, PlanStep},
    records::{
        service::keep,
        types::{FrontierCoverage, FrontierProof, Kept},
    },
    row_texts::types::{Cell as Typed, CellText},
    runs::types::RuleNumber,
    stored_facts::{
        service::frontier_bucket,
        types::{ArenaBytes, Bucket, Digest, KeptWords, PackedWords},
    },
};

/// A scenario: what a row asks of the part.
type Input = fn() -> Value;
/// What a scenario answers: a yes as one, or what it counts or picks.
type Value = u64;
/// Nothing of this part is refused.
type Failure = Infallible;

fn plan(frontiers: usize) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                comparisons: vec![],
                kept: vec![],
            },
        }],
        frontier: vec![PlanInput::Step(StepId(0)); frontiers],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}

fn input_waits_after_the_ones_before_it() -> Value {
    u64::from(frontier_bucket(Bucket(3), InputNumber(2)) == Bucket(5))
}

fn file_is_counted_and_text_begins_where_the_worker_left_off() -> Value {
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
    assert!(tables.captured.is_empty());
    u64::from(base.0) * 10 + u64::from(tables.reads.0)
}

fn line_count_asks_for_the_engine_rows() -> Value {
    let engine = engine_asked(&[Relation::FilesystemLineCount]);
    u64::from(engine.file == EngineRows::Asked) * 100
        + u64::from(engine.lines == EngineRows::Asked) * 10
        + u64::from(engine.any == EngineRows::Asked)
}

fn recovery_from_syntax_errors_is_partial_in_the_relations_of_code() -> Value {
    let mut tables = tables_for(WorkerNumber(0), &[]);
    start_partials(
        &MetadataUseCaseImpl,
        &mut tables,
        &[],
        Recovery::SyntaxErrors,
        &[Relation::Defines, Relation::FilesystemFile],
    );
    assert_eq!(tables.partials[0].relation, Relation::Defines);
    u64::try_from(tables.partials.len()).unwrap_or(u64::MAX)
}

fn findings_are_put_in_the_order_of_their_keys() -> Value {
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
    u64::from(tables.rendered.order[0].row.0)
}

fn findings_have_bucket_zero_and_each_frontier_the_buckets_after() -> Value {
    u64::from(first_buckets(&[plan(2), plan(1)]) == [Bucket(1), Bucket(3)])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_input_of_the_frontier_waits_in_the_bucket_after_the_ones_before_it"),
        word: Word::Succeeds,
        input: || input_waits_after_the_ones_before_it,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_is_counted_and_its_text_begins_where_the_worker_left_off"),
        word: Word::Succeeds,
        input: || file_is_counted_and_text_begins_where_the_worker_left_off,
        expect: Expect::Value(61),
    },
    Case {
        name: CaseName("a_rule_that_asks_for_the_line_count_asks_for_the_engine_rows"),
        word: Word::Succeeds,
        input: || line_count_asks_for_the_engine_rows,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_recovery_from_syntax_errors_is_partial_in_the_relations_of_code"),
        word: Word::Succeeds,
        input: || recovery_from_syntax_errors_is_partial_in_the_relations_of_code,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("findings_are_put_in_the_order_of_their_keys"),
        word: Word::Succeeds,
        input: || findings_are_put_in_the_order_of_their_keys,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_findings_have_bucket_zero_and_each_frontier_the_buckets_after"),
        word: Word::Succeeds,
        input: || findings_have_bucket_zero_and_each_frontier_the_buckets_after,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Value {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |scenario| {
        Ok::<_, Failure>(subject(scenario))
    });
}
