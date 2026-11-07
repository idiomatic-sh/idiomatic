//! A trusted selected publication must come from the actual stored selector algorithm.
use super::{
    RelationOwner,
    fixtures::{columns, read},
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{ColumnsView, FileId, Integer, Relation, Value},
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Operand, ParameterId, PlanComparison, Test, WholeNumber,
};
use std::sync::Arc;
use {
    crate::domain::operating::types::OperatorContext,
    crate::domain::operator::contract::views::BorrowedRows,
    crate::domain::stored_selection::types::Selector,
    crate::domain::table_model::collections::representations::types::PhysicalShape,
    crate::domain::table_model::types::Rows, crate::domain::table_model::types::SelectionIdentity,
    crate::domain::table_scratch::types::OperatorScratch,
    ryo_values::domain::files::types::SourceView,
};

#[test]
fn real_selector_producer_is_the_only_trusted_selection_path() {
    for limit in [Retention::Refused, Retention::Default] {
        exercise_real_selector(limit);
    }
}
enum Retention {
    Refused,
    Default,
}
fn exercise_real_selector(limit: Retention) {
    let relation = Relation::FilesystemLineCount;
    let schema = kinds(relation);
    let physical = columns(schema);
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let program = Arc::new(Selector {
        relation,
        identity: Arc::new(SelectionIdentity {
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Parameter(ParameterId(0)),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            }],
            bindings: vec![ParameterBinding::Number(WholeNumber(101))],
        }),
        columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
        distinct: false,
    });
    let active = vec![Arc::clone(&program), Arc::clone(&program)];
    let fixture =
        crate::application::file_dag::derived::owner::selection::fixtures::SelectionFixture::new(
            &[relation],
            &[],
            active.clone(),
            active,
            schema,
        )
        .unwrap();
    let owner = fixture.owner();
    if matches!(limit, Retention::Refused) {
        owner
            .derived
            .set_optional_limit(
                crate::application::file_dag::derived::owner::index::types::ByteLimit(0),
            )
            .unwrap();
    }
    let (mut symbols, mut memo, mut scratch) =
        (Symbols::default(), Vec::new(), OperatorScratch::default());
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let population = owner.source(relation, &Rows::Stored(source)).unwrap();
    let proof = owner
        .admit(population.as_ref(), BorrowedRows::Stored(source))
        .unwrap();
    let publication = owner
        .derived
        .select(&program, source, &owner, &population, &proof, &mut context)
        .unwrap()
        .expect("real producer");
    let resolved = owner
        .resolve(publication.as_ref(), &|_| Ok(Rows::Stored(source)))
        .unwrap();
    assert_eq!(
        read(&resolved),
        [vec![
            Value::Number(Integer(101)),
            Value::File(FileId(9)),
            Value::Number(Integer(101))
        ]]
    );
    assert_eq!(
        owner.inspect(publication.as_ref(), BorrowedRows::from(&resolved)),
        Ok(PhysicalShape::Complete)
    );
    assert_eq!(
        owner.checks.get(),
        crate::application::file_dag::relations::types::CheckCount(1),
        "trusted output uses the producer's checked population"
    );
    let old_token = owner
        .derived
        .select(&program, source, &owner, &population, &proof, &mut context)
        .unwrap()
        .expect("real repeated producer");
    let repeated = owner
        .resolve(old_token.as_ref(), &|_| Ok(Rows::Stored(source)))
        .unwrap();
    let Rows::Selected(second_selected) = &repeated else {
        panic!("actual repeated selected publication");
    };
    let Rows::Selected(selected) = &resolved else {
        panic!("producer must publish compact rows")
    };
    assert!(
        Arc::ptr_eq(&selected.selection.rows, &second_selected.selection.rows),
        "second registered consumer reuses the producer match IDs"
    );
    assert_eq!(
        fixture.snapshot(None).counts.builds,
        1,
        "two registered consumers build only once"
    );
    assert_eq!(
        fixture.snapshot(None).counts.reuses,
        1,
        "second request actually uses retained match IDs"
    );
    owner.release_population();
    assert!(matches!(
        owner
            .derived
            .select(&program, source, owner, &population, &proof, &mut context),
        Err(crate::domain::operating::errors::OperatingError::NotRead)
    ));
}

use ryo_values::domain::query_bindings::types::ParameterBinding;
