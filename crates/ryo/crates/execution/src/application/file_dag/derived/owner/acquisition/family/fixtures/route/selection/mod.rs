//! The selection entry: predicate, candidate and distinct selectors acquired through the owner.
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::route::exit::{
    Exit, released, released_at_exit,
};
use crate::application::file_dag::derived::owner::selection::fixtures::SelectionFixture;
use crate::domain::{
    operating::{
        errors::OperatingError,
        types::{AfterTheFiles, OperatorContext},
    },
    stored_selection::{
        fixtures::program,
        types::{SelectionIdentity, Selector},
    },
    table_scratch::types::OperatorScratch,
};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::{
        service::to_bits,
        types::{CellKind, ColumnsView, FactColumn, Relation, Value},
    },
    terms::{
        service::term_symbol,
        types::{DeclarationKind, NameText, Symbol},
    },
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{
        ColumnIndex, PlanComparison, LiteralText, Operand, ParameterId, QueryLiteral, Test,
    },
    query_bindings::types::ParameterBinding,
};
use std::sync::Arc;

/// A fixed set of integer rows for the distinct and predicate selectors.
const DISTINCT: [&[u64]; 3] = [
    &[7, 7, 7, 7, 7, 8],
    &[1, 2, 0, 7, 0, 1],
    &[10, 10, 20, 11, 20, 10],
];
pub(super) fn selection(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    if kind == DerivedKind::CandidatePostings {
        return fanout(exit);
    }
    let columns = DISTINCT.map(|column| FactColumn(column.to_vec()));
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
        columns: &columns,
    };
    let mut distinct = (*program(7)).clone();
    distinct.columns = vec![ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)];
    distinct.distinct = ryo_values::domain::plan_parts::types::RepeatedRows::Dropped;
    let distinct = Arc::new(distinct);
    let mut bag = (*distinct).clone();
    bag.distinct = ryo_values::domain::plan_parts::types::RepeatedRows::Kept;
    let bag = Arc::new(bag);
    let chosen = if kind == DerivedKind::DistinctRepresentatives {
        &distinct
    } else {
        &bag
    };
    run_with(
        Selected {
            kind,
            relation: Relation::Function,
            active: &[Arc::clone(&distinct)],
            authored: vec![Arc::clone(&distinct), Arc::clone(&bag)],
            source,
            kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
            chosen,
            fanned: false,
        },
        exit,
        &mut Symbols::default(),
    )
}
/// The text-equality selectors of one relation share a single fanout scan.
fn fanout(exit: Exit) -> Result<DerivedKind, OperatingError> {
    let mut symbols = Symbols::default();
    let mut bits = vec![to_bits(Value::Symbol(term_symbol(DeclarationKind::Method))).0; 128];
    for row in [1, 4, 8] {
        bits[row] = to_bits(Value::Symbol(term_symbol(DeclarationKind::Class))).0;
    }
    bits[127] = to_bits(Value::Symbol(Symbol(u32::MAX))).0;
    for number in 0..7 {
        let word = intern_derived(
            &mut symbols,
            ByteSlice(b""),
            NameText(&format!("word{number}")),
        );
        bits[20 + number * 2] = to_bits(Value::Symbol(word)).0;
    }
    let columns = [FactColumn(bits)];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let selector = |text: String| {
        Arc::new(Selector {
            relation: Relation::Defines,
            identity: Arc::new(SelectionIdentity {
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                bindings: Vec::new(),
            }),
            columns: vec![ColumnIndex(0), ColumnIndex(0)],
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
        })
    };
    let mut selectors = vec![selector("class".into())];
    selectors.extend((0..7).map(|number| selector(format!("word{number}"))));
    let chosen = Arc::clone(&selectors[0]);
    run_with(
        Selected {
            kind: DerivedKind::CandidatePostings,
            relation: Relation::Defines,
            active: &selectors,
            authored: selectors.clone(),
            source,
            kinds: &[CellKind::Symbol],
            chosen: &chosen,
            fanned: true,
        },
        exit,
        &mut symbols,
    )
}
/// A selector whose parameter is bound to a table: the comparison refuses with `WrongBinding`.
fn refusing(base: &Selector) -> Arc<Selector> {
    let mut bad = base.clone();
    let bind = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    bad.identity = Arc::new(SelectionIdentity {
        comparisons: vec![base.identity.comparisons[0].clone(), bind],
        bindings: vec![ParameterBinding::Tables(Vec::new())],
    });
    Arc::new(bad)
}
/// One selection demand: the selectors the request prepared and the one the consumer selects.
struct Selected<'a> {
    kind: DerivedKind,
    relation: Relation,
    active: &'a [Arc<Selector>],
    authored: Vec<Arc<Selector>>,
    source: ColumnsView<'a>,
    kinds: &'static [CellKind],
    chosen: &'a Arc<Selector>,
    fanned: bool,
}
fn run_with(
    Selected {
        kind,
        relation,
        active,
        mut authored,
        source,
        kinds,
        chosen,
        fanned,
    }: Selected<'_>,
    exit: Exit,
    symbols: &mut Symbols,
) -> Result<DerivedKind, OperatingError> {
    let error = refusing(chosen);
    authored.push(Arc::clone(&error));
    let fixture = SelectionFixture::new(&[relation], &[], active.to_vec(), authored, kinds)?;
    let floor = fixture.source_floor(relation, source)?;
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let result = {
        let mut context = OperatorContext {
            phase: AfterTheFiles::Before,
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            matcher: crate::domain::matching::types::Matcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
            bindings: &[],
            symbols: (&mut *symbols).into(),
            source: SourceView(b""),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        match exit {
            Exit::Cancelled => {
                // The demand is acquired, then withdrawn without its rows being read.
                let withdrawn = fixture.select(chosen, source, &mut context)?;
                assert!(withdrawn.is_some(), "{kind:?}: the selection is published");
                Ok(kind)
            }
            Exit::Succeeds => {
                let mut every = active.to_vec();
                if !fanned {
                    every = vec![Arc::clone(chosen)];
                }
                for selector in &every {
                    assert!(
                        fixture.select(selector, source, &mut context)?.is_some(),
                        "{kind:?}: the selection is published"
                    );
                }
                if fanned {
                    assert_eq!(
                        fixture.snapshot(None).fanouts,
                        1,
                        "{kind:?}: the aliases share one physical scan"
                    );
                }
                Ok(kind)
            }
            Exit::Errors => fixture.select(&error, source, &mut context).map(|_| kind),
        }
    };
    // The consumer's own state (the operator scratch and the memo it filled) ends with the exit.
    drop(scratch);
    drop(memo);
    released_at_exit(
        kind,
        exit,
        fixture.owner().derived().publication_ownership_bytes(),
        floor,
    );
    fixture.owner().release_population();
    released(
        kind,
        exit,
        fixture.owner().derived().publication_ownership_bytes(),
    );
    result
}
