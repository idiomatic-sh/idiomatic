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

fn column(alias: &str, name: &str) -> ast::ColumnReference {
    ast::ColumnReference {
        alias: ast::Alias::Written(tokens::AliasName(alias.into())),
        column: syntax::ColumnName(name.into()),
    }
}
fn source(alias: &str) -> ast::QuerySource {
    ast::QuerySource {
        reference: ast::SourceReference::Relation(tokens::WrittenRelation("function".into())),
        alias: ast::Alias::Written(tokens::AliasName(alias.into())),
        span: lowering_builder().span,
    }
}
fn lower(key: &str) -> Result<plans::Operation, LoweringError> {
    let mut builder = lowering_builder();
    let join = ast::Join {
        source: source("right"),
        on: vec![ast::JoinKey {
            left: column("left", key),
            right: column("right", key),
        }],
    };
    let select = ast::Select {
        columns: ast::Columns::Listed(vec![ast::SelectColumn {
            column: column("left", "file"),
            rename: ast::Renaming::Unchanged,
        }]),
        from: source("left"),
        join: ast::Joining::Joined(join.clone()),
        filters: vec![],
    };
    let input = crate::domain::resolving::service::read_source(&mut builder, &select.from)?;
    super::service::select_join(
        &mut builder,
        &tokens::StepName(
            "result".into(),
            ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
        ),
        &select,
        &join,
        input,
        crate::domain::lowering::types::LastStep::Join,
    )?;
    Ok(builder.steps.remove(0).operation)
}
fn typed_join_keeps_keys_and_selected_side() {
    let plans::Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        filter,
        view,
        reach,
    } = lower("file").unwrap()
    else {
        panic!("join operation")
    };
    assert_eq!(left, parts::Input::Relation(Relation::Function));
    assert_eq!(right, left);
    assert_eq!(left_on, vec![parts::ColumnIndex(2)]);
    assert_eq!(right_on, left_on);
    assert_eq!(
        output[0],
        parts::JoinColumn {
            side: parts::Side::Left,
            column: parts::ColumnIndex(2)
        }
    );
    assert!(filter.is_empty() && view.is_empty() && reach.is_none());
}
fn unknown_join_key_keeps_name_and_span() {
    assert_eq!(
        lower("missing"),
        Err(LoweringError::UnknownColumn {
            name: "missing".into(),
            span: lowering_builder().span
        })
    );
}

/// The refusal of a join keyed by `key`, whose select keeps the filters it is given.
fn refusal_of(key: ast::JoinKey, filters: Vec<ast::Condition>) -> LoweringError {
    let mut builder = lowering_builder();
    let join = ast::Join {
        source: source("right"),
        on: vec![key],
    };
    let select = ast::Select {
        columns: ast::Columns::Listed(vec![ast::SelectColumn {
            column: column("left", "file"),
            rename: ast::Renaming::Unchanged,
        }]),
        from: source("left"),
        join: ast::Joining::Joined(join.clone()),
        filters,
    };
    let input = crate::domain::resolving::service::read_source(&mut builder, &select.from)
        .expect("the source is read");
    super::service::select_join(
        &mut builder,
        &tokens::StepName(
            "result".into(),
            ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
        ),
        &select,
        &join,
        input,
        crate::domain::lowering::types::LastStep::Join,
    )
    .expect_err("the join is refused")
}
fn a_key_naming_neither_side_is_refused() {
    let key = ast::JoinKey {
        left: column("elsewhere", "file"),
        right: column("right", "file"),
    };
    assert!(matches!(
        refusal_of(key, vec![]),
        LoweringError::UnknownSide { .. }
    ));
}
fn a_where_naming_a_column_its_side_lacks_is_refused() {
    let key = ast::JoinKey {
        left: column("left", "file"),
        right: column("right", "file"),
    };
    let filter = ast::Filter {
        left: ast::WrittenOperand::Column(column("left", "missing")),
        test: ast::WrittenTest::Equal,
        right: ast::WrittenOperand::Text(tokens::WrittenLiteral("x".into())),
        span: lowering_builder().span,
    };
    assert!(matches!(
        refusal_of(key, vec![ast::Condition::Test(filter)]),
        LoweringError::UnknownSideColumn { .. }
    ));
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("typed_join_keeps_keys_and_selected_side"),
        word: Word::Succeeds,
        input: || typed_join_keeps_keys_and_selected_side,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unknown_join_key_keeps_name_and_span"),
        word: Word::Succeeds,
        input: || unknown_join_key_keeps_name_and_span,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_key_naming_neither_side_is_refused"),
        word: Word::Succeeds,
        input: || a_key_naming_neither_side_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_where_naming_a_column_its_side_lacks_is_refused"),
        word: Word::Succeeds,
        input: || a_where_naming_a_column_its_side_lacks_is_refused,
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
