//! Empty retention charges no position slots; request inventory allocates only active groups.
use crate::application::file_dag::derived::owner::selection::fixtures::SelectionFixture;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::stored_selection::fixtures::program;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell,
};
use ryo_values::domain::files::types::SourceView;
type Failure = OperatingError;

#[test]
fn empty_retention_counts_no_position_slots_and_invalid_group_is_not_allocation_failure()
-> Result<(), Failure> {
    let selector = program(2);
    let active = vec![selector.clone(), selector.clone()];
    let fixture = SelectionFixture::new(
        &[Relation::Function],
        &[],
        active.clone(),
        [selector.clone()],
        &[CellKind::Number; 3],
    )?;
    let columns = [
        FactColumn::default(),
        FactColumn::default(),
        FactColumn::default(),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
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
    let before = fixture.snapshot(None);
    let selected = fixture
        .select(&selector, source, &mut context)?
        .expect("complete empty result");
    assert!(selected.selection.rows.is_empty());
    let after = fixture.snapshot(None);
    assert_eq!(after.selections, 1);
    assert_eq!(after.retained_positions, 0);
    assert!(after.bytes > before.bytes && after.bytes <= after.limit);
    let fresh = SelectionFixture::new(
        &[Relation::Function],
        &[],
        active,
        [selector.clone()],
        source.kinds,
    )?;
    assert_eq!(fresh.snapshot(None).retained_positions, 0);
    fresh.corrupt_prepared_group_count();
    assert!(matches!(
        fresh.select(&selector, source, &mut context),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(
        fresh.snapshot(None).counts.refusals,
        0,
        "invalid preparation is not an allocation or payload-budget refusal"
    );
    Ok(())
}

#[test]
fn request_selector_inventory_allocates_repetition_only_for_active_groups() -> Result<(), Failure> {
    let authored = (0..1024).map(program).collect::<Vec<_>>();
    let active = authored[2].clone();
    let known_inactive = authored[3].clone();
    let fixture = SelectionFixture::new(
        &[Relation::Function],
        &[],
        vec![active.clone(), active.clone()],
        authored,
        &[CellKind::Number; 3],
    )?;
    let snapshot = fixture.snapshot(None);
    assert_eq!(snapshot.prepared_selectors, 1024);
    assert_eq!(snapshot.active_repetition_groups, 1);
    assert!(
        snapshot.repetition_capacity < 256,
        "inactive request groups allocate no file counter slots"
    );
    let columns = (0..3)
        .map(|column| {
            FactColumn(
                (0..4)
                    .map(|row| to_bits(Cell::Number(Integer(row + column))).0)
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
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
    let selected = fixture
        .select(&active, source, &mut context)?
        .expect("active prepared selector");
    assert_eq!(selected.selection.rows.as_slice(), &[2]);
    let inactive = fixture
        .select(&known_inactive, source, &mut context)?
        .expect("known descriptor remains legal without a repetition slot");
    assert_eq!(inactive.selection.rows.as_slice(), &[3]);
    assert_eq!(fixture.snapshot(None).active_repetition_groups, 1);
    Ok(())
}
