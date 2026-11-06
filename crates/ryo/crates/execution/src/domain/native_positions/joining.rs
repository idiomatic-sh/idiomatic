//! Equality joins over borrowed positions and prepared tree rows.

use super::types::{PositionInput, PositionWork};
use crate::domain::{
    comparing::service::holds_all,
    lowering::{
        constants::EDGE,
        service::{kind_of, viewed},
        types::LoweredShape,
    },
    operating::{
        errors::OperatingError,
        types::{NameCount, OperatorContext},
    },
    operator::contract::{
        types::{PrimitiveSemantics, WorkCounters, join_result},
    },
    positions::{
        service::{count, followed, in_tree, key_into, row, value},
        types::{FoundPositions, Positions, RowPositions, Slots},
    },
    table_cells::types::TreeRows,
    table_model::types::Verdict,
    walks::{
        service::{allows, give, take_filter},
        types::Admit,
    },
};
use ryo_derived::domain::contract::types::RawOrder;
use ryo_language::domain::elements::service::children;
use ryo_language::domain::names::service::name_count;
use ryo_operators::inbound::use_cases::operators::interface::{
    JoinInput, JoinInterrupted, OperatorCall, PositionJoinDriver,
};
use ryo_relations::domain::relations::types::{Position, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, PlanComparison, Input},
    plans::types::Operation,
    rows::types::RowIndex,
};

struct StepMatches<'a> {
    right: &'a Positions,
    order: &'a RawOrder,
    place: RowIndex,
    end: RowIndex,
    current: Option<RowIndex>,
    counts: &'a WorkCounters,
}
impl PositionJoinDriver for StepMatches<'_> {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        while self.place.0 < self.end.0 {
            let place = self.place;
            self.place.0 += 1;
            if let Some(right) = self.order.rows().get(place.0) {
                self.current = Some(RowIndex(*right));
                self.counts.found(1, 1);
                return Ok(true);
            }
        }
        self.current = None;
        Ok(false)
    }
    fn right_positions(&self) -> &[u32] {
        self.current.map_or(&[], |at| row(self.right, at).0)
    }
}

#[derive(Clone, Copy)]
enum TreeCandidate {
    Pending,
    Exhausted,
}
#[derive(Clone, Copy)]
enum TreeAdmission {
    Unchecked,
    Admitted,
}

struct TreeMatch<'a, 't, 'c, 'f> {
    tree: &'a TreeRows<'t>,
    admit: &'a mut Admit<'f>,
    context: &'a mut OperatorContext<'c>,
    hit: RowPositions<'a>,
    pending: TreeCandidate,
    ready: TreeAdmission,
    error: Option<OperatingError>,
    counts: &'a WorkCounters,
}
impl PositionJoinDriver for TreeMatch<'_, '_, '_, '_> {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        self.ready = TreeAdmission::Unchecked;
        if matches!(self.pending, TreeCandidate::Exhausted) {
            return Ok(false);
        }
        self.pending = TreeCandidate::Exhausted;
        self.counts.found(1, 1);
        match allows(self.admit, self.tree, Position(self.hit.0[0]), self.context) {
            Ok(verdict) => {
                self.ready = if verdict == Verdict::Holds {
                    TreeAdmission::Admitted
                } else {
                    TreeAdmission::Unchecked
                };
                Ok(matches!(self.ready, TreeAdmission::Admitted))
            }
            Err(error) => {
                self.error = Some(error);
                Err(JoinInterrupted)
            }
        }
    }
    fn right_positions(&self) -> &[u32] {
        if matches!(self.ready, TreeAdmission::Admitted) && self.error.is_none() {
            self.hit.0
        } else {
            &[]
        }
    }
}

struct EdgeMatches<'a, 't, 'c> {
    tree: &'a TreeRows<'t>,
    filter: &'a [PlanComparison],
    cells: &'a mut Vec<Value>,
    context: &'a mut OperatorContext<'c>,
    found: &'a FoundPositions,
    at: RowIndex,
    current: Option<RowIndex>,
    error: Option<OperatingError>,
    counts: &'a WorkCounters,
}
impl PositionJoinDriver for EdgeMatches<'_, '_, '_> {
    fn next(&mut self) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        self.current = None;
        while let Some(child) = self.found.0.get(self.at.0).copied() {
            let at = self.at;
            self.at.0 += 1;
            self.counts.found(1, 1);
            if self.filter.is_empty() {
                self.current = Some(at);
                return Ok(true);
            }
            self.cells.clear();
            for field in EDGE {
                self.cells.push(value(self.tree, field, Position(child)));
            }
            let kinds = EDGE.map(kind_of);
            let names = NameCount(name_count(self.context.symbols.as_ref()).0);
            match holds_all(self.filter, self.cells, &kinds, names, self.context) {
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
        self.current
            .and_then(|at| self.found.0.get(at.0..at.0 + 1))
            .unwrap_or(&[])
    }
}

