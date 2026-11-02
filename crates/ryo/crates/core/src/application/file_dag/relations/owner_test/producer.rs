//! A trusted selected publication must come from the actual stored selector algorithm.
use super::{
    RelationOwner,
    fixtures::{columns, read},
};
use crate::domain::{
    operating::types::OperatorContext,
    operator::contract::views::BorrowedRows,
    stored_selection::types::Selector,
    table_model::{
        collections::representations::types::PhysicalShape,
        types::{ParameterBinding, Rows, SelectionIdentity},
    },
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_language::domain::{
    relations::{
        service::kinds,
        types::{ColumnsView, FileId, Integer, Relation, Value},
    },
    symbols::types::Symbols,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Operand, ParameterId, Test, WholeNumber,
};
use std::sync::Arc;

#[test]
fn real_selector_producer_is_the_only_trusted_selection_path() {
    let relation = Relation::FilesystemLineCount;
    let schema = kinds(relation);
    let physical = columns(schema);
    let source = ColumnsView {
        kinds: schema,
        columns: &physical,
    };
    let owner = RelationOwner::new(true);
    let program = Arc::new(Selector {
        relation,
        identity: Arc::new(SelectionIdentity {
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Parameter(ParameterId(0)),
                origin: None,
            }],
            bindings: vec![ParameterBinding::Number(WholeNumber(101))],
        }),
        columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
        distinct: false,
    });
    let mut cache = crate::application::file_dag::derived::indexes::new(
        &[relation],
        &[],
        crate::application::file_dag::derived::indexes::LIMIT,
    )
    .unwrap();
    // Two registered consumers make this population eligible for retained match IDs.
    cache.selectors = vec![Arc::clone(&program), Arc::clone(&program)];
    cache.selection_views = Some(
        crate::application::file_dag::derived::selection::prepare_views_with_schemas(
            cache.selectors.iter().cloned(),
            &|_| schema,
        )
        .unwrap(),
    );
    let (mut symbols, mut memo, mut scratch) =
        (Symbols::default(), Vec::new(), OperatorScratch::default());
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let produced = owner
        .produce_selection(
            &program,
            &|asked| {
                assert_eq!(asked, relation);
                Ok(Rows::Stored(source))
            },
            &mut context,
        )
        .unwrap()
        .expect("real producer");
    let publication = owner.publish_selection(produced).unwrap();
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
        1,
        "trusted output uses the producer's checked population"
    );
    let old_token = owner
        .produce_selection(&program, &|_| Ok(Rows::Stored(source)), &mut context)
        .unwrap()
        .expect("real repeated producer");
    let Rows::Selected(selected) = &resolved else {
        panic!("producer must publish compact rows")
    };
    assert!(
        Arc::ptr_eq(&selected.selection.rows, &old_token.selection.rows),
        "second registered consumer reuses the producer match IDs"
    );
    drop(context);
    assert_eq!(
        cache.selection_counts.builds, 1,
        "two registered consumers build only once"
    );
    assert_eq!(
        cache.selection_counts.reuses, 1,
        "second request actually uses retained match IDs"
    );
    owner.release_population();
    assert!(matches!(
        owner.publish_selection(old_token),
        Err(crate::domain::operating::errors::OperatingError::NotRead)
    ));
}
