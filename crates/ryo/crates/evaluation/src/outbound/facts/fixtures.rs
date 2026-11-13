//! The `facts` port's test support: the facts of a file read eagerly, every relation in the one walk the interpreter's parse step
//! makes, to compare with the lazy adapters.

use super::types::{ElementRow, Kinds};
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::Facts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, KindBits, Position, Relation};

/// Every walk the inner facts are asked for, counted: how a test sees the tree was walked once.
#[derive(Debug)]
pub struct CountedWalks<F> {
    /// The facts counted.
    pub inner: F,
    /// How many walks were asked for.
    pub walks: std::cell::Cell<usize>,
    /// Every relation asked for, in order.
    pub asked: std::cell::RefCell<Vec<Relation>>,
}

impl<F: Facts> Facts for CountedWalks<F> {
    fn walk(&self, kinds: Kinds<'_>, visit: &mut dyn FnMut(Position, KindBits)) {
        self.walks.set(self.walks.get() + 1);
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
        self.asked.borrow_mut().push(relation);
        self.inner.relation(relation, symbols)
    }
}
