//! The `reshaping` service's cases, on query text read by the real syntax adapter: the calls that count, digest, capture and read rows.

use ryo_language::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::ArgumentFault;
use crate::domain::lowering::types::Builder;
use crate::domain::reshaping::service::{
    call_capture, call_group_count, call_group_digest, call_rank, call_rows, call_words,
};
use crate::domain::selecting::service::lower_select;
use crate::domain::syntax::constants::{operator_named, parameter_kind_named};
use crate::outbound::syntax::factory::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::{Operation, PlanStep, QueryOperator, StepScope};
use ryo_values::domain::program::types::{Expression, QueryView};

/// The query text a row gives.
type Input = &'static str;
/// The operations of the plan's steps, in order, named.
type Value = &'static str;
/// What the lowering refuses a row with.
type Failure = LoweringError;

const PER_FILE_COUNT: &str = "let f = select * from \"function\"; let c = group-count(from=f, keys=[\"file\"], count=\"n\"); emit c subject file key(n);";
const PER_FILE_DIGEST: &str = "let f = select * from \"function\"; let d = group-digest(from=f, keys=[\"file\"], distinct=\"name\", digest=\"d\", count=\"n\"); emit d subject file key(n);";
const CROSS_DIGEST: &str = "let f = select * from \"function\"; let d = group-digest(from=f, keys=[\"name\"], distinct=\"file\", digest=\"d\", count=\"n\"); emit d subject name key(n);";
/// The functions of each name counted by the files they are in: only the name and the file are asked of the rows.
const DISTINCT_COUNT: &str = "let f = select * from \"function\"; let c = group-count(from=f, keys=[\"name\"], count=\"n\", distinct=\"file\"); emit c subject name key(n);";
/// The tables a parameter holds, as rows.
const TABLES_AS_ROWS: &str = "param tables: list<table>; let r = rows(from=$tables, columns=[\"name\"]); emit r subject name key(name);";
/// A parameter that holds no tables, read as rows.
const NUMBER_AS_ROWS: &str = "param limit: integer; let r = rows(from=$limit, columns=[\"name\"]); emit r subject name key(name);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_count_grouped_by_file_stays_per_file"),
        word: Word::Succeeds,
        input: || PER_FILE_COUNT,
        expect: Expect::Value("select group-count"),
    },
    Case {
        name: CaseName("a_digest_grouped_by_file_stays_per_file"),
        word: Word::Succeeds,
        input: || PER_FILE_DIGEST,
        expect: Expect::Value("select group-digest"),
    },
    Case {
        name: CaseName("a_digest_grouped_by_name_needs_several_files"),
        word: Word::Succeeds,
        input: || CROSS_DIGEST,
        expect: Expect::Value("select group-digest*"),
    },
    Case {
        name: CaseName("a_count_of_the_distinct_values_of_a_column_is_one_step_after_the_files"),
        word: Word::Succeeds,
        input: || DISTINCT_COUNT,
        expect: Expect::Value("select group-count*"),
    },
    Case {
        name: CaseName("the_tables_of_a_parameter_are_one_step_after_the_files"),
        word: Word::Succeeds,
        input: || TABLES_AS_ROWS,
        expect: Expect::Value("rows*"),
    },
    Case {
        name: CaseName("rows_of_a_parameter_that_holds_no_tables_are_refused"),
        word: Word::Refuses,
        input: || NUMBER_AS_ROWS,
        expect: Expect::Refusal(|error| {
            selects(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::ExpectedTables,
                    ..
                }
            ))
        }),
    },
];

/// The relations the rows may read.
const CATALOG: &[Relation] = &[
    Relation::Function,
    Relation::SyntaxChild,
    Relation::SyntaxTree,
    Relation::WorkspaceMember,
];

/// What an operation is, for the answer.
fn operation_name(operation: &Operation) -> &'static str {
    match operation {
        Operation::SpanBetween { .. } => "span-between",
        Operation::Select { .. } => "select",
        Operation::Project {
            distinct: RepeatedRows::Kept,
            ..
        } => "project-keyed",
        Operation::Project { .. } => "project",
        Operation::Join { .. } => "join",
        Operation::Replace { .. } => "replace",
        Operation::OrderedMatch { .. } => "ordered-match",
        Operation::GroupCount { .. } => "group-count",
        Operation::GroupDigest { .. } => "group-digest",
        Operation::Antijoin { .. } => "antijoin",
        Operation::Union { .. } => "union",
        Operation::Extend { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Hosted { .. } => "other",
        Operation::Rows { .. } => "rows",
        Operation::Axis { .. } => "axis",
    }
}

/// The operations of the steps, in order, each starred when it runs after the files.
fn named(steps: &[PlanStep]) -> String {
    steps
        .iter()
        .map(|step| {
            format!(
                "{}{}",
                operation_name(&step.operation),
                if step.scope == StepScope::CrossFile {
                    "*"
                } else {
                    ""
                }
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lowers the text step by step with the part's own lowering, but the answer must be one of the known lists.
fn subject(text: Input) -> Result<Value, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("the row's text is readable");
    let mut builder = Builder {
        catalog: CATALOG,
        hosted_steps: Vec::new(),
        parameters: &program.parameters,
        kinds: program
            .parameters
            .iter()
            .map(|parameter| {
                parameter_kind_named(&parameter.kind.0).expect("the row's parameter type is known")
            })
            .collect(),
        steps: Vec::new(),
        lowered: Vec::new(),
        span: program.emit.span,
        origin: None,
        comparison_origins: Vec::new(),
    };
    for step in &program.steps {
        builder.span = step.span;
        match &step.expression {
            Expression::Select(select) => lower_select(&mut builder, &step.name, select)?,
            Expression::Call(call) => {
                let operator = operator_named(&call.name.0).expect("the row's call is an operator");
                match operator {
                    QueryOperator::GroupCount => call_group_count(&mut builder, &step.name, call)?,
                    QueryOperator::GroupDigest => {
                        call_group_digest(&mut builder, &step.name, call)?;
                    }
                    QueryOperator::Capture => call_capture(&mut builder, &step.name, call)?,
                    QueryOperator::Rank => call_rank(&mut builder, &step.name, call)?,
                    QueryOperator::Rows => call_rows(&mut builder, &step.name, call)?,
                    QueryOperator::Words => call_words(&mut builder, &step.name, call)?,
                    QueryOperator::Replace
                    | QueryOperator::SpanBetween
                    | QueryOperator::OrderedMatch
                    | QueryOperator::Select
                    | QueryOperator::Project
                    | QueryOperator::Extend
                    | QueryOperator::Join
                    | QueryOperator::Union
                    | QueryOperator::Intersection
                    | QueryOperator::Difference
                    | QueryOperator::Antijoin
                    | QueryOperator::Axis
                    | QueryOperator::Closure
                    | QueryOperator::Cycles => {
                        panic!("the row's call is not one this part lowers")
                    }
                }
            }
        }
    }
    let known = [
        "select group-count",
        "select group-digest",
        "select group-digest*",
        "select group-count*",
        "rows*",
    ];
    Ok(known
        .into_iter()
        .find(|list| *list == named(&builder.steps))
        .unwrap_or("unexpected"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
