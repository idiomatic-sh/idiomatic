//! Request-only interning. Runtime acquisition compares prepared IDs, never request metadata.
use ryo_execution_domain::domain::{
    derived_views::types::{
        contract::value::{
            DerivedContract, DerivedSlots, PreparedViewGroup, PreparedViewUse, kind_of,
        },
        *,
    },
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    operator::contract::{
        resources::{IndexColumns, ResourceProgram},
        types::OperatorContract,
    },
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, InputNumber};
use std::sync::Arc;
pub(super) mod identity;
use identity::types::StaticRequest;
use types::InputPositions;
/// The derived views a request's operators bind, prepared once for the request.
pub struct RequestViews {
    semantic: crate::application::file_dag::derived::owner::types::DerivedAccess,
    plans: Vec<Arc<DerivedContract>>,
    bindings: Vec<Arc<PreparedViewUse>>,
    logical: Vec<ryo_values::domain::plan_parts::types::Input>,
}
impl RequestViews {
    /// The views of one request, prepared through `semantic`.
    pub fn new(
        semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    ) -> Self {
        Self {
            semantic: semantic.into(),
            plans: Vec::new(),
            bindings: Vec::new(),
            logical: Vec::new(),
        }
    }
    /// Bind finite semantic requirements once per compiled operator. No rows or cache are read here.
    pub fn bind(
        &mut self,
        contract: &mut OperatorContract,
    ) -> Result<(), OperatingError> {
        self.logical.clear();
        self.logical
            .extend_from_slice(ryo_values::domain::plans::service::named(&contract.inputs));
        contract.derived = DerivedSlots {
            slots: self.bind_resources(&contract.resources, &contract.input_kinds)?,
        };
        Ok(())
    }

