//! What a run asks of the inputs a case lends it, in order: the trace a streamed run and its held twin must share.
use ryo_relations::domain::relations::types::Relation;
use std::cell::RefCell;

/// One thing a run asked of the inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Demand {
    /// A relation's rows, lent.
    Lent(Relation),
    /// A relation's rows, refused: the engine records the refusal and it decides the file's outcome.
    Refused(Relation),
    /// What the files handed over for a frontier entry.
    Frontier,
    /// The rows a bound observation answers.
    Observation,
}

/// What a run asked of the inputs, in order.
#[derive(Debug, Default)]
pub struct Demands(RefCell<Vec<Demand>>);

impl Demands {
    /// Records one thing the run asked.
    pub(crate) fn push(&self, demand: Demand) {
        self.0.borrow_mut().push(demand);
    }

    /// Records a relation the run asked for: lent when the inputs hold it (one of `held`), refused when they do not.
    pub(crate) fn asked(&self, relation: Relation, mut held: impl Iterator<Item = Relation>) {
        self.push(if held.any(|each| each == relation) {
            Demand::Lent(relation)
        } else {
            Demand::Refused(relation)
        });
    }

    /// What the run asked so far, in order.
    #[must_use]
    pub fn trace(&self) -> Vec<Demand> {
        self.0.borrow().clone()
    }
}
