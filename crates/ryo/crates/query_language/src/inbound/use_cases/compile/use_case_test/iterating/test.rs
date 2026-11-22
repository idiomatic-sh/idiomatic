//! The `iterating` service's cases, on query text read by the real syntax adapter: a `worklist` call over the steps a query
//! names, its partition keys matched by kind.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::iterating::service::call_worklist;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::ArgumentFault;
use crate::domain::lowering::types::Builder;
use crate::domain::selecting::service::lower_select;
use crate::domain::syntax::constants::operator_named;
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::selects;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::query_operators::types::QueryOperator;
use ryo_values::domain::query_syntax::types::Expression;
use ryo_values::domain::query_tokens::types::QueryView;

/// The query text a row gives.
type Input = &'static str;
/// How many steps the query lowered to.
type Value = usize;
/// What the lowering refuses a row with.
type Failure = LoweringError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_worklist_over_named_steps_lowers"),
        word: Word::Succeeds,
        input: || "let roots = select id as root, name as state, file from \"function\"; let lattice = select name as lower, name as upper from \"function\"; let trace = worklist(from=roots,root=root,state=state,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,before=field,after=field,lattice=lattice,lower=lower,upper=upper,keys=[file],output=reached); emit trace subject root key(reached);",
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("matching_keys_of_different_kinds_are_refused"),
        word: Word::Refuses,
        input: || "let roots = select id as root, name as state, name as file from \"function\"; let lattice = select name as lower, name as upper from \"function\"; let trace = worklist(from=roots,root=root,state=state,edges=\"syntax-child\",source=parent,destination=child,order=ordinal,before=field,after=field,lattice=lattice,lower=lower,upper=upper,keys=[file],output=reached); emit trace subject root key(reached);",
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
];

/// The relations the rows may read.
const CATALOG: &[Relation] = &[Relation::Function, Relation::SyntaxChild];

/// Lowers the text step by step with the part's own lowering.
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
        names: crate::domain::lowering::types::StepNames::default(),
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
                assert_eq!(
                    operator,
                    QueryOperator::Worklist,
                    "the row's call is a worklist"
                );
                call_worklist(&mut builder, &step.name, call)?;
            }
        }
    }
    Ok(builder.steps.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
