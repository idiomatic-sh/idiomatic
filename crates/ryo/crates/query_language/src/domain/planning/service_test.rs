//! The `planning` service's cases, on query text read by the real syntax adapter: where each step of the plan runs.

use ryo_language::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::calling::service::lower_call;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::planning::service::frontier_of;
use crate::domain::selecting::service::lower_select;
use crate::outbound::syntax::factory::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::{Operation, PlanStep, StepScope};
use ryo_values::domain::program::types::{Expression, QueryView};

/// The query text a row gives.
type Input = &'static str;
/// The steps, in order, named, and how many inputs the per-file steps hand over.
type Value = (&'static str, usize);
/// What the lowering refuses a row with.
type Failure = LoweringError;

/// What the project writes from the manifests is not of one file, so the step that reads it runs after the files.
const MANIFESTS: &str =
    "let m = select * from \"workspace-member\"; emit m subject package key(package);";
const TEXT_AFTER_CROSS: &str = "let f = select * from \"function\"; let c = group-count(from=f, keys=[\"name\"], count=\"n\"); let s = select * from c where name = \"main\"; emit s subject name key(n);";
/// Two functions on one span, with the file dropped from both: a span is a place in one file, so the two are of one file.
const SPAN_JOIN: &str = "let f = project(from=\"function\", columns=[\"span\", \"name\"]); let j = select l.\"name\" as \"a\", r.\"name\" as \"b\" from f as l join f as r on l.\"span\" = r.\"span\"; emit j subject a key(b);";
const CROSS_COUNT: &str = "let f = select * from \"function\"; let c = group-count(from=f, keys=[\"name\"], count=\"n\"); emit c subject name key(n);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("what_the_project_writes_is_read_after_the_files"),
        word: Word::Succeeds,
        input: || MANIFESTS,
        expect: Expect::Value(("select*", 1)),
    },
    Case {
        name: CaseName("the_text_of_a_name_after_a_cross_file_step_is_read"),
        word: Word::Succeeds,
        input: || TEXT_AFTER_CROSS,
        expect: Expect::Value(("select group-count* select*", 1)),
    },
    Case {
        name: CaseName("a_join_on_a_span_is_of_one_file_though_the_rows_drop_the_file"),
        word: Word::Succeeds,
        input: || SPAN_JOIN,
        expect: Expect::Value(("project join", 0)),
    },
    Case {
        name: CaseName("a_count_grouped_by_name_needs_several_files"),
        word: Word::Succeeds,
        input: || CROSS_COUNT,
        expect: Expect::Value(("select group-count*", 1)),
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
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => "other",
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

/// Lowers the text and names the steps with where each runs and how many inputs the per-file steps hand over, but the answer must be one of the known lists.
fn subject(text: Input) -> Result<Value, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("the row's text is readable");
    let mut builder = Builder {
        catalog: CATALOG,
        hosted_steps: Vec::new(),
        parameters: &program.parameters,
        kinds: Vec::new(),
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
            Expression::Call(call) => lower_call(&mut builder, &step.name, call)?,
        }
    }
    let known = [
        "select*",
        "select group-count* select*",
        "project join",
        "select group-count*",
    ];
    let steps = known
        .into_iter()
        .find(|list| *list == named(&builder.steps))
        .unwrap_or("unexpected");
    Ok((steps, frontier_of(&builder.steps).len()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
