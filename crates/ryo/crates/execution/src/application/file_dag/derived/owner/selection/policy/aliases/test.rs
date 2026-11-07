//! Symbol spellings: term aliases, absent spellings and malformed symbols keep ordinary text equality.
use crate::application::file_dag::derived::owner::selection::fixtures::SelectionFixture;
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
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell,
};
use ryo_relations::domain::terms::service::term_symbol;
use ryo_relations::domain::terms::types::{DeclarationKind, NameText, Symbol};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, PlanComparison, LiteralText, Operand, ParameterId, QueryLiteral, Test,
};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;
type Failure = OperatingError;

/// A closed term and a file-interned name can spell the same text. An equality
/// index over raw Symbol IDs must decline that ambiguous discriminator.
#[test]
fn symbol_discriminator_preserves_text_equality_with_term_aliases() -> Result<(), Failure> {
    let mut symbols = Symbols::default();
    let dynamic_class = intern_derived(&mut symbols, ByteSlice(b""), NameText("class"));
    let rare = intern_derived(&mut symbols, ByteSlice(b""), NameText("rare"));
    let class_term = term_symbol(DeclarationKind::Class);
    let method_term = term_symbol(DeclarationKind::Method);
    assert_ne!(dynamic_class, class_term);
    let mut kind = vec![to_bits(Cell::Symbol(method_term)).0; 128];
    for row in [0, 3, 17] {
        kind[row] = to_bits(Cell::Symbol(class_term)).0;
    }
    kind[2] = to_bits(Cell::Symbol(dynamic_class)).0;
    kind[4] = to_bits(Cell::Symbol(rare)).0;
    let columns = [
        FactColumn(kind),
        FactColumn(
            (0..128)
                .map(|row| to_bits(Cell::Number(Integer(row % 2))).0)
                .collect(),
        ),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol, CellKind::Number],
        columns: &columns,
    };
    let selector = |word: LiteralText| {
        Arc::new(Selector {
            relation: Relation::Defines,
            identity: Arc::new(crate::domain::stored_selection::types::SelectionIdentity {
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Literal(QueryLiteral::Text(word)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                bindings: Vec::new(),
            }),
            columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
        })
    };
    let rare_selector = selector(LiteralText("rare".into()));
    let class_selector = selector(LiteralText("class".into()));
    let active = vec![rare_selector.clone(), class_selector.clone()];
    let fixture = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        active.clone(),
        active,
        source.kinds,
    )?;
    assert!(fixture.snapshot(None).repeated_discriminator);
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: crate::domain::matching::types::Matcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let rare_rows = fixture
        .select(&rare_selector, source, &mut context)?
        .expect("indexed rare");
    assert_eq!(**rare_rows.selection.rows, [4]);
    let indexes = fixture.snapshot(None);
    assert_eq!(indexes.entries, 1, "unique name uses the index");
    let candidates_before = indexes.counts.candidates;
    let scans_before = indexes.counts.scanned;
    let class_rows = fixture
        .select(&class_selector, source, &mut context)?
        .expect("term alias scan");
    assert_eq!(**class_rows.selection.rows, [0, 2, 3, 17]);
    let indexes = fixture.snapshot(None);
    assert_eq!(indexes.entries, 1);
    assert_eq!(indexes.counts.candidates, candidates_before);
    assert_eq!(indexes.counts.scanned - scans_before, 128);
    let mut actual = Table::default();
    materialize(
        &Rows::Selected(class_rows.rows.clone()),
        &mut actual,
        &mut Vec::new(),
    );
    drop(context);
    let expected = expected_columns(source, &class_selector.columns, &[0, 2, 3, 17]);
    assert_eq!(actual, expected);
    assert_eq!(actual.count.0, 4);
    assert_eq!(
        &actual.cells[6..9],
        &actual.cells[9..12],
        "duplicate bag rows"
    );
    Ok(())
}

