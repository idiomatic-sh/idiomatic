//! `source-location` from a line index of the file's bytes, built the first time a rule asks for it; every other fact is the
//! adapter's inner one's.

use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::Facts;
use crate::outbound::facts::types::{ElementRow, Kinds};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::{kinds, to_bits};
use ryo_relations::domain::relations::types::{
    ColumnsView, FactColumn, Integer, KindBits, Position, Relation, Value,
};
use std::cell::OnceCell;

/// `source-location` over the facts of `inner`.
#[derive(Debug)]
pub(crate) struct FactsLocationAdapter<'a, F> {
    inner: F,
    source: &'a [u8],
    built: OnceCell<Vec<FactColumn>>,
}

impl<'a, F> FactsLocationAdapter<'a, F> {
    /// Over `inner`, for the file whose bytes are `source`.
    pub(crate) fn new(inner: F, source: &'a [u8]) -> Self {
        Self {
            inner,
            source,
            built: OnceCell::new(),
        }
    }
}

/// Where each line of `source` starts: CRLF is one break; a lone CR or LF also ends a line.
fn line_starts(source: &[u8]) -> Vec<u32> {
    let mut starts = vec![0];
    let mut at = 0;
    while let Some(byte) = source.get(at) {
        let newline = matches!(*byte, b'\r' | b'\n');
        if *byte == b'\r' && source.get(at + 1) == Some(&b'\n') {
            at += 1;
        }
        at += 1;
        if newline {
            starts.push(u32::try_from(at).unwrap_or(u32::MAX));
        }
    }
    starts
}

/// The one-based line and the zero-based byte column of `offset`.
fn place(starts: &[u32], offset: u32) -> (i64, i64) {
    let line = starts.partition_point(|start| *start <= offset);
    let start = line
        .checked_sub(1)
        .and_then(|line| starts.get(line))
        .copied()
        .unwrap_or(0);
    (
        i64::try_from(line).unwrap_or(i64::MAX),
        i64::from(offset.saturating_sub(start)),
    )
}

impl<F: Facts> Facts for FactsLocationAdapter<'_, F> {
    fn walk(&self, kinds: Kinds<'_>, visit: &mut dyn FnMut(Position, KindBits)) {
        self.inner.walk(kinds, visit);
    }

    fn element(&self, at: Position) -> ElementRow {
        self.inner.element(at)
    }

    fn relation(
        &self,
        relation: Relation,
        symbols: &mut Symbols,
    ) -> Result<ColumnsView<'_>, FactsError> {
        if relation != Relation::SourceLocation {
            return self.inner.relation(relation, symbols);
        }
        let columns = self.built.get_or_init(|| {
            let starts = line_starts(self.source);
            let mut columns = vec![FactColumn::default(); kinds(relation).len()];
            self.inner.walk(Kinds::Expanded, &mut |position, _| {
                let ElementRow([id, _, _, file, span]) = self.inner.element(position);
                let Value::Span(located) = span else {
                    return;
                };
                if located.start > located.end
                    || usize::try_from(located.end.0).unwrap_or(usize::MAX) > self.source.len()
                {
                    return;
                }
                let (start_line, start_column) = place(&starts, located.start.0);
                let (end_line, end_column) = place(&starts, located.end.0);
                let row = [
                    id,
                    file,
                    span,
                    Value::Number(Integer(start_line)),
                    Value::Number(Integer(end_line)),
                    Value::Number(Integer(i64::from(located.start.0))),
                    Value::Number(Integer(i64::from(located.end.0))),
                    Value::Number(Integer(start_column)),
                    Value::Number(Integer(end_column)),
                ];
                for (column, value) in columns.iter_mut().zip(row) {
                    column.0.push(to_bits(value).0);
                }
            });
            columns
        });
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns,
        })
    }
}
