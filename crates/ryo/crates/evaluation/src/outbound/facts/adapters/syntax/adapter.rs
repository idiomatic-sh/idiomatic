//! The parser's tree, kept as the parse step built it, and every relation the language's readers write, each walked alone the first
//! time it is asked for.

use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::Facts;
use crate::outbound::facts::types::{ElementRow, Kinds};
use ryo_execution_runtime::outbound::facts::types::ColumnsMut;
use ryo_language::domain::elements::service::{
    end_at, expanded_positions, kind_at, kind_names, positions, start_at, text_at,
};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_language::inbound::use_cases::parse::types::ParseEngine;
use ryo_relations::domain::relations::service::{all, kinds, node_id};
use ryo_relations::domain::relations::types::{
    ColumnsView, FactColumn, FileId, KindBits, Position, Relation, Span, Value,
};
use ryo_relations::domain::terms::types::Symbol;
use std::cell::{OnceCell, RefCell};

/// The tree of one file and the relations built from it so far.
pub(crate) struct FactsSyntaxAdapter<'a> {
    language: &'a dyn ParseEngine,
    elements: RefCell<&'a mut Elements>,
    source: &'a [u8],
    file: FileId,
    built: Vec<OnceCell<Vec<FactColumn>>>,
}

impl std::fmt::Debug for FactsSyntaxAdapter<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(stringify!(FactsSyntaxAdapter))
            .field("file", &self.file)
            .finish_non_exhaustive()
    }
}

impl<'a> FactsSyntaxAdapter<'a> {
    /// Over the tree the parse step built into `elements` for `file`, whose bytes are `source`.
    pub(crate) fn new(
        language: &'a dyn ParseEngine,
        elements: &'a mut Elements,
        source: &'a [u8],
        file: FileId,
    ) -> Self {
        Self {
            language,
            elements: RefCell::new(elements),
            source,
            file,
            built: all().iter().map(|_| OnceCell::new()).collect(),
        }
    }
}

/// The name of the kind of the element at `at`, in the file's own vocabulary; the first name for a kind the vocabulary lacks, which
/// a tree its language built never has.
fn kind_name_at(elements: &Elements, at: Position) -> Symbol {
    let names = kind_names(elements);
    names
        .get(usize::from(kind_at(elements, at).0))
        .or_else(|| names.first())
        .copied()
        .unwrap_or(Symbol(0))
}

/// The tree's readers, walked for one relation: what an adapter composed over the syntax adapter builds a relation of the language
/// with, keeping it as its own.
pub(crate) trait Walks {
    /// The columns of `relation`, walked alone now; names its readers make are interned in `symbols`.
    fn walked(&self, relation: Relation, symbols: &mut Symbols) -> Vec<FactColumn>;
}

impl Walks for FactsSyntaxAdapter<'_> {
    fn walked(&self, relation: Relation, symbols: &mut Symbols) -> Vec<FactColumn> {
        let mut columns = vec![FactColumn::default(); kinds(relation).len()];
        {
            let mut writer: ColumnsMut<'_> = [(relation, columns.as_mut_slice())]
                .into_iter()
                .collect();
            let mut elements = self.elements.borrow_mut();
            ryo_language::inbound::use_cases::parse::service::walk(
                self.language,
                &mut elements,
                symbols,
                ByteSlice(self.source),
                self.file,
                &[relation],
                &mut writer,
            );
        }
        columns
    }
}

impl Facts for FactsSyntaxAdapter<'_> {
    fn walk(&self, kinds: Kinds<'_>, visit: &mut dyn FnMut(Position, KindBits)) {
        let elements = self.elements.borrow();
        let mut wanted = [0_u64; 1024];
        if let Kinds::Only(only) = kinds {
            for kind in only {
                if let Some(word) = wanted.get_mut(usize::from(kind.0) / 64) {
                    *word |= 1 << (kind.0 % 64);
                }
            }
        }
        let added = matches!(kinds, Kinds::Expanded)
            .then(|| expanded_positions(&elements))
            .into_iter()
            .flatten();
        for position in positions(&elements).chain(added) {
            let kind = kind_at(&elements, position);
            let visited = match kinds {
                Kinds::Every | Kinds::Expanded => true,
                Kinds::Only(_) => wanted
                    .get(usize::from(kind.0) / 64)
                    .is_some_and(|word| word & (1 << (kind.0 % 64)) != 0),
            };
            if visited {
                visit(position, kind);
            }
        }
    }

    fn element(&self, at: Position) -> ElementRow {
        let elements = self.elements.borrow();
        ElementRow([
            Value::Node(node_id(self.file, at)),
            Value::Symbol(kind_name_at(&elements, at)),
            Value::Symbol(text_at(&elements, at)),
            Value::File(self.file),
            Value::Span(Span {
                start: start_at(&elements, at),
                end: end_at(&elements, at),
            }),
        ])
    }

    fn relation(
        &self,
        relation: Relation,
        symbols: &mut Symbols,
    ) -> Result<ColumnsView<'_>, FactsError> {
        let place = all()
            .iter()
            .position(|each| *each == relation)
            .ok_or(FactsError::Unserved { relation })?;
        let cell = self
            .built
            .get(place)
            .ok_or(FactsError::Unserved { relation })?;
        if cell.get().is_none() {
            drop(cell.set(self.walked(relation, symbols)));
        }
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns: cell.get().map_or(&[][..], Vec::as_slice),
        })
    }
}
