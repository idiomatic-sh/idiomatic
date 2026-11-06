//! Domain behavior from typed values, independent of parsing and adapters.
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types as syntax;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

fn select(column: &str) -> syntax::Select {
    syntax::Select {
        columns: syntax::Columns::Listed(vec![syntax::SelectColumn {
            column: syntax::ColumnReference {
                alias: syntax::Alias::Unwritten,
                column: syntax::ColumnName(column.into()),
            },
            rename: syntax::Renaming::Unchanged,
        }]),
        from: syntax::QuerySource {
            reference: syntax::SourceReference::Relation(syntax::WrittenRelation(
                "function".into(),
            )),
            alias: syntax::Alias::Unwritten,
            span: lowering_builder().span,
        },
        join: syntax::Joining::Alone,
        filters: vec![],
    }
}

fn program() -> syntax::Program {
    let span = lowering_builder().span;
    syntax::Program {
        linked: ryo_values::domain::program::types::Linking::Unlinked,
        imports: vec![],
        origins: vec![],
        fragment_contracts: vec![],
        step_origins: Default::default(),
        emit_origin: ryo_values::domain::program::types::EmitOrigin::Unlocated,
        parameters: vec![],
        observations: vec![],
        steps: vec![syntax::LetStep {
            name: syntax::StepName("result".into(), None),
            expression: syntax::Expression::Select(select("file")),
            span,
        }],
        emit: syntax::WrittenEmit {
            step: syntax::StepName("result".into(), None),
            subject: syntax::ColumnName("file".into()),
            key: vec![syntax::ColumnName("file".into())],
            span,
        },
    }
}
fn typed_program_keeps_emitted_projection() {
    let plan = super::service::lower(&program(), &[Relation::Function], &[]).unwrap();
    assert_eq!(plan.emit.step, parts::StepId(0));
    assert_eq!(
        plan.steps,
        vec![plans::PlanStep {
            scope: plans::StepScope::PerFile,
            operation: plans::Operation::Project {
                from: parts::Input::Relation(Relation::Function),
                columns: vec![parts::ColumnIndex(2)],
                distinct: parts::RepeatedRows::Dropped,
            }
        }]
    );
}
fn unknown_parameter_kind_keeps_its_location() {
    let mut program = program();
    let span = program.emit.span;
    program.parameters.push(syntax::ParameterDeclaration {
        name: syntax::WrittenParameter("limit".into()),
        kind: syntax::TypeName("nothing".into()),
        span,
    });
    assert_eq!(
        super::service::lower(&program, &[Relation::Function], &[]),
        Err(LoweringError::UnknownParameterType {
            name: "nothing".into(),
            span
        })
    );
}

fn origin_of(span: syntax::QuerySpan) -> syntax::QueryOrigin {
    syntax::QueryOrigin {
        source_id: syntax::QuerySourceId("module".into()),
        span,
        trace: vec![],
    }
}
fn a_refusal_of_an_expanded_fragment_names_its_origin() {
    let mut program = program();
    let span = program.emit.span;
    program.emit.step = syntax::StepName("missing".into(), None);
    program.emit_origin = ryo_values::domain::program::types::EmitOrigin::Located(origin_of(span));
    assert!(matches!(
        super::service::lower(&program, &[Relation::Function], &[]),
        Err(LoweringError::Located { .. })
    ));
}
fn a_fragment_input_that_is_not_a_relation_is_refused() {
    let mut program = program();
    let span = program.emit.span;
    let first = syntax::StepIdentity(1);
    program.steps[0].name = syntax::StepName("result".into(), Some(first));
    program.fragment_contracts.push(syntax::FragmentContract {
        first,
        result: first,
        inputs: vec![syntax::FragmentContractInput {
            name: syntax::WrittenName("rows".into()),
            value: syntax::ArgumentValue::Number(ryo_relations::domain::relations::types::Integer(1)),
            relation: syntax::RelationSignature {
                columns: vec![],
                span,
            },
            span,
        }],
        output: syntax::RelationSignature {
            columns: vec![],
            span,
        },
        output_span: span,
        origin: origin_of(span),
    });
    assert!(matches!(
        super::service::lower(&program, &[Relation::Function], &[]),
        Err(LoweringError::FragmentContract { .. })
    ));
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("typed_program_keeps_emitted_projection"),
        word: Word::Succeeds,
        input: || typed_program_keeps_emitted_projection,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unknown_parameter_kind_keeps_its_location"),
        word: Word::Succeeds,
        input: || unknown_parameter_kind_keeps_its_location,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_refusal_of_an_expanded_fragment_names_its_origin"),
        word: Word::Succeeds,
        input: || a_refusal_of_an_expanded_fragment_names_its_origin,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_fragment_input_that_is_not_a_relation_is_refused"),
        word: Word::Succeeds,
        input: || a_fragment_input_that_is_not_a_relation_is_refused,
        expect: Expect::Value(()),
    },
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
