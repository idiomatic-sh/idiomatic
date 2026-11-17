//! The `calling` service's cases, on query text read by the real syntax adapter: the calls that shape rows.

use crate::inbound::use_cases::compile::use_case_test::SELECT_PROJECT;
use ryo_relations::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::calling::service::lower_call;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::selecting::service::lower_select;
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_syntax::types::Expression;
use ryo_values::domain::query_tokens::types::QueryView;

/// The query text a row gives.
type Input = &'static str;
/// The operations of the plan's steps, in order, named.
type Value = &'static str;
/// What the lowering refuses a row with.
type Failure = LoweringError;

const CALLED: &str = "let f = select * from \"function\"; let p = project(from=f, columns=[\"file\"]); emit p subject file key(file);";
/// The files of a union of two name sets of the module identity: a file holds a row in each, so its rows are not distinct on the
/// file the module identity is keyed on.
const NAMES_OF_EITHER_KIND: &str = "let a = select file, \"module-name\" as name from \"module-identity\"; let b = select file, \"package-name\" as name from \"module-identity\"; let u = union(left=a, right=b); let p = project(from=u, columns=[\"file\"]); emit p subject file key(file);";
/// The files of the module identity, its key.
const MODULE_FILES: &str = "let m = select * from \"module-identity\"; let p = project(from=m, columns=[\"file\"]); emit p subject file key(file);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_call_is_one_step_of_its_own_operation"),
        word: Word::Succeeds,
        input: || CALLED,
        expect: Expect::Value(SELECT_PROJECT),
    },
    Case {
        name: CaseName(
            "a_projection_of_rows_not_proven_distinct_on_its_columns_drops_repeated_rows",
        ),
        word: Word::Succeeds,
        input: || NAMES_OF_EITHER_KIND,
        expect: Expect::Value("select* select* union* project*"),
    },
    Case {
        name: CaseName("a_projection_of_a_key_of_a_relation_keeps_its_rows_as_they_are"),
        word: Word::Succeeds,
        input: || MODULE_FILES,
        expect: Expect::Value("select* project-keyed*"),
    },
];

/// The relations the rows may read.
const CATALOG: &[Relation] = &[
    Relation::Function,
    Relation::ModuleIdentity,
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
        Operation::BandJoin(_) => "band-join",
        Operation::Replace { .. } => "replace",
        Operation::SpanBetween { .. } => "span-between",
        Operation::SpanBounds { .. } => "span-bounds",
        Operation::SourceMatches { .. } => "source-matches",
        Operation::OrderedMatch { .. } => "ordered-match",
        Operation::OrderedDfs { .. } => "ordered-dfs",
        Operation::Worklist { .. } => "worklist",
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
            Expression::Call(call) => lower_call(&mut builder, &step.name, call)?,
        }
    }
    let known = [
        "select project",
        "select* project-keyed*",
        "select* select* union* project*",
        "select* select* union* project-keyed*",
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
