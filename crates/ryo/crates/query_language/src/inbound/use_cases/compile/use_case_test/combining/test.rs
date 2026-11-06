//! The `combining` service's cases, on query text read by the real syntax adapter: the set calls.

use ryo_relations::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use crate::domain::combining::service::call_set;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::selecting::service::lower_select;
use crate::domain::syntax::constants::operator_named;
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::program::types::{Expression, QueryView};

/// The query text a row gives.
type Input = &'static str;
/// The operations of the plan's steps, in order, named.
type Value = &'static str;
/// What the lowering refuses a row with.
type Failure = LoweringError;

const ALIGNED: &str = "let a = select name, file from \"function\"; let b = select file, name from \"function\"; let u = union(left=a, right=b); emit u subject file key(name);";
const UNALIGNED: &str = "let a = select name, file from \"function\"; let b = select name, id from \"function\"; let u = union(left=a, right=b); emit u subject file key(name);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_columns_of_a_set_operation_are_matched_by_name"),
        word: Word::Succeeds,
        input: || ALIGNED,
        expect: Expect::Value("project project project-keyed union"),
    },
    Case {
        name: CaseName("inputs_with_other_columns_are_refused"),
        word: Word::Refuses,
        input: || UNALIGNED,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::BadArgument { .. }))
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
        Operation::Select { .. } => "select",
        Operation::Project {
            distinct: RepeatedRows::Kept,
            ..
        } => "project-keyed",
        Operation::Project { .. } => "project",
        Operation::Join { .. } => "join",
        Operation::Replace { .. } => "replace",
        Operation::SpanBetween { .. } => "span-between",
        Operation::SpanBounds { .. } => "span-bounds",
        Operation::SourceMatches { .. } => "source-matches",
        Operation::OrderedMatch { .. } => "ordered-match",
        Operation::OrderedDfs { .. } => "ordered-dfs",
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
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
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
        observation_inputs: &[],
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
            Expression::Call(call) => {
                let operator = operator_named(&call.name.0).expect("the row's call is an operator");
                call_set(&mut builder, &step.name, call, operator)?;
            }
        }
    }
    let known = ["project project project-keyed union"];
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
