//! `qualified-name` and the `binding-*` relations, each built the first time a rule asks for it from the language's own analysis
//! over the tree its inner adapter keeps; every other fact is the inner adapter's.

use crate::outbound::facts::adapters::syntax::adapter::Walks;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::Facts;
use crate::outbound::facts::types::{ElementRow, Kinds};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{
    ColumnsView, FactColumn, KindBits, Position, Relation,
};
use std::cell::OnceCell;

/// The relations this adapter makes.
const SEMANTIC: [Relation; 5] = [
    Relation::QualifiedName,
    Relation::BindingEvent,
    Relation::BindingRead,
    Relation::BindingShadow,
    Relation::BindingUse,
];

/// Name binding and resolution over the tree of `inner`.
#[derive(Debug)]
pub(crate) struct FactsSemanticAdapter<F> {
    inner: F,
    built: [OnceCell<Vec<FactColumn>>; 5],
}

impl<F> FactsSemanticAdapter<F> {
    /// Over `inner`.
    pub(crate) fn new(inner: F) -> Self {
        Self {
            inner,
            built: Default::default(),
        }
    }
}

impl<F: Facts + Walks> Facts for FactsSemanticAdapter<F> {
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
        let Some(cell) = SEMANTIC
            .iter()
            .position(|each| *each == relation)
            .and_then(|place| self.built.get(place))
        else {
            return self.inner.relation(relation, symbols);
        };
        if cell.get().is_none() {
            drop(cell.set(self.inner.walked(relation, symbols)));
        }
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns: cell.get().map_or(&[][..], Vec::as_slice),
        })
    }
}
