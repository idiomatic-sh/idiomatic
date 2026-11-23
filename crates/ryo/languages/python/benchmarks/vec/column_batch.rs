//! Hold a bounded group of adjacent file loans until every prefix in the group
//! is finished. The nested continuation visits the executor in request order.
use super::source_batch::SourceBatch;
use ryo::languages::NodeTable;
use ryo_python::fixtures::columns::{ColumnFile, ColumnWalk};

pub(super) fn walked<S, P, C>(
    walks: &mut [&mut ColumnWalk],
    tables: &mut [NodeTable],
    states: &mut [S],
    paths: &[String],
    sources: &SourceBatch,
    first: usize,
    prepare: &mut P,
    consume: &mut C,
) where
    P: FnMut(usize, &mut ColumnFile<'_, '_>, &mut S),
    C: FnMut(usize, &mut ColumnFile<'_, '_>, &S),
{
    fn group<S, P, C>(
        walks: &mut [&mut ColumnWalk],
        tables: &mut [NodeTable],
        states: &mut [S],
        paths: &[String],
        sources: &SourceBatch,
        first: usize,
        slot: usize,
        prepare: &mut P,
        consume: &mut C,
        earlier: &mut dyn FnMut(&mut C),
    ) where
        P: FnMut(usize, &mut ColumnFile<'_, '_>, &mut S),
        C: FnMut(usize, &mut ColumnFile<'_, '_>, &S),
    {
        let Some((walk, tail)) = walks.split_first_mut() else {
            earlier(consume);
            return;
        };
        let (table, rest) = tables.split_first_mut().expect("one table per loan");
        let (state, more) = states.split_first_mut().expect("one state per loan");
        let mut called = false;
        let _ = walk.walked(
            &paths[slot],
            sources.get(first + slot),
            table,
            &mut |file| {
                called = true;
                prepare(slot, file, state);
                group(
                    tail,
                    rest,
                    more,
                    paths,
                    sources,
                    first,
                    slot + 1,
                    prepare,
                    consume,
                    &mut |visit| {
                        earlier(visit);
                        visit(slot, file, state);
                    },
                );
            },
        );
        if !called {
            group(
                tail,
                rest,
                more,
                paths,
                sources,
                first,
                slot + 1,
                prepare,
                consume,
                earlier,
            );
        }
    }
    group(
        walks,
        tables,
        states,
        paths,
        sources,
        first,
        0,
        prepare,
        consume,
        &mut |_| {},
    );
}
