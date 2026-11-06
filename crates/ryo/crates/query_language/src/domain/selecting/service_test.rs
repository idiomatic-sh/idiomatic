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
                column: syntax::ColumnName(column.into()),
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

fn projection_is_typed_and_file_scoped() {
    let mut builder = lowering_builder();
    super::service::lower_select(
        &mut builder,
        &tokens::StepName(
            "result".into(),
            ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
        ),
        &select("file"),
    )
    .unwrap();
    assert_eq!(
        builder.steps,
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
fn invalid_projection_keeps_exact_column_error() {
    let mut builder = lowering_builder();
    let span = builder.span;
    assert_eq!(
        super::service::lower_select(
            &mut builder,
            &tokens::StepName(
                "result".into(),
                ryo_values::domain::query_tokens::types::StepIdentified::Unassigned
            ),
            &select("missing")
        ),
        Err(LoweringError::UnknownColumn {
            name: "missing".into(),
            span
        })
    );
    assert!(builder.steps.is_empty());
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("projection_is_typed_and_file_scoped"),
        word: Word::Succeeds,
        input: || projection_is_typed_and_file_scoped,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_projection_keeps_exact_column_error"),
        word: Word::Succeeds,
        input: || invalid_projection_keeps_exact_column_error,
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
