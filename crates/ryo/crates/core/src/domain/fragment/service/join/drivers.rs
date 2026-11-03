//! Fixed position inputs and predicate admission, with no row-value materialization.
use super::*;
use ryo_derived::domain::contract::types::RawOrder;
pub(super) struct Tree<'a, 'f, 'c, 'p> {
    #[cfg(feature = "diagnostic-row-progress")]
    pub(super) progress: crate::domain::operator::contract::views::ProgressObserver,
    pub(super) tree: &'a TreeRows<'f>,
    pub(super) admit: &'a mut Admit<'p>,
    pub(super) context: &'a mut OperatorContext<'c>,
    pub(super) hit: [u32; 1],
    pub(super) pending: bool,
    pub(super) ready: bool,
    pub(super) error: Option<OperatingError>,
}
impl PositionJoinDriver for Tree<'_, '_, '_, '_> {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        self.ready = false;
        if !self.pending {
            return Ok(false);
        }
        self.pending = false;
        #[cfg(feature = "diagnostic-row-progress")]
        (self.progress)(
            crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
            1,
        );
        #[cfg(feature = "diagnostic-row-progress")]
        if !matches!(&*self.admit, Admit::All) {
            (self.progress)(
                crate::domain::operator::contract::views::ProgressCounter::PredicateEvaluations,
                1,
            );
        }
        match allows(self.admit, self.tree, Position(self.hit[0]), self.context) {
            Ok(verdict) => {
                self.ready = verdict == Verdict::Holds;
                Ok(self.ready)
            }
            Err(error) => {
                self.error = Some(error);
                Err(JoinInterrupted)
            }
        }
    }
    fn right_positions(&self) -> &[u32] {
        if self.ready && self.error.is_none() {
            #[cfg(feature = "diagnostic-row-progress")]
            (self.progress)(
                crate::domain::operator::contract::views::ProgressCounter::PositionRowsYielded,
                1,
            );
            &self.hit
        } else {
            &[]
        }
    }
}
pub(super) struct Steps<'a> {
    #[cfg(feature = "diagnostic-row-progress")]
    pub(super) progress: crate::domain::operator::contract::views::ProgressObserver,
    pub(super) right: &'a Positions,
    pub(super) order: &'a RawOrder,
    pub(super) places: std::ops::Range<usize>,
    pub(super) current: Option<usize>,
}
impl PositionJoinDriver for Steps<'_> {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        for place in self.places.by_ref() {
            if let Some(right) = self.order.rows().get(place) {
                #[cfg(feature = "diagnostic-row-progress")]
                (self.progress)(
                    crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                    1,
                );
                self.current = Some(*right);
                return Ok(true);
            }
        }
        self.current = None;
        Ok(false)
    }
    fn right_positions(&self) -> &[u32] {
        #[cfg(feature = "diagnostic-row-progress")]
        if self.current.is_some() {
            (self.progress)(
                crate::domain::operator::contract::views::ProgressCounter::PositionRowsYielded,
                1,
            );
        }
        self.current
            .map_or(&[], |at| row(self.right, RowIndex(at)).0)
    }
}
pub(super) struct Edges<'a, 'f, 'c> {
    #[cfg(feature = "diagnostic-row-progress")]
    pub(super) progress: crate::domain::operator::contract::views::ProgressObserver,
    pub(super) tree: &'a TreeRows<'f>,
    pub(super) filter: &'a [Comparison],
    pub(super) cells: &'a mut Vec<Value>,
    pub(super) context: &'a mut OperatorContext<'c>,
    pub(super) found: &'a [u32],
    pub(super) at: usize,
    pub(super) current: Option<usize>,
    pub(super) error: Option<OperatingError>,
}
impl PositionJoinDriver for Edges<'_, '_, '_> {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        self.current = None;
        while let Some(child) = self.found.get(self.at).copied() {
            let at = self.at;
            self.at += 1;
            #[cfg(feature = "diagnostic-row-progress")]
            (self.progress)(
                crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                1,
            );
            #[cfg(feature = "diagnostic-row-progress")]
            if !self.filter.is_empty() {
                (self.progress)(
                    crate::domain::operator::contract::views::ProgressCounter::PredicateEvaluations,
                    1,
                );
            }
            match edge_admitted(
                self.tree,
                self.filter,
                Position(child),
                self.cells,
                self.context,
            ) {
                Ok(Verdict::Holds) => {
                    self.current = Some(at);
                    return Ok(true);
                }
                Ok(Verdict::Fails) => {}
                Err(error) => {
                    self.error = Some(error);
                    return Err(JoinInterrupted);
                }
            }
        }
        Ok(false)
    }
    fn right_positions(&self) -> &[u32] {
        if self.error.is_some() {
            return &[];
        }
        #[cfg(feature = "diagnostic-row-progress")]
        if self.current.is_some() {
            (self.progress)(
                crate::domain::operator::contract::views::ProgressCounter::PositionRowsYielded,
                1,
            );
        }
        self.current
            .and_then(|at| self.found.get(at..at.saturating_add(1)))
            .unwrap_or(&[])
    }
}
