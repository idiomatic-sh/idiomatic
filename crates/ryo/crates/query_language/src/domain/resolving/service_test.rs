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

fn position_preserves_column_order() {
    let fields = super::service::relation_fields(Relation::Function);
    assert_eq!(
        super::service::position(
            &fields,
            syntax::QuerySpelling("span"),
            lowering_builder().span
        ),
        Ok(parts::ColumnIndex(3))
    );
}
fn missing_column_preserves_name_and_span() {
    let fields = super::service::relation_fields(Relation::Function);
    let span = lowering_builder().span;
    assert_eq!(
        super::service::position(&fields, syntax::QuerySpelling("missing"), span),
        Err(LoweringError::UnknownColumn {
            name: "missing".into(),
            span
        })
    );
}

fn a_relation_the_catalog_lacks_is_refused() {
    let builder = lowering_builder();
    let source = syntax::QuerySource {
        reference: syntax::SourceReference::Relation(syntax::WrittenRelation("nothing".into())),
        alias: syntax::Alias::Unwritten,
        span: builder.span,
    };
    assert!(matches!(
        super::service::read_source(&builder, &source),
        Err(LoweringError::UnknownRelation { .. })
    ));
}
fn a_step_nobody_defined_is_refused() {
    let builder = lowering_builder();
    assert!(matches!(
        super::service::step_named(&builder, &syntax::StepName("missing".into(), None)),
        Err(LoweringError::UnknownStep { .. })
    ));
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("position_preserves_column_order"),
        word: Word::Succeeds,
        input: || position_preserves_column_order,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("missing_column_preserves_name_and_span"),
        word: Word::Succeeds,
        input: || missing_column_preserves_name_and_span,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_relation_the_catalog_lacks_is_refused"),
        word: Word::Succeeds,
        input: || a_relation_the_catalog_lacks_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_step_nobody_defined_is_refused"),
        word: Word::Succeeds,
        input: || a_step_nobody_defined_is_refused,
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
