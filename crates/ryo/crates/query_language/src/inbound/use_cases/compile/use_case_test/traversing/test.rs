//! The `traversing` service's cases, on query text read by the real syntax adapter: the calls that walk an axis.

use ryo_relations::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::ArgumentFault;
use crate::domain::lowering::types::Builder;
use crate::domain::selecting::service::lower_select;
use crate::domain::syntax::constants::operator_named;
use crate::domain::traversing::service::{call_axis, call_closure, call_ordered_dfs};
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::selects;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_operators::types::QueryOperator;
use ryo_values::domain::query_syntax::types::Expression;
use ryo_values::domain::query_tokens::types::QueryView;

/// The query text a row gives.
type Input = &'static str;
/// The operations of the plan's steps, in order, named.
type Value = &'static str;
/// What the lowering refuses a row with.
type Failure = LoweringError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordered_dfs_keys"),
        word: Word::Succeeds,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=[file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Value("ordered-dfs"),
    },
    Case {
        name: CaseName("ordered_dfs_without_keys"),
        word: Word::Succeeds,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,output=visit); emit visits subject id key(visit);",
        expect: Expect::Value("ordered-dfs"),
    },
    Case {
        name: CaseName("ordered_dfs_empty_keys"),
        word: Word::Succeeds,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=[],output=visit); emit visits subject id key(visit);",
        expect: Expect::Value("ordered-dfs"),
    },
    Case {
        name: CaseName("ordered_dfs_multiple_keys"),
        word: Word::Succeeds,
        input: || "let roots = select child as root, file, field from \"syntax-child\"; let visits = ordered-dfs(from=roots,root=root,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=ordinal,keys=[file,field],output=visit); emit visits subject id key(visit);",
        expect: Expect::Value("ordered-dfs"),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_missing_root"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=[file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::Missing,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_missing_edges"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,source=parent,destination=child,order=ordinal,edge=field,keys=[file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::Missing,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_unknown_root_column"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=missing,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=[file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownColumn { .. }))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_unknown_order_column"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=missing,edge=field,keys=[file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownColumn { .. }))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_entity_kinds"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=name,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=[file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::MatchingGraphEntities,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_order_kind"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=field,edge=field,keys=[file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::NumericGraphOrder,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_key_kinds"),
        word: Word::Refuses,
        input: || "let e = select parent, child, ordinal, field as file from \"syntax-child\"; let visits = ordered-dfs(from=\"function\",root=id,edges=e,source=parent,destination=child,order=ordinal,edge=ordinal,keys=[file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::MatchingGraphKeys,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_key_list"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=file,output=visit); emit visits subject id key(visit);",
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
        name: CaseName("ordered_dfs_refuses_unknown_key"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=[missing],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownColumn { .. }))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_repeated_keys"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=[file,file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_edge_destination_collision"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=child,keys=[file],output=visit); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_ordinal_root_collision"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=[file],output=id); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("ordered_dfs_refuses_ordinal_key_collision"),
        word: Word::Refuses,
        input: || "let visits = ordered-dfs(from=\"function\",root=id,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,edge=field,keys=[file],output=file); emit visits subject id key(visit);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
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

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
