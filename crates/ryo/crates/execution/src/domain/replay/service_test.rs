//! The replay of a selection class result: the accepted prefix first, then the terminal error.

use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    positions::types::ClassPlace,
    shared_walk::{
        service::evaluate,
        service_test::{relation_plan, with_context},
        types::{WalkWork, prepared_walk::value::PreparedWalk},
    },
    table_model::types::Rows,
};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::{
    relations::types::{FileId, NodeId, Offset, Relation, Span, Value as CellValue},
    terms::types::NameText,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, LiteralText, Operand, PlanComparison, QueryLiteral, Test},
    rows::types::RowCount,
};

#[derive(Default)]
struct Collected(Vec<Vec<CellValue>>);
impl crate::domain::operating::types::Emit for Collected {
    fn row(
        &mut self,
        _: &mut OperatorContext<'_>,
        row: &[CellValue],
    ) -> Result<(), OperatingError> {
        self.0.push(row.to_vec());
        Ok(())
    }
}

#[test]
fn generic_late_error_replays_exact_accepted_prefix_and_sink_refusal() {
    let comparison = PlanComparison {
        test: Test::Matches,
        left: Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
        right: Operand::Column(ColumnIndex(2)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let plans = [relation_plan(Relation::SemanticSyntax, vec![comparison])];
    let prepared = PreparedWalk::compile(&plans, &[vec![]]);
    let mut symbols = Symbols::default();
    let valid = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ByteSlice(b""),
        NameText("x"),
    );
    let invalid = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ByteSlice(b""),
        NameText("["),
    );
    let table = ryo_values::domain::rows::types::Table {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::SemanticSyntax).to_vec(),
        cells: [valid, invalid, valid]
            .iter()
            .enumerate()
            .flat_map(|(at, text)| {
                [
                    CellValue::Node(NodeId(at as u64)),
                    CellValue::Symbol(valid),
                    CellValue::Symbol(*text),
                    CellValue::File(FileId(0)),
                    CellValue::Span(Span {
                        start: Offset(0),
                        end: Offset(1),
                    }),
                ]
            })
            .collect(),
        count: RowCount(3),
    };
    let source = Rows::Computed(&table);
    with_context(&mut symbols, |context| {
        let batch = evaluate(
            &prepared,
            Relation::SemanticSyntax,
            &source,
            &[Vec::new().into_boxed_slice()],
            context,
            &[ClassPlace(0)],
            &mut WalkWork::default(),
        );
        let result = batch.class(ClassPlace(0)).unwrap();
        assert_eq!(result.positions.0, [0]);
        assert_eq!(batch.visits(ClassPlace(0)).0, 2);
        assert!(matches!(
            &result.completion,
            Err(OperatingError::InvalidRegex { .. })
        ));
        let mut collected = Collected::default();
        let replayed = crate::domain::replay::service::selected(
            result,
            &source,
            crate::domain::replay::types::SelectionColumns::Columns(&[
                ColumnIndex(2),
                ColumnIndex(0),
                ColumnIndex(2),
            ]),
            context,
            &mut collected,
        );
        assert!(matches!(replayed, Err(OperatingError::InvalidRegex { .. })));
        assert_eq!(
            collected.0,
            vec![vec![
                CellValue::Symbol(valid),
                CellValue::Node(NodeId(0)),
                CellValue::Symbol(valid)
            ]]
        );
        struct Refuse;
        impl crate::domain::operating::types::Emit for Refuse {
            fn row(
                &mut self,
                _: &mut OperatorContext<'_>,
                _: &[CellValue],
            ) -> Result<(), OperatingError> {
                Err(OperatingError::NotRead)
            }
        }
        assert!(matches!(
            crate::domain::replay::service::selected(
                result,
                &source,
                crate::domain::replay::types::SelectionColumns::All,
                context,
                &mut Refuse
            ),
            Err(OperatingError::NotRead)
        ));
    });
}
