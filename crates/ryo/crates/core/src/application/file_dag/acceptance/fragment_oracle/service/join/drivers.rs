//! Fixed position inputs and predicate admission, with no row-value materialization.
use crate::application::file_dag::acceptance::fragment_oracle::types::CandidatePlaces;
use crate::domain::comparing::service::holds_all;
use crate::domain::lowering::constants::EDGE;
use crate::domain::lowering::service::kind_of;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{NameCount, OperatorContext};
use crate::domain::positions::service::{row, value};
use crate::domain::positions::types::{Positions, Slots};
use crate::domain::table_cells::types::TreeRows;
use crate::domain::table_model::types::Verdict;
use crate::domain::walks::service::allows;
use crate::domain::walks::types::Admit;
use ryo_derived::domain::contract::types::RawOrder;
use ryo_language::domain::symbols::service::name_count;
use ryo_operators::domain::contract::types::{JoinInterrupted, PositionJoinDriver};
use ryo_relations::domain::relations::types::{CellKind, Position, Value};
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::rows::types::RowIndex;

/// Whether the edge to the element at `child` is one the comparisons admit.
fn edge_admitted(
    tree: &TreeRows<'_>,
    comparisons: &[Comparison],
    child: Position,
    cells: &mut Vec<Value>,
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    if comparisons.is_empty() {
        return Ok(Verdict::Holds);
    }
    cells.clear();
    for field in EDGE {
        cells.push(value(tree, field, child));
    }
    let kinds: Vec<CellKind> = EDGE.iter().map(|field| kind_of(*field)).collect();
    let names = NameCount(name_count(context.symbols.as_ref()).0);
    holds_all(comparisons, cells, &kinds, names, context)
}

pub(super) enum TreeHitState {
    Pending,
    Ready,
    Consumed,
}
pub(super) struct OracleTreeDriver<'a, 'f, 'c, 'p> {
    #[cfg(feature = "diagnostic-row-progress")]
    pub(super) progress: crate::domain::operator::contract::views::ProgressObserver,
    pub(super) tree: &'a TreeRows<'f>,
    pub(super) admit: &'a mut Admit<'p>,
    pub(super) context: &'a mut OperatorContext<'c>,
    pub(super) hit: Slots,
    pub(super) state: TreeHitState,
    pub(super) error: Option<OperatingError>,
}
impl PositionJoinDriver for OracleTreeDriver<'_, '_, '_, '_> {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        let pending = matches!(self.state, TreeHitState::Pending);
        self.state = TreeHitState::Consumed;
        if !pending {
            return Ok(false);
        }
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
        match allows(self.admit, self.tree, Position(self.hit.0[0]), self.context) {
            Ok(verdict) => {
                if verdict == Verdict::Holds {
                    self.state = TreeHitState::Ready;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(error) => {
                self.error = Some(error);
                Err(JoinInterrupted)
            }
        }
    }
    fn right_positions(&self) -> &[u32] {
        if matches!(self.state, TreeHitState::Ready) && self.error.is_none() {
            #[cfg(feature = "diagnostic-row-progress")]
            (self.progress)(
                crate::domain::operator::contract::views::ProgressCounter::PositionRowsYielded,
                1,
            );
            &self.hit.0
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
    pub(super) places: CandidatePlaces,
    pub(super) current: Option<RowIndex>,
}
impl PositionJoinDriver for Steps<'_> {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        for place in self.places.0.by_ref() {
            if let Some(right) = self.order.rows().get(place) {
                #[cfg(feature = "diagnostic-row-progress")]
                (self.progress)(
                    crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                    1,
                );
                self.current = Some(RowIndex(*right));
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
        self.current.map_or(&[], |at| row(self.right, at).0)
    }
}
pub(super) struct OracleEdgeDriver<'a, 'f, 'c> {
    #[cfg(feature = "diagnostic-row-progress")]
    pub(super) progress: crate::domain::operator::contract::views::ProgressObserver,
    pub(super) tree: &'a TreeRows<'f>,
    pub(super) filter: &'a [Comparison],
    pub(super) cells: &'a mut Vec<Value>,
    pub(super) context: &'a mut OperatorContext<'c>,
    pub(super) found: &'a Slots,
    pub(super) at: RowIndex,
    pub(super) current: Option<RowIndex>,
    pub(super) error: Option<OperatingError>,
}
impl PositionJoinDriver for OracleEdgeDriver<'_, '_, '_> {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        self.current = None;
        while let Some(child) = self.found.0.get(self.at.0).copied() {
            let at = self.at;
            self.at.0 += 1;
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
            .and_then(|at| self.found.0.get(at.0..at.0.saturating_add(1)))
            .unwrap_or(&[])
    }
}
