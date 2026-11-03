//! The `idioms` service's cases, on query text read by the real syntax adapter: the pairs of functions with equal sets of texts.

use ryo_language::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::calling::service::lower_call;
use crate::domain::idioms::service::{push_same_sets, recognize_same_sets};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, Sourced};
use crate::domain::resolving::service::{input_argument, positions_of, words_of};
use crate::domain::rewriting::service::same_sets;
use crate::domain::selecting::service::lower_select;
use crate::domain::syntax::types::ArgumentSlot;
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

/// Whether two functions of one name have the same tokens, said as two antijoins of their texts each way.
const SAME_TEXTS: &str = "let t = project(from=\"syntax-tree\", columns=[\"id\", \"text\"]); let f = select * from \"function\"; let p0 = select l.\"id\" as \"function\", r.\"id\" as \"other\", l.\"name\" as \"name\" from f as l join f as r on l.\"name\" = r.\"name\"; let pairs = select * from p0 where function != other; let own-texts = select l.\"function\" as \"function\", l.\"other\" as \"other\", r.\"text\" as \"text\" from pairs as l join t as r on l.\"function\" = r.\"id\"; let own-only = antijoin(left=own-texts, right=t, left-on=[\"other\", \"text\"], right-on=[\"id\", \"text\"]); let own-miss = project(from=own-only, columns=[\"function\", \"other\"]); let other-texts = select l.\"function\" as \"function\", l.\"other\" as \"other\", r.\"text\" as \"text\" from pairs as l join t as r on l.\"other\" = r.\"id\"; let other-only = antijoin(left=other-texts, right=t, left-on=[\"function\", \"text\"], right-on=[\"id\", \"text\"]); let other-miss = project(from=other-only, columns=[\"function\", \"other\"]); let miss = union(left=own-miss, right=other-miss); let copies = antijoin(left=pairs, right=miss, left-on=[\"function\", \"other\"], right-on=[\"function\", \"other\"]); emit copies subject function key(other);";
/// The same shape over other columns: files holding a function of one name whose sets of texts are alike.
const SAME_PARAMETERS: &str = "let t = project(from=\"syntax-tree\", columns=[\"file\", \"text\"]); let f = select * from \"function\"; let p0 = select l.\"file\" as \"function\", r.\"file\" as \"other\", l.\"name\" as \"label\" from f as l join f as r on l.\"name\" = r.\"name\"; let pairs = select * from p0 where function != other; let own-texts = select l.\"function\" as \"function\", l.\"other\" as \"other\", r.\"text\" as \"text\" from pairs as l join t as r on l.\"function\" = r.\"file\"; let own-only = antijoin(left=own-texts, right=t, left-on=[\"other\", \"text\"], right-on=[\"file\", \"text\"]); let own-miss = project(from=own-only, columns=[\"function\", \"other\"]); let other-texts = select l.\"function\" as \"function\", l.\"other\" as \"other\", r.\"text\" as \"text\" from pairs as l join t as r on l.\"other\" = r.\"file\"; let other-only = antijoin(left=other-texts, right=t, left-on=[\"function\", \"text\"], right-on=[\"file\", \"text\"]); let other-miss = project(from=other-only, columns=[\"function\", \"other\"]); let miss = union(left=own-miss, right=other-miss); let copies = antijoin(left=pairs, right=miss, left-on=[\"function\", \"other\"], right-on=[\"function\", \"other\"]); emit copies subject function key(other);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("same_texts_said_by_antijoins_is_said_by_one_digest_for_each_function"),
        word: Word::Succeeds,
        input: || SAME_TEXTS,
        expect: Expect::Value("joined group-digest join join* select* project*"),
    },
    Case {
        name: CaseName("the_same_shape_over_other_relations_is_said_by_digests_too"),
        word: Word::Succeeds,
        input: || SAME_PARAMETERS,
        expect: Expect::Value("joined group-digest join join* select* project*"),
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

/// Lowers the text up to its last step, then says that step with the part's own idiom and names the steps the idiom pushed, but the answer must be one of the known lists.
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
    let (last, earlier) = program
        .steps
        .split_last()
        .expect("the row's text has steps");
    for step in earlier {
        builder.span = step.span;
        match &step.expression {
            Expression::Select(select) => lower_select(&mut builder, &step.name, select)?,
            Expression::Call(call) => lower_call(&mut builder, &step.name, call)?,
        }
    }
    builder.span = last.span;
    let Expression::Call(call) = &last.expression else {
        return Ok("not a call");
    };
    let Sourced {
        input: left,
        fields,
    } = input_argument(&builder, call, ArgumentSlot::Left)?;
    let Sourced {
        input: right,
        fields: right_fields,
    } = input_argument(&builder, call, ArgumentSlot::Right)?;
    let left_on = positions_of(&builder, &fields, &words_of(call, ArgumentSlot::LeftOn)?)?;
    let right_on = positions_of(
        &builder,
        &right_fields,
        &words_of(call, ArgumentSlot::RightOn)?,
    )?;
    let Some(idiom) = same_sets(&builder.steps, left, right, &left_on, &right_on) else {
        return Ok("not the idiom");
    };
    let way = if recognize_same_sets(&builder, idiom).is_some() {
        "joined"
    } else {
        "separate"
    };
    let before = builder.steps.len();
    push_same_sets(&mut builder, &last.name, idiom);
    let pushed = format!("{way} {}", named(&builder.steps[before..]));
    let known = ["joined group-digest join join* select* project*"];
    Ok(known
        .into_iter()
        .find(|list| *list == pushed)
        .unwrap_or("unexpected"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