/// A spelling absent from both symbol registries cannot match a nonempty
/// literal, but a preceding comparison can still fail before that equality.
#[test]
fn absent_symbol_literal_preserves_ordered_errors() -> Result<(), Failure> {
    let mut symbols = Symbols::default();
    let present = intern_derived(&mut symbols, ByteSlice(b""), NameText("present"));
    let columns = [FactColumn(vec![to_bits(Cell::Symbol(present)).0; 64])];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let absent = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("absent".into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let bad = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let selector = |comparisons| {
        Arc::new(Selector {
            relation: Relation::Defines,
            identity: Arc::new(crate::domain::stored_selection::types::SelectionIdentity {
                comparisons,
                bindings: vec![ParameterBinding::Tables(Vec::new())],
            }),
            columns: vec![ColumnIndex(0), ColumnIndex(0)],
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
        })
    };
    let first = selector(vec![absent.clone(), bad.clone()]);
    let reversed = selector(vec![bad, absent]);
    let active = vec![first.clone(), reversed.clone()];
    let fixture = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        active.clone(),
        active,
        source.kinds,
    )?;
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: crate::domain::matching::types::Matcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let selected = fixture
        .select(&first, source, &mut context)?
        .expect("compact result");
    assert!(selected.selection.rows.is_empty());
    let mut actual = Table::default();
    materialize(
        &Rows::Selected(selected.rows.clone()),
        &mut actual,
        &mut Vec::new(),
    );
    assert_eq!(fixture.snapshot(None).counts.scanned, 0);
    assert_eq!(fixture.snapshot(None).entries, 0);
    assert!(matches!(
        fixture.select(&reversed, source, &mut context),
        Err(OperatingError::WrongBinding)
    ));
    drop(context);
    // The generic selection policy preserves authored predicate order. Test that
    // pure comparison boundary directly: public preparation separately rejects
    // Tables where a scalar/list predicate binding is required.
    let mut predicate_context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: crate::domain::matching::types::Matcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
        bindings: &first.identity.bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let row = [Cell::Symbol(present)];
    assert_eq!(
        crate::domain::comparing::service::holds_all(
            &first.identity.comparisons,
            &row,
            &[CellKind::Symbol],
            crate::domain::operating::types::NameCount(0),
            &mut predicate_context
        )?,
        crate::domain::table_model::types::Verdict::Fails
    );
    assert!(matches!(
        crate::domain::comparing::service::holds_all(
            &reversed.identity.comparisons,
            &row,
            &[CellKind::Symbol],
            crate::domain::operating::types::NameCount(0),
            &mut predicate_context
        ),
        Err(OperatingError::WrongBinding)
    ));
    assert_eq!(actual.count.0, 0);
    Ok(())
}

/// Invalid Symbol IDs can decode as empty text in ordinary evaluation. They
/// must not be ruled out by an absent empty literal.
#[test]
fn empty_literal_preserves_malformed_symbol_fallback() -> Result<(), Failure> {
    let mut symbols = Symbols::default();
    let invalid = Symbol(u32::MAX);
    let columns = [FactColumn(vec![to_bits(Cell::Symbol(invalid)).0; 3])];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let selector = Arc::new(Selector {
        relation: Relation::Defines,
        identity: Arc::new(crate::domain::stored_selection::types::SelectionIdentity {
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("".into()))),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            }],
            bindings: Vec::new(),
        }),
        columns: vec![ColumnIndex(0), ColumnIndex(0)],
        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
    });
    let fixture = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        vec![selector.clone()],
        [selector.clone()],
        source.kinds,
    )?;
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: crate::domain::matching::types::Matcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let selected = fixture
        .select(&selector, source, &mut context)?
        .expect("compact result");
    assert_eq!(**selected.selection.rows, [0, 1, 2]);
    assert_eq!(fixture.snapshot(None).counts.scanned, 3);
    let mut actual = Table::default();
    materialize(
        &Rows::Selected(selected.rows.clone()),
        &mut actual,
        &mut Vec::new(),
    );
    drop(context);
    let expected = expected_columns(source, &selector.columns, &[0, 1, 2]);
    assert_eq!(actual, expected);
    assert_eq!(actual.count.0, 3);
    Ok(())
}