    fn bind_resources(
        &mut self,
        program: &ResourceProgram,
        kinds: &[Vec<ryo_relations::domain::relations::types::CellKind>],
    ) -> Result<[Option<Arc<PreparedViewGroup>>; DerivedKind::ALL.len()], OperatingError> {
        let mut slots: [Option<Arc<PreparedViewGroup>>; DerivedKind::ALL.len()] =
            std::array::from_fn(|_| None);
        match program {
            ResourceProgram::Index { input, columns } => {
                let columns = match columns {
                    IndexColumns::Whole => (0..kinds.get(input.0).map_or(0, Vec::len))
                        .map(|at| ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX)))
                        .collect(),
                    IndexColumns::Columns(columns) | IndexColumns::ColumnsUnlessEmpty(columns) => {
                        columns.clone()
                    }
                };
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::Equality { columns },
                    InputPositions(&[input.0]),
                )?;
            }
            ResourceProgram::Join(join) => {
                if let Some(reach) = &join.reach {
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::Adjacency {
                            endpoints: [reach.source, reach.destination],
                            keys: reach.keys.clone(),
                        },
                        InputPositions(&[1]),
                    )?;
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::PartitionOrder {
                            keys: reach.keys.clone(),
                        },
                        InputPositions(&[1]),
                    )?;
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::ClosureAdjacency {
                            source: reach.source,
                            destination: reach.destination,
                        },
                        InputPositions(&[1]),
                    )?;
                } else {
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::Equality {
                            columns: join.right_on.clone(),
                        },
                        InputPositions(&[1]),
                    )?;
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::Equality {
                            columns: join.left_on.clone(),
                        },
                        InputPositions(&[0]),
                    )?;
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::NativeLookup {
                            left_on: join.left_on.clone(),
                            right_on: join.right_on.clone(),
                            view: join.view.clone(),
                        },
                        InputPositions(&[0, 1]),
                    )?;
                }
            }
            ResourceProgram::Axis(axis) => {
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::Adjacency {
                        endpoints: [axis.parent, axis.child],
                        keys: Vec::new(),
                    },
                    InputPositions(&[1]),
                )?;
                let from = match axis.direction {
                    Direction::Parent | Direction::Ancestor | Direction::NearestAncestor => {
                        axis.child
                    }
                    Direction::Child
                    | Direction::Descendant
                    | Direction::Deep
                    | Direction::PreviousSibling
                    | Direction::NextSibling => axis.parent,
                };
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::Equality {
                        columns: vec![from],
                    },
                    InputPositions(&[1]),
                )?;
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::Membership {
                        column: axis.domain_id,
                    },
                    InputPositions(&[2, 1]),
                )?;
            }
            ResourceProgram::Closure {
                keys,
                source,
                destination,
            } => {
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::PartitionOrder { keys: keys.clone() },
                    InputPositions(&[0]),
                )?;
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::ClosureAdjacency {
                        source: *source,
                        destination: *destination,
                    },
                    InputPositions(&[0]),
                )?;
            }
            ResourceProgram::Cycles {
                source,
                destination,
            } => {
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::CycleAdjacency {
                        source: *source,
                        destination: *destination,
                    },
                    InputPositions(&[0]),
                )?;
            }
            ResourceProgram::Ordered {
                left_on,
                right_on,
                left_order,
                right_order,
            } => {
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::OrderedKeys {
                        left_on: left_on.clone(),
                        right_on: right_on.clone(),
                        left_order: *left_order,
                        right_order: *right_order,
                    },
                    InputPositions(&[0, 1]),
                )?;
            }
            ResourceProgram::Dfs(program) => {
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::OrderedDfs(program.clone()),
                    InputPositions(&[0, 1]),
                )?;
            }
            ResourceProgram::Count { keys, .. } | ResourceProgram::Digest { keys, .. } => {
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::PartitionOrder { keys: keys.clone() },
                    InputPositions(&[0]),
                )?;
            }
            ResourceProgram::None | ResourceProgram::Appended(_) | ResourceProgram::Rank(_) => {}
        }
        Ok(slots)
    }

    fn add(
        &mut self,
        slots: &mut [Option<Arc<PreparedViewGroup>>; DerivedKind::ALL.len()],
        kinds: &[Vec<ryo_relations::domain::relations::types::CellKind>],
        request: ViewRequest,
        positions: InputPositions<'_>,
    ) -> Result<(), OperatingError> {
        let positions = positions.0;
        let mut schemas = Vec::with_capacity(positions.len());
        let mut missing = None;
        for position in positions {
            if let Some(kinds) = kinds.get(*position) {
                schemas.push(InputShape { kinds, tree: None });
            } else {
                missing = Some(OperatingError::MissingInput {
                    position: *position + 1,
                });
                break;
            }
        }
        let at = kind_of(&request).ordinal();
        if slots[at]
            .as_ref()
            .is_some_and(|group| group.secondary.is_some())
            || positions.len() > 2
        {
            return Err(OperatingError::WrongOperation);
        }
        let plan = if let Some(error) = missing {
            self.intern_missing(&request, &schemas, error)?
        } else {
            self.intern(&request, &schemas)?
        };
        let mut inputs = [InputNumber(0); 2];
        for (input, position) in inputs.iter_mut().zip(positions) {
            *input = InputNumber(*position);
        }
        let relations = std::array::from_fn(|at| {
            positions
                .get(at)
                .and_then(|position| self.logical.get(*position))
                .and_then(|input| match input {
                    ryo_values::domain::plan_parts::types::Input::Relation(relation) => {
                        Some(*relation)
                    }
                    ryo_values::domain::plan_parts::types::Input::Step(_)
                    | ryo_values::domain::plan_parts::types::Input::BoundObservation(_) => None,
                })
        });
        let binding = self.bindings.iter().find(|binding| {
            Arc::ptr_eq(&binding.plan, &plan)
                && binding.inputs == inputs
                && binding.relations == relations
                && usize::from(binding.len.0) == positions.len()
        });
        let binding = if let Some(binding) = binding {
            (binding).clone()
        } else {
            let binding = Arc::new(PreparedViewUse {
                plan,
                inputs,
                relations,
                len: InputsUsed(positions.len() as u8),
            });
            self.bindings.push((binding).clone());
            binding
        };
        slots[at] = Some(if let Some(previous) = slots[at].take() {
            Arc::new(PreparedViewGroup {
                primary: (previous.primary).clone(),
                secondary: Some(binding),
            })
        } else {
            Arc::new(PreparedViewGroup {
                primary: binding,
                secondary: None,
            })
        });
        Ok(())
    }
    fn intern_missing(
        &mut self,
        request: &ViewRequest,
        schemas: &[InputShape<'_>],
        error: OperatingError,
    ) -> Result<Arc<DerivedContract>, OperatingError> {
        if let Some(plan) = self.plans.iter().find(|plan| {
            StaticRequest(&plan.request) == StaticRequest(request)
                && plan.metadata == Err(error.clone())
                && plan.schemas.len() == schemas.len()
                && plan
                    .schemas
                    .iter()
                    .zip(schemas)
                    .all(|(prepared, actual)| prepared == actual.kinds)
        }) {
            return Ok((plan).clone());
        }
        let mut plan = super::prepare::view(self.semantic.clone(), request, schemas)?;
        plan.metadata = Err(error);
        let plan = Arc::new(plan);
        self.plans.push((plan).clone());
        Ok(plan)
    }
    pub(in crate::application::file_dag) fn intern(
        &mut self,
        request: &ViewRequest,
        schemas: &[InputShape<'_>],
    ) -> Result<Arc<DerivedContract>, OperatingError> {
        if let Some(plan) = self.plans.iter().find(|plan| {
            StaticRequest(&plan.request) == StaticRequest(request)
                && plan.schemas.len() == schemas.len()
                && plan
                    .schemas
                    .iter()
                    .zip(schemas)
                    .all(|(prepared, actual)| prepared == actual.kinds)
        }) {
            return Ok((plan).clone());
        }
        let plan = Arc::new(super::prepare::view(
            self.semantic.clone(),
            request,
            schemas,
        )?);
        self.plans.push((plan).clone());
        Ok(plan)
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn count(&self) -> types::PlanCount {
        types::PlanCount(self.plans.len())
    }
}

#[cfg(test)]
mod test;

pub(in crate::application::file_dag) mod types;

use ryo_derived::domain::request::types::DerivedKind;
