//! Adapt already-admitted finite physical data; all semantic builds cross the injected port.
use crate::domain::derived_views::types::{
    contract::value::DerivedContract, DerivedInputs, DerivedScratch, ViewRequest,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{
    derived::portable_fault, failure::FailureLatch, 
};
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use crate::outbound::derived::interface::DerivedPort;
use std::{cell::RefCell, sync::Arc};

use super::semantic_rows::{AdmittedRows, CanonicalTexts};
use super::storage::DerivedStorage;
use super::types::InputPosition;

fn first<'a>(
    rows: &[BorrowedRows<'a>],
    at: InputPosition,
) -> Result<BorrowedRows<'a>, OperatingError> {
    rows.get(at.0)
        .copied()
        .ok_or(OperatingError::MissingInput { position: at.0 + 1 })
}

/// Decline is a readable unsupported strategy, not an error or optional-retention decision.
pub(in crate::application::file_dag::derived::owner::acquisition) fn build(
    port: Arc<dyn DerivedPort>,
    plan: &DerivedContract,
    inputs: DerivedInputs<'_, '_>,
    scratch: &mut DerivedScratch,
    recycled: Option<DerivedStorage>,
) -> Result<Option<DerivedStorage>, OperatingError> {
    plan.metadata.as_ref().map_err(Clone::clone)?;
    let failure = RefCell::new(FailureLatch::default());
    let recycled = recycled.and_then(DerivedStorage::into_recycled);
    let mut graph_identity = None;
    let mut built_plan = Arc::clone(&plan.semantic);
    let answer = match inputs {
        DerivedInputs::Rows(inputs) => {
            if let ViewRequest::NativeLookup { .. } = &plan.request {
                let BorrowedRows::Tree(tree) = first(inputs, InputPosition(1))? else {
                    return Err(OperatingError::WrongOperation);
                };
                let form = match tree.form {
                    crate::domain::table_cells::types::TreeForm::Elements => {
                        ryo_derived::domain::native::types::NativeForm::Elements
                    }
                    crate::domain::table_cells::types::TreeForm::Edges => {
                        ryo_derived::domain::native::types::NativeForm::Edges
                    }
                };
                port.build(
                    &plan.semantic,
                    ryo_derived::domain::contract::types::BuildInputs::Native(form),
                    &mut scratch.semantic,
                    recycled,
                )
            } else {
                let source = AdmittedRows {
                    rows: first(inputs, InputPosition(0))?,
                    failure: &failure,
                };
                port.build(
                    &plan.semantic,
                    ryo_derived::domain::contract::types::BuildInputs::Rows(&source),
                    &mut scratch.semantic,
                    recycled,
                )
            }
        }
        DerivedInputs::Text { rows, texts } => {
            let left = AdmittedRows {
                rows: first(rows, InputPosition(0))?,
                failure: &failure,
            };
            match &plan.request {
                ViewRequest::OrderedKeys { .. } => {
                    let right = AdmittedRows {
                        rows: first(rows, InputPosition(1))?,
                        failure: &failure,
                    };
                    port.build(
                        &plan.semantic,
                        ryo_derived::domain::contract::types::BuildInputs::Ordered {
                            left: &left,
                            right: &right,
                            texts,
                        },
                        &mut scratch.semantic,
                        recycled,
                    )
                }
                _ => return Err(OperatingError::WrongOperation),
            }
        }
        DerivedInputs::Membership { rows, space } => {
            let source = AdmittedRows {
                rows,
                failure: &failure,
            };
            let space = match space {
                crate::domain::derived_views::types::MembershipSpace::Values => {
                    ryo_derived::domain::membership::types::MarkSpace::Values
                }
                crate::domain::derived_views::types::MembershipSpace::Dense(values) => {
                    ryo_derived::domain::membership::types::MarkSpace::Dense(values)
                }
                crate::domain::derived_views::types::MembershipSpace::Native(tree) => {
                    ryo_derived::domain::membership::types::MarkSpace::Native {
                        file: tree.file,
                        elements: ryo_derived::domain::membership::types::MarkCount(
                            tree.elements.kind.len(),
                        ),
                    }
                }
            };
            port.build(
                &plan.semantic,
                ryo_derived::domain::contract::types::BuildInputs::Membership {
                    rows: &source,
                    space,
                },
                &mut scratch.semantic,
                recycled,
            )
        }
        DerivedInputs::Partition {
            rows,
            positions,
            key,
        } => {
            let source = AdmittedRows {
                rows,
                failure: &failure,
            };
            port.build(
                &plan.semantic,
                ryo_derived::domain::contract::types::BuildInputs::Partition {
                    rows: &source,
                    positions: Some(positions),
                    key,
                },
                &mut scratch.semantic,
                recycled,
            )
        }
        DerivedInputs::Dfs {
            roots,
            edges,
            symbols,
            source,
            paths,
            arenas,
        } => {
            let roots = AdmittedRows {
                rows: roots,
                failure: &failure,
            };
            let edges = AdmittedRows {
                rows: edges,
                failure: &failure,
            };
            let mut texts = CanonicalTexts {
                failure: &failure,
                symbols,
                source,
                paths,
                arenas,
            };
            port.build(
                &plan.semantic,
                ryo_derived::domain::contract::types::BuildInputs::Dfs {
                    roots: &roots,
                    edges: &edges,
                    texts: &mut texts,
                },
                &mut scratch.semantic,
                recycled,
            )
        }
        DerivedInputs::Adjacency { rows, source } => {
            let ViewRequest::Adjacency { endpoints, keys } = &plan.request else {
                return Err(OperatingError::WrongOperation);
            };
            let expected = match source.orientation {
                crate::domain::stored_graph::types::Orientation::Forward => *endpoints,
                crate::domain::stored_graph::types::Orientation::Reverse => {
                    [endpoints[1], endpoints[0]]
                }
            };
            if source.endpoints != expected {
                return Err(OperatingError::ShapeMismatch);
            }
            graph_identity = Some(Arc::new(source.identity));
            let rows = AdmittedRows {
                rows,
                failure: &failure,
            };
            // Identity/proof is engine-only; canonical endpoint columns are a plain semantic request.
            let prepared = if source.endpoints == *endpoints {
                Arc::clone(&plan.semantic)
            } else {
                Arc::new(port.prepare(
                    ryo_derived::domain::request::types::DerivedRequest::Adjacency {
                        endpoints: source.endpoints,
                        keys: keys.clone(),
                    },
                    plan.schemas.clone(),
                ))
            };
            built_plan = Arc::clone(&prepared);
            port.build(
                &prepared,
                ryo_derived::domain::contract::types::BuildInputs::Rows(&rows),
                &mut scratch.semantic,
                recycled,
            )
        }
    };
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error);
    }
    match answer.map_err(portable_fault)? {
        ryo_derived::domain::contract::types::BuildOutcome::Built(payload) => Ok(Some(DerivedStorage::new(
            port,
            built_plan,
            payload,
            graph_identity,
        ))),
        ryo_derived::domain::contract::types::BuildOutcome::Declined => Ok(None),
    }
}
