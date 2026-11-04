//! The `selecting` service's cases, on query text read by the real syntax adapter: the steps a `select` makes.

use ryo_relations::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run, selects};
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

const KEYED: &str =
    "let a = select id, name from \"function\" where name = \"main\"; emit a subject name key(id);";
const EITHER: &str = "let a = select * from \"function\" where name = \"run\" or name = \"inner\"; emit a subject file key(span);";
const EITHER_KEYED: &str = "let a = select id, name from \"function\" where name = \"run\" or (name = \"inner\" and file = file); emit a subject name key(id);";
const UNKNOWN_IN_OR: &str = "let a = select * from \"function\" where name = \"run\" or nothing = \"inner\"; emit a subject file key(span);";
const UNKEYED: &str =
    "let a = select name, file as \"f\" from \"function\"; emit a subject f key(f);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_projection_keeping_a_column_that_says_which_row_it_is_is_one_step_with_the_filter",
        ),
        word: Word::Succeeds,
        input: || KEYED,
        expect: Expect::Value("select"),
    },
    Case {
        name: CaseName("a_projection_keeping_no_such_column_drops_repeats"),
        word: Word::Succeeds,
        input: || UNKEYED,
        expect: Expect::Value("project"),
    },
    Case {
        name: CaseName("an_or_filters_each_alternative_and_unites_them"),
        word: Word::Succeeds,
        input: || EITHER,
        expect: Expect::Value("select select union"),
    },
    Case {
        name: CaseName("an_or_with_a_projection_unites_the_alternatives_before_it"),
        word: Word::Succeeds,
        input: || EITHER_KEYED,
        expect: Expect::Value("select select union project-keyed"),
    },
    Case {
        name: CaseName("an_unknown_column_in_an_alternative_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_IN_OR,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LoweringError::UnknownColumn { name, .. } if name == "nothing"))
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
        if let Expression::Select(select) = &step.expression {
            builder.span = step.span;
            lower_select(&mut builder, &step.name, select)?;
        }
    }
    let known = [
        "select",
        "project",
        "select select union",
        "select select union project-keyed",
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

#[test]
fn many_or_alternatives_keep_each_filter_and_bound_union_depth() {
    let tests = (0..195)
        .map(|number| format!("name = \"alternative_{number}\""))
        .collect::<Vec<_>>();
    let query = format!(
        "let found = select * from \"function\" where {}; emit found subject file key(id);",
        tests.join(" or ")
    );
    let program = winnow().read(QueryView(&query)).expect("readable OR query");
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
        if let Expression::Select(select) = &step.expression {
            builder.span = step.span;
            lower_select(&mut builder, &step.name, select).expect("lowered OR query");
        }
    }
    let mut depths = Vec::new();
    let mut selects = 0;
    let mut unions = 0;
    for step in &builder.steps {
        let depth = match &step.operation {
            Operation::Select { .. } => {
                selects += 1;
                0
            }
            Operation::Union { left, right } => {
                unions += 1;
                let depth_of = |input: &ryo_values::domain::plan_parts::types::Input| match input {
                    ryo_values::domain::plan_parts::types::Input::Step(id) => depths[id.0 as usize],
                    ryo_values::domain::plan_parts::types::Input::Relation(_) => 0,
                };
                1 + depth_of(left).max(depth_of(right))
            }
            other => panic!("unexpected operation in OR reduction: {other:?}"),
        };
        depths.push(depth);
    }
    assert_eq!(selects, 195);
    assert_eq!(unions, 194);
    assert!(depths.last().copied().unwrap_or_default() <= 8);
}