pub(super) fn join(
    semantics: &dyn PrimitiveSemantics,
    order: Option<&RawOrder>,
    operation: &Operation,
    tree: &TreeRows<'_>,
    inputs: &[PositionInput<'_>],
    output: &LoweredShape,
    context: &mut OperatorContext<'_>,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Positions, OperatingError> {
    let Operation::Join {
        right,
        left_on,
        right_on,
        filter,
        view,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let left = inputs
        .first()
        .copied()
        .ok_or(OperatingError::MissingInput { position: 1 })?;
    let key_column = left_on
        .first()
        .and_then(|on| {
            left.shape
                .columns
                .get(usize::try_from(on.0).unwrap_or(usize::MAX))
        })
        .copied()
        .ok_or(OperatingError::NotRun)?;
    let mut slots = Slots::default();
    match right {
        Input::Step(_) => {
            let right = inputs
                .get(1)
                .copied()
                .ok_or(OperatingError::MissingInput { position: 2 })?;
            let order = order.ok_or(OperatingError::WrongOperation)?;
            let PositionWork { probe, .. } = work;
            for at in positions(count(left.rows)) {
                counts.found(0, 1);
                let each = row(left.rows, at);
                probe.0.clear();
                key_into(tree, &left.shape.columns, each, left_on, probe);
                let hits = order.hits(&probe.0);
                let mut driver = StepMatches {
                    right: right.rows,
                    order,
                    place: RowIndex(hits.start),
                    end: RowIndex(hits.end),
                    current: None,
                    counts,
                };
                join_result(
                    semantics.operators(OperatorCall::EqualityJoin {
                        input: JoinInput::Positions {
                            left: each.0,
                            driver: &mut driver,
                            target: &mut slots.0,
                        },
                    }),
                    None,
                )?;
            }
        }
        Input::Relation(Relation::SyntaxTree) => {
            let mut admit = if filter.is_empty() {
                Admit::All
            } else {
                Admit::Filter(take_filter(tree, filter, context), filter)
            };
            let mut result = Ok(());
            for at in positions(count(left.rows)) {
                counts.found(0, 1);
                let each = row(left.rows, at);
                let Some(hit) = followed(tree, key_column, each) else {
                    continue;
                };
                let hit = in_tree(tree, hit);
                let hit_slot = [hit.0];
                let mut driver = TreeMatch {
                    tree,
                    admit: &mut admit,
                    context,
                    hit: RowPositions(&hit_slot),
                    pending: TreeCandidate::Pending,
                    ready: TreeAdmission::Unchecked,
                    error: None,
                    counts,
                };
                if let Err(error) = join_result(
                    semantics.operators(OperatorCall::EqualityJoin {
                        input: JoinInput::Positions {
                            left: each.0,
                            driver: &mut driver,
                            target: &mut slots.0,
                        },
                    }),
                    driver.error.take(),
                ) {
                    result = Err(error);
                    break;
                }
            }
            give(admit, context);
            result?;
        }
        Input::Relation(Relation::SyntaxChild) => {
            let PositionWork { found, cells, .. } = work;
            let by_parent = right_on
                .first()
                .is_some_and(|on| viewed(view, *on) == ColumnIndex(0));
            for at in positions(count(left.rows)) {
                counts.found(0, 1);
                let each = row(left.rows, at);
                let Some(start) = followed(tree, key_column, each) else {
                    continue;
                };
                let start = in_tree(tree, start);
                found.0.clear();
                if by_parent {
                    found
                        .0
                        .extend(children(tree.elements, start).map(|child| child.0));
                } else if start.0 >= 1 {
                    found.0.push(start.0);
                }
                let mut driver = EdgeMatches {
                    tree,
                    filter,
                    cells,
                    context,
                    found,
                    at: RowIndex(0),
                    current: None,
                    error: None,
                    counts,
                };
                join_result(
                    semantics.operators(OperatorCall::EqualityJoin {
                        input: JoinInput::Positions {
                            left: each.0,
                            driver: &mut driver,
                            target: &mut slots.0,
                        },
                    }),
                    driver.error.take(),
                )?;
            }
        }
        Input::Relation(_) | Input::BoundObservation(_) => return Err(OperatingError::NotAStep),
    }
    Ok(Positions {
        width: output.width,
        slots,
    })
}

use ryo_values::domain::rows::service::positions;
