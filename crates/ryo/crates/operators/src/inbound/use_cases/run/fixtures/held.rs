//! The rows an engine holds for a run after the files, as the plan and run cases lend them: the host's rows of each relation,
//! counted per read, and the token every withheld input answers.
use super::demands::{Demand, Demands};
use crate::domain::contract::types::WithheldInput;
use crate::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::rows::types::Table;
use std::cell::Cell;

/// The rows an engine holds for a run after the files, each lent in place: no frontier entry or bound observation has rows,
/// and a withheld input answers its token.
#[derive(Debug, Default)]
pub(crate) struct HeldInputs {
    /// The host's rows of each relation, by its place.
    pub(super) hosted: Vec<Table>,
    /// The token every withheld input answers, when the case withholds them.
    pub(super) withheld: Option<WithheldInput>,
    /// How many times the host's rows were read.
    pub(super) hosted_reads: Cell<usize>,
    /// The rows of an input no case holds.
    pub(super) empty: Table,
    /// What the run asked of the inputs, in order: every relation, frontier entry and bound observation.
    pub(crate) demands: Demands,
}

impl HeldInputs {
    /// The rows of a frontier entry or a bound observation, `demand` recorded: none, or the token every withheld input
    /// answers.
    pub(super) fn held(&self, demand: Demand) -> HeldRows<'_> {
        self.demands.push(demand);
        self.withheld
            .map_or(HeldRows::Held(&self.empty), HeldRows::Withheld)
    }

    /// The host's rows of `relation`, the read counted: none for a relation the case holds no rows of, or the token every
    /// withheld input answers.
    pub(super) fn hosted(&self, relation: HostedNumber) -> HeldRows<'_> {
        self.hosted_reads.set(self.hosted_reads.get() + 1);
        let held = self
            .hosted
            .get(usize::try_from(relation.0).unwrap_or(usize::MAX));
        match (self.withheld, held) {
            (Some(token), _) => HeldRows::Withheld(token),
            (None, Some(table)) => HeldRows::Held(table),
            (None, None) => HeldRows::Held(&self.empty),
        }
    }
}
