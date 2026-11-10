//! Domain behavior from typed values, independent of parsing and adapters.
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types as syntax;
use ryo_values::domain::query_syntax::types as ast;
use ryo_values::domain::query_tokens::types as tokens;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

fn select(column: &str) -> ast::Select {
    ast::Select {
        columns: ast::Columns::Listed(vec![ast::SelectColumn {
            column: ast::ColumnReference {
                alias: ast::Alias::Unwritten,
                column: ryo_values::domain::descriptions::types::ColumnName(column.into()),
            },
            rename: ast::Renaming::Unchanged,
        }]),
        from: ast::QuerySource {
            reference: ast::SourceReference::Relation(tokens::WrittenRelation("function".into())),
            alias: ast::Alias::Unwritten,
            span: lowering_builder().span,
        },
        join: ast::Joining::Alone,
        filters: vec![],
    }
}

fn program() -> syntax::WrittenProgram {
    let span = lowering_builder().span;
    syntax::WrittenProgram {
        linked: ryo_values::domain::query_tokens::types::Linking::Unlinked,
        imports: vec![],
        origins: vec![],
        fragment_contracts: vec![],
        step_origins: Default::default(),
        emit_origin: ryo_values::domain::query_tokens::types::Attribution::Unlocated,
        parameters: vec![],
        observations: vec![],
        steps: vec![syntax::LetStep {
            name: tokens::StepName(
                "result".into(),
                ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
            ),
            expression: ast::Expression::Select(select("file")),
            span,
        }],
        emit: syntax::WrittenEmit {
            step: tokens::StepName(
                "result".into(),
                ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
            ),
            subject: ryo_values::domain::descriptions::types::ColumnName("file".into()),
            key: vec![ryo_values::domain::descriptions::types::ColumnName(
                "file".into(),
            )],
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
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
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
        parameter: tokens::WrittenParameter("limit".into()),
        kind: tokens::TypeName("nothing".into()),
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

fn origin_of(span: tokens::QuerySpan) -> tokens::QueryOrigin {
    tokens::QueryOrigin {
        source_id: tokens::QuerySourceId("module".into()),
        span,
        trace: vec![],
    }
}
fn a_refusal_of_an_expanded_fragment_names_its_origin() {
    let mut program = program();
    let span = program.emit.span;
    program.emit.step = tokens::StepName(
        "missing".into(),
        ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
    );
    program.emit_origin =
        ryo_values::domain::query_tokens::types::Attribution::Located(origin_of(span));
    assert!(matches!(
        super::service::lower(&program, &[Relation::Function], &[]),
        Err(LoweringError::Located { .. })
    ));
}
fn a_fragment_input_that_is_not_a_relation_is_refused() {
    let mut program = program();
    let span = program.emit.span;
    let first = tokens::StepIdentity(1);
    program.steps[0].name = tokens::StepName(
        "result".into(),
        ryo_values::domain::query_tokens::types::StepIdentified::Assigned(first),
    );
    program.fragment_contracts.push(syntax::FragmentContract {
        first,
        result: first,
        inputs: vec![syntax::FragmentContractInput {
            name: tokens::WrittenName("rows".into()),
            value: ast::ArgumentValue::Number(ryo_relations::domain::relations::types::Integer(1)),
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
