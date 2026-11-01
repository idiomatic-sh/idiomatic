//! The `calling` service's cases, on query text read by the real syntax adapter: the calls that shape rows.

use ryo_language::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::calling::service::lower_call;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::selecting::service::lower_select;
use crate::outbound::syntax::factory::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::{Operation, PlanStep, StepScope};
use ryo_values::domain::program::types::{Expression, QueryView};

/// The query text a row gives.
type Input = &'static str;
/// The operations of the plan's steps, in order, named.
type Value = &'static str;
/// What the lowering refuses a row with.
type Failure = LoweringError;

const CALLED: &str = "let f = select * from \"function\"; let p = project(from=f, columns=[\"file\"]); emit p subject file key(file);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_call_is_one_step_of_its_own_operation"),
    word: Word::Succeeds,
    input: || CALLED,
    expect: Expect::Value("select project"),
}];

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
        Operation::SpanBetween { .. } => "span-between",
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

/// Lowers the text step by step with the part's own lowering, but the answer must be one of the known lists.
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
    let known = ["select project"];
    Ok(known
        .into_iter()
        .find(|list| *list == named(&builder.steps))
        .unwrap_or("unexpected"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
