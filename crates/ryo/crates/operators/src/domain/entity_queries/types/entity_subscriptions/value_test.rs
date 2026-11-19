//! A second reader constructs a field index and every lookup preserves provider order and empty buckets.
use super::super::entity_columns::value::EntityFileColumns;
use super::super::entity_plan::value::QueryPlan;
use super::super::{
    AncestorDistance, EntityAttribute, EntityColumn, EntityExport, EntityExportId, EntityQueryOp,
    EntityStage,
};
use super::value::EntitySubscriptions;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::entity_queries::service::literal;
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    constants::NO_SYMBOL,
    service::kinds,
    types::{ColumnsView, FactColumn, Relation},
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{ColumnIndex, LiteralText, Side, StepId, Test},
};

type Input = StepId;
type Value = (usize, [Option<u32>; 3]);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_shared_empty_spelling_keeps_all_source_positions_in_provider_order"),
        word: Word::Succeeds,
        input: || StepId(1),
        expect: Expect::Value((3, [Some(2), Some(0), Some(1)])),
    },
    Case {
        name: CaseName("an_absent_spelling_answers_a_known_empty_ordered_set"),
        word: Word::Succeeds,
        input: || StepId(2),
        expect: Expect::Value((0, [None; 3])),
    },
];
fn subject(query: Input) -> Result<Value, Failure> {
    let fields = vec![
        EntityColumn(EntityAttribute::Id, AncestorDistance(0)),
        EntityColumn(EntityAttribute::Kind, AncestorDistance(0)),
    ];
    let select = |id, word: &str| EntityStage {
        expr: EntityQueryOp::Filter {
            tests: vec![literal(
                ColumnIndex(1),
                Test::Equal,
                &LiteralText(word.into()),
            )],
        },
        fields: fields.clone(),
        operands: vec![StepId(0)],
        physical: Some(EntityExportId(id)),
    };
    let plan = QueryPlan::from_stages(
        vec![
            EntityStage {
                expr: EntityQueryOp::Syntax,
                fields: fields.clone(),
                operands: vec![],
                physical: None,
            },
            select(1, ""),
            select(2, "absent"),
        ],
        vec![
            EntityExport(EntityExportId(1), StepId(1)),
            EntityExport(EntityExportId(2), StepId(2)),
        ],
    );
    let symbols = Symbols::default();
    let texts = Texts {
        symbols: &symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    let missing = u64::from(NO_SYMBOL.0);
    let columns = [
        vec![2, 0, 1],
        vec![missing; 3],
        vec![missing; 3],
        vec![0; 3],
        vec![0; 3],
    ]
    .map(FactColumn);
    let edges = std::array::from_fn::<_, 5, _>(|_| FactColumn(vec![]));
    let source = ColumnsView {
        kinds: kinds(Relation::SemanticSyntax),
        columns: &columns,
    };
    let children = ColumnsView {
        kinds: kinds(Relation::SemanticChild),
        columns: &edges,
    };
    let file = EntityFileColumns::open(&plan, source, children, &texts).ok_or(())?;
    let subscription = plan.partitions.as_ref().ok_or(())?;
    let rows = file.source(Side::Left);
    let mut index = EntitySubscriptions::default();
    if index
        .selected(subscription, StepId(1), &rows, &fields, &file)
        .is_some()
    {
        return Err(());
    }
    let got = index
        .selected(subscription, query, &rows, &fields, &file)
        .ok_or(())?;
    Ok((got.len(), std::array::from_fn(|i| got.get(i).map(|p| p.0))))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject)
}
