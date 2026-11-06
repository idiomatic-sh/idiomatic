//! The `closure` and `axis` calls, on query text read by the real syntax adapter.

use crate::domain::lowering::types::Builder;
use crate::domain::selecting::service::lower_select;
use crate::domain::syntax::constants::operator_named;
use crate::domain::traversing::service::call_axis;
use crate::domain::traversing::service::call_closure;
use crate::domain::traversing::service::call_ordered_dfs;
use crate::outbound::syntax::fixtures::winnow::winnow;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::query_operators::types::QueryOperator;
use ryo_values::domain::query_syntax::types::Expression;
use ryo_values::domain::query_tokens::types::QueryView;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::ArgumentFault;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::selects;

/// The children of each function: one axis walks from a node to its children.
const CHILDREN: &str = "let f = select * from \"function\"; let kids = axis(from=f, relation=\"syntax-child\", direction=child, domain=\"syntax-tree\"); emit kids subject id key(related);";

/// The parent of each name: the same walk the other way.
const PARENTS: &str = "let names = select * from \"syntax-tree\" where kind = \"name\"; let owners = axis(from=names, relation=\"syntax-child\", direction=parent, domain=\"syntax-tree\"); emit owners subject id key(related);";

/// The token just before each name under its parent: a sibling walk over the tree's own edges.
const BEFORE_NAMES: &str = "let names = select * from \"syntax-tree\" where kind = \"name\"; let before = axis(from=names, relation=\"syntax-child\", direction=previous-sibling, domain=\"syntax-tree\"); emit before subject id key(related);";

/// A sibling walk over edges a query made, which keep no order of the children.
const SIBLINGS_OF_A_STEP: &str = "let edges = select * from \"syntax-child\"; let names = select * from \"syntax-tree\" where kind = \"name\"; let after = axis(from=names, relation=edges, direction=next-sibling, domain=\"syntax-tree\"); emit after subject id key(related);";

/// The query text a row gives.
type Input = &'static str;

/// The operations of the plan's steps, in order, named.
type Value = &'static str;

/// What the lowering refuses a row with.
type Failure = LoweringError;

/// The relations the rows may read.
const CATALOG: &[Relation] = &[
    Relation::Function,
    Relation::SyntaxChild,
    Relation::SyntaxTree,
    Relation::WorkspaceMember,
];

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

/// What an operation is, for the answer.
fn operation_name(operation: &Operation) -> &'static str {
    match operation {
        Operation::SpanBetween { .. } => "span-between",
        Operation::SpanBounds { .. } => "span-bounds",
        Operation::SourceMatches { .. } => "source-matches",
        Operation::Select { .. } => "select",
        Operation::Project {
            distinct: RepeatedRows::Kept,
            ..
        } => "project-keyed",
        Operation::Project { .. } => "project",
        Operation::Join { .. } => "join",
        Operation::Replace { .. } => "replace",
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
        | Operation::Cycles { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::Hosted { .. } => "other",
        Operation::Rows { .. } => "rows",
        Operation::Axis { .. } => "axis",
        Operation::Closure { .. } => "closure",
    }
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
                match operator {
                    QueryOperator::OrderedDfs => call_ordered_dfs(&mut builder, &step.name, call)?,
                    QueryOperator::Axis => call_axis(&mut builder, &step.name, call)?,
                    QueryOperator::Closure | QueryOperator::Cycles => {
                        call_closure(&mut builder, &step.name, call, operator)?;
                    }
                    QueryOperator::Replace
                    | QueryOperator::SpanBetween
                    | QueryOperator::SpanBounds
                    | QueryOperator::SourceMatches
                    | QueryOperator::RenderSyntax
                    | QueryOperator::FitsRendered
                    | QueryOperator::OrderedMatch
                    | QueryOperator::Select
                    | QueryOperator::Project
                    | QueryOperator::Extend
                    | QueryOperator::Join
                    | QueryOperator::Union
                    | QueryOperator::Intersection
                    | QueryOperator::Difference
                    | QueryOperator::Antijoin
                    | QueryOperator::GroupCount
                    | QueryOperator::GroupDigest
                    | QueryOperator::Capture
                    | QueryOperator::Words
                    | QueryOperator::Rank
                    | QueryOperator::Rows => {
                        panic!("the row's call is not one this part lowers")
                    }
                }
            }
        }
    }
    for (step, lowered) in builder.steps.iter().zip(&builder.lowered) {
        if let Operation::Closure { keys, .. } = &step.operation {
            assert_eq!(lowered.fields.len(), keys.len() + 2);
            assert!(lowered.fields.iter().all(|field| field.unique == crate::domain::lowering::types::Uniqueness::Shared));
            if !keys.is_empty() {
                assert_eq!(&*lowered.fields[0].name.0, "file");
            }
        }
    }
    if let Some(step) = builder.steps.last()
        && let Operation::OrderedDfs {
            root_keys,
            edge_keys,
            ..
        } = &step.operation
    {
        let fields = &builder.lowered.last().expect("DFS step is lowered").fields;
        assert_eq!(root_keys.len(), edge_keys.len());
        assert_eq!(fields.len(), root_keys.len() + 5);
        assert!(
            fields
                .iter()
                .all(|field| field.unique == crate::domain::lowering::types::Uniqueness::Shared)
        );
        assert_eq!(
            fields.last().expect("ordinal exists").kind,
            ryo_relations::domain::relations::types::CellKind::Number
        );
        return Ok("ordered-dfs");
    }
    let known = ["select axis", "select closure"];
    Ok(known
        .into_iter()
        .find(|list| *list == named(&builder.steps))
        .unwrap_or("unexpected"))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("closure_omitted"),
        word: Word::Succeeds,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child); emit c subject parent key(child);",
        expect: Expect::Value("select closure"),
    },
    Case {
        name: CaseName("closure_empty"),
        word: Word::Succeeds,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[]); emit c subject parent key(child);",
        expect: Expect::Value("select closure"),
    },
    Case {
        name: CaseName("closure_one_key"),
        word: Word::Succeeds,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[file]); emit c subject parent key(child);",
        expect: Expect::Value("select closure"),
    },
    Case {
        name: CaseName("closure_multiple_keys"),
        word: Word::Succeeds,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[file, ordinal]); emit c subject parent key(child);",
        expect: Expect::Value("select closure"),
    },
    Case {
        name: CaseName("closure_refuses_repeated_key"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[file,file]); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("closure_refuses_endpoint_key"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[parent]); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("closure_refuses_missing_key"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[missing]); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownColumn { .. }))
        }),
    },
    Case {
        name: CaseName("closure_refuses_scalar_keys"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=file); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::ExpectedNames,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("closure_refuses_numeric_key"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[42]); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::ExpectedNames,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("a_walk_to_the_children_of_a_node_is_one_axis"),
        word: Word::Succeeds,
        input: || CHILDREN,
        expect: Expect::Value("select axis"),
    },
    Case {
        name: CaseName("a_walk_to_the_parent_of_a_node_is_one_axis"),
        word: Word::Succeeds,
        input: || PARENTS,
        expect: Expect::Value("select axis"),
    },
    Case {
        name: CaseName("a_sibling_of_the_tree_is_one_axis"),
        word: Word::Succeeds,
        input: || BEFORE_NAMES,
        expect: Expect::Value("select axis"),
    },
    Case {
        name: CaseName("a_sibling_walk_over_edges_a_query_made_is_refused"),
        word: Word::Refuses,
        input: || SIBLINGS_OF_A_STEP,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::SiblingOverTree,
                    ..
                }
            ))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
