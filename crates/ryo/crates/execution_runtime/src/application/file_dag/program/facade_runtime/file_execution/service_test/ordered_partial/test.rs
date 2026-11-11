//! A recognised ordered existence answers as its declined twin through the real compiled owner, whole and read in part.
use super::super::super::*;
use super::super::route_controls::fixtures::{MeaningCalls, Supplied, run_read};
use super::fixtures::first_of_each_line;
use ryo_execution_domain::domain::coverage::types::{Lacking, Partial};
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::domain::stored_facts::types::PartialCause;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::types::{RowCount, Table},
};
use std::sync::Arc;

/// A recognised ordered existence over a computed source answers what its declined twin answers, whole and read in part: read in
/// part, it is partial in the file and in the repository.
#[test]
fn a_recognised_ordered_existence_read_in_part_answers_as_its_declined_twin() {
    let schema = kinds(Relation::SourceLocation).to_vec();
    // Two locations of file 0 on the same line, at bytes 5 and 1.
    let cells = [(1, 5), (2, 1)]
        .into_iter()
        .flat_map(|(row, byte)| {
            schema
                .iter()
                .enumerate()
                .map(move |(column, kind)| match kind {
                    CellKind::Node => Cell::Node(node_id(FileId(0), Position(row))),
                    CellKind::File => Cell::File(FileId(0)),
                    CellKind::Span => Cell::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                    CellKind::Number => Cell::Number(Integer(if column == 5 { byte } else { 1 })),
                    CellKind::Symbol => {
                        Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0))
                    }
                    CellKind::Flag => Cell::Flag(Flag::Lowered),
                })
        })
        .collect();
    let source = Table {
        kinds: schema,
        cells,
        count: RowCount(2),
    };
    let partials = [Partial {
        relation: Relation::SourceLocation,
        cause: PartialCause("partial provider fixture".into()),
        lacking: Lacking::Any,
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::File(FileId(0)),
    }];
    // Each answer, and how many times the ordered meaning was entered for it.
    let answer = |kept: Vec<ColumnIndex>, publication: &[Partial]| {
        let calls = Arc::new(MeaningCalls::default());
        let shown = match run_read(
            first_of_each_line(kept),
            &[(Relation::SourceLocation, source.clone())],
            (&[], publication),
            Supplied::Computed,
            Arc::clone(&calls),
        ) {
            RootOutcome::Partial { table, affected } => format!("partial {table:?} {affected:?}"),
            RootOutcome::Complete(table) => format!("complete {table:?}"),
            RootOutcome::Failed(error) => format!("failed {error:?}"),
            RootOutcome::Unknown(error) => format!("unknown {error:?}"),
            _ => "other".to_owned(),
        };
        (
            shown,
            calls.ordered.load(std::sync::atomic::Ordering::Relaxed),
        )
    };
    for publication in [&[][..], &partials[..]] {
        let (recognised, ordered) = answer(vec![], publication);
        let (twin, declined) = answer(vec![ColumnIndex(0), ColumnIndex(1)], publication);
        let read_in_part = !publication.is_empty();
        assert_eq!(recognised, twin, "read in part: {read_in_part}");
        assert!(
            ordered > 0,
            "the plan is recognised and its ordered meaning entered"
        );
        assert_eq!(
            declined, 0,
            "the twin is declined and never enters the ordered meaning"
        );
        if read_in_part {
            assert!(recognised.starts_with("partial"), "{recognised}");
            assert!(
                recognised.ends_with("[None, Some(FileId(0))]"),
                "{recognised}"
            );
        } else {
            assert!(recognised.starts_with("complete"), "{recognised}");
        }
    }
}
