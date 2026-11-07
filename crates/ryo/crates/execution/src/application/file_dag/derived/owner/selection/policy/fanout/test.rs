//! Literal fanout routes distinct active demands through one physical scan.
use crate::application::file_dag::derived::owner::selection::fixtures::SelectionFixture;
use crate::domain::coverage::types::{Lacking, Partial};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::stored_selection::fixtures::expected_columns;
use crate::domain::stored_selection::types::Selector;
use crate::domain::table_model::service::materialize;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Relation, Value as Cell,
};
use ryo_relations::domain::terms::service::term_symbol;
use ryo_relations::domain::terms::types::{DeclarationKind, NameText, Symbol};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, PlanComparison, LiteralText, Operand, ParameterId, QueryLiteral, Test,
};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::Arc;
type Failure = OperatingError;

/// Distinct active literal demands share one physical scan, while each selector
/// retains the ordinary text-equality bag, order, and caller-owned error path.
#[test]
fn literal_fanout_routes_aliases_once_and_refuses_incomplete_inputs() -> Result<(), Failure> {
    let mut symbols = Symbols::default();
    let dynamic_class = intern_derived(&mut symbols, ByteSlice(b""), NameText("class"));
    let extras: Vec<_> = (0..7)
        .map(|number| {
            intern_derived(
                &mut symbols,
                ByteSlice(b""),
                NameText(&format!("word{number}")),
            )
        })
        .collect();
    let mut bits = vec![to_bits(Cell::Symbol(term_symbol(DeclarationKind::Method))).0; 128];
    for row in [1, 4, 8] {
        bits[row] = to_bits(Cell::Symbol(term_symbol(DeclarationKind::Class))).0;
    }
    bits[5] = to_bits(Cell::Symbol(dynamic_class)).0;
    bits[127] = to_bits(Cell::Symbol(Symbol(u32::MAX))).0;
    for (number, symbol) in extras.iter().enumerate() {
        bits[20 + number * 2] = to_bits(Cell::Symbol(*symbol)).0;
    }
    let columns = [FactColumn(bits)];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let selector = |text: LiteralText| {
        Arc::new(Selector {
            relation: Relation::Defines,
            identity: Arc::new(crate::domain::table_model::types::SelectionIdentity {
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Literal(QueryLiteral::Text(text)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                bindings: Vec::new(),
            }),
            columns: vec![ColumnIndex(0), ColumnIndex(0)],
            distinct: false,
        })
    };
    let mut selectors = vec![selector(LiteralText("class".into()))];
    selectors.extend((0..7).map(|number| selector(LiteralText(format!("word{number}").into()))));
    let inactive = selector(LiteralText("absent".into()));
    let bad = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let mut error_selector = (*selectors[0]).clone();
    error_selector.identity = Arc::new(crate::domain::table_model::types::SelectionIdentity {
        comparisons: vec![selectors[0].identity.comparisons[0].clone(), bad.clone()],
        bindings: vec![ParameterBinding::Tables(Vec::new())],
    });
    let error_selector = Arc::new(error_selector);
    let mut absent_then_error = (*selectors[0]).clone();
    absent_then_error.identity = Arc::new(crate::domain::table_model::types::SelectionIdentity {
        comparisons: vec![
            selector(LiteralText("absent".into())).identity.comparisons[0].clone(),
            bad,
        ],
        bindings: vec![ParameterBinding::Tables(Vec::new())],
    });
    let absent_then_error = Arc::new(absent_then_error);
    let authored: Vec<_> = selectors
        .iter()
        .cloned()
        .chain([
            inactive.clone(),
            error_selector.clone(),
            absent_then_error.clone(),
        ])
        .collect();
    let fixture = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        selectors.clone(),
        authored.clone(),
        source.kinds,
    )?;
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
    assert!(
        fixture
            .select(&inactive, source, &mut context)?
            .expect("ordinary absent-literal fallback")
            .selection
            .rows
            .is_empty()
    );
    assert_eq!(fixture.snapshot(None).fanout_payload_bytes, 0);
    let mut actual = Vec::new();
    for selector in &selectors {
        let selected = fixture
            .select(selector, source, &mut context)?
            .expect("compact selection");
        let mut table = Table::default();
        materialize(
            &Rows::Selected(selected.rows.clone()),
            &mut table,
            &mut Vec::new(),
        );
        actual.push(table);
    }
    let indexes = fixture.snapshot(None);
    assert_eq!(indexes.fanouts, 1);
    assert_eq!(
        indexes.retained_positions,
        indexes.retained_positions_capacity
    );
    assert_eq!(indexes.counts.scanned, 128, "one source pass");
    assert_eq!(indexes.entries, 0, "no whole-column index");
    assert_eq!(actual[0].count.0, 4);
    assert!(matches!(
        fixture.select(&error_selector, source, &mut context),
        Err(OperatingError::WrongBinding)
    ));
    assert!(
        fixture
            .select(&absent_then_error, source, &mut context)?
            .expect("empty complete")
            .selection
            .rows
            .is_empty()
    );
    drop(context);
    for (number, (selector, result)) in selectors.iter().zip(actual).enumerate() {
        let positions = if number == 0 {
            vec![1, 4, 5, 8]
        } else {
            vec![20 + (number - 1) as u32 * 2]
        };
        let expected = expected_columns(source, &selector.columns, &positions);
        assert_eq!(result, expected);
    }
    let constrained = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        selectors.clone(),
        authored.clone(),
        source.kinds,
    )?;
    constrained.limit_to_metadata();
    let mut budget_context = OperatorContext {
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
    assert_eq!(
        constrained
            .select(&selectors[0], source, &mut budget_context)?
            .expect("fallback")
            .selection
            .rows
            .len(),
        4
    );
    assert_eq!(constrained.snapshot(None).fanouts, 0);
    drop(budget_context);
    let empty_columns = [FactColumn(Vec::new())];
    let empty = ColumnsView {
        columns: &empty_columns,
        ..source
    };
    let complete_empty = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        selectors.clone(),
        authored.clone(),
        source.kinds,
    )?;
    let mut empty_context = OperatorContext {
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
    assert!(
        complete_empty
            .select(&selectors[0], empty, &mut empty_context)?
            .expect("complete empty")
            .selection
            .rows
            .is_empty()
    );
    assert_eq!(complete_empty.snapshot(None).fanouts, 0);
    assert!(
        matches!(
            complete_empty.select(
                &selectors[0],
                ColumnsView {
                    columns: &[],
                    ..source
                },
                &mut empty_context,
            ),
            Err(OperatingError::NotRead)
        ),
        "one immutable population cannot be rebound to a different physical backing"
    );
    let malformed = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        selectors.clone(),
        authored.clone(),
        source.kinds,
    )?;
    assert!(
        matches!(
            malformed.select(
                &selectors[0],
                ColumnsView {
                    columns: &[],
                    ..source
                },
                &mut empty_context,
            ),
            Err(OperatingError::ShapeMismatch)
        ),
        "a fresh owner inspects and refuses mismatched schema width"
    );
    assert_eq!(malformed.owner().checks.get(), crate::application::file_dag::relations::types::CheckCount(1));
    drop(empty_context);
    let partial = Partial {
        relation: Relation::Defines,
        cause: PartialCause("missing syntax".into()),
        lacking: Lacking::Any,
        at: None,
    };
    let incomplete = SelectionFixture::new(
        &[Relation::Defines],
        &[partial],
        selectors.clone(),
        authored,
        source.kinds,
    )?;
    let mut partial_context = OperatorContext {
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
    assert!(
        incomplete
            .select(&selectors[0], source, &mut partial_context)?
            .is_none()
    );
    assert_eq!(incomplete.snapshot(None).fanouts, 0);
    Ok(())
}
