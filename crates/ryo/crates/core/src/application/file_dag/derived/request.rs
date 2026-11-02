//! Request-only interning. Runtime acquisition compares prepared IDs, never request metadata.
use crate::domain::{
    derived_views::{
        collections::views::registration::registered,
        service::{kind, slot},
        types::*,
    },
    operating::errors::OperatingError,
    operation_schema::types::InputSchema,
    operator::contract::{
        resources::{IndexColumns, ResourceProgram},
        types::OperatorContract,
    },
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};
use std::sync::Arc;
#[path = "request/identity.rs"]
mod identity;
use identity::same_request;
#[derive(Default)]
pub(in crate::application::file_dag) struct RequestViews {
    plans: Vec<Arc<DerivedContract>>,
    bindings: Vec<Arc<PreparedViewUse>>,
    logical: Vec<ryo_values::domain::plan_parts::types::Input>,
}
impl RequestViews {
    /// Bind finite semantic requirements once per compiled operator. No rows or cache are read here.
    pub(in crate::application::file_dag) fn bind(
        &mut self,
        contract: &mut OperatorContract,
    ) -> Result<(), OperatingError> {
        self.logical.clear();
        self.logical
            .extend_from_slice(ryo_values::domain::plans::service::named(&contract.inputs));
        let streaming = self.bind_mode(&contract.resources, &contract.input_kinds)?;
        let collected = self.bind_mode(&contract.collected_resources, &contract.input_kinds)?;
        contract.derived = DerivedSlots {
            streaming,
            collected,
        };
        Ok(())
    }

    fn bind_mode(
        &mut self,
        program: &ResourceProgram,
        kinds: &[Vec<ryo_language::domain::relations::types::CellKind>],
    ) -> Result<[Option<Arc<PreparedViewGroup>>; 12], OperatingError> {
        let mut slots: [Option<Arc<PreparedViewGroup>>; 12] = std::array::from_fn(|_| None);
        match program {
            ResourceProgram::Index { input, columns } => {
                let columns = match columns {
                    IndexColumns::Whole => (0..kinds.get(*input).map_or(0, Vec::len))
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
                    &[*input],
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
                        &[1],
                    )?;
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::PartitionOrder {
                            keys: reach.keys.clone(),
                        },
                        &[1],
                    )?;
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::ClosureAdjacency {
                            source: reach.source,
                            destination: reach.destination,
                        },
                        &[1],
                    )?;
                } else {
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::Equality {
                            columns: join.right_on.clone(),
                        },
                        &[1],
                    )?;
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::Equality {
                            columns: join.left_on.clone(),
                        },
                        &[0],
                    )?;
                    self.add(
                        &mut slots,
                        kinds,
                        ViewRequest::NativeLookup {
                            left_on: join.left_on.clone(),
                            right_on: join.right_on.clone(),
                            view: join.view.clone(),
                        },
                        &[0, 1],
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
                    &[1],
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
                    &[1],
                )?;
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::Membership {
                        column: axis.domain_id,
                    },
                    &[2, 1],
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
                    &[0],
                )?;
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::ClosureAdjacency {
                        source: *source,
                        destination: *destination,
                    },
                    &[0],
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
                    &[0],
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
                    &[0, 1],
                )?;
            }
            ResourceProgram::Dfs(program) => {
                self.add(
                    &mut slots,
                    kinds,
                    ViewRequest::OrderedDfs(program.clone()),
                    &[0, 1],
                )?;
            }
            ResourceProgram::None
            | ResourceProgram::Appended(_)
            | ResourceProgram::Rank(_)
            | ResourceProgram::Count { .. }
            | ResourceProgram::Digest { .. } => {}
        }
        Ok(slots)
    }

    fn add(
        &mut self,
        slots: &mut [Option<Arc<PreparedViewGroup>>; 12],
        kinds: &[Vec<ryo_language::domain::relations::types::CellKind>],
        request: ViewRequest,
        positions: &[usize],
    ) -> Result<(), OperatingError> {
        let mut schemas = Vec::with_capacity(positions.len());
        let mut missing = None;
        for position in positions {
            if let Some(kinds) = kinds.get(*position) {
                schemas.push(InputSchema { kinds, tree: None });
            } else {
                missing = Some(OperatingError::MissingInput {
                    position: *position + 1,
                });
                break;
            }
        }
        let at = slot(kind(&request));
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
        let mut inputs = [0; 2];
        inputs[..positions.len()].copy_from_slice(positions);
        let relations = std::array::from_fn(|at| {
            positions
                .get(at)
                .and_then(|position| self.logical.get(*position))
                .and_then(|input| match input {
                    ryo_values::domain::plan_parts::types::Input::Relation(relation) => {
                        Some(*relation)
                    }
                    ryo_values::domain::plan_parts::types::Input::Step(_) => None,
                })
        });
        let binding = self.bindings.iter().find(|binding| {
            Arc::ptr_eq(&binding.plan, &plan)
                && binding.inputs == inputs
                && binding.relations == relations
                && usize::from(binding.len) == positions.len()
        });
        let binding = if let Some(binding) = binding {
            Arc::clone(binding)
        } else {
            let binding = Arc::new(PreparedViewUse {
                plan,
                inputs,
                relations,
                len: positions.len() as u8,
            });
            self.bindings.push(Arc::clone(&binding));
            binding
        };
        slots[at] = Some(if let Some(previous) = slots[at].take() {
            Arc::new(PreparedViewGroup {
                primary: Arc::clone(&previous.primary),
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
        schemas: &[InputSchema<'_>],
        error: OperatingError,
    ) -> Result<Arc<DerivedContract>, OperatingError> {
        if let Some(plan) = self.plans.iter().find(|plan| {
            same_request(&plan.request, request)
                && plan.metadata == Err(error.clone())
                && plan.schemas.len() == schemas.len()
                && plan
                    .schemas
                    .iter()
                    .zip(schemas)
                    .all(|(prepared, actual)| prepared == actual.kinds)
        }) {
            return Ok(Arc::clone(plan));
        }
        let mut plan = registered(kind(request)).prepare(request, schemas)?;
        plan.metadata = Err(error);
        let plan = Arc::new(plan);
        self.plans.push(Arc::clone(&plan));
        Ok(plan)
    }
    pub(in crate::application::file_dag) fn intern(
        &mut self,
        request: &ViewRequest,
        schemas: &[InputSchema<'_>],
    ) -> Result<Arc<DerivedContract>, OperatingError> {
        if let Some(plan) = self.plans.iter().find(|plan| {
            same_request(&plan.request, request)
                && plan.schemas.len() == schemas.len()
                && plan
                    .schemas
                    .iter()
                    .zip(schemas)
                    .all(|(prepared, actual)| prepared == actual.kinds)
        }) {
            return Ok(Arc::clone(plan));
        }
        let plan = Arc::new(registered(kind(request)).prepare(request, schemas)?);
        self.plans.push(Arc::clone(&plan));
        Ok(plan)
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn count(&self) -> usize {
        self.plans.len()
    }
}

#[cfg(test)]
#[path = "request/tests.rs"]
mod tests;
