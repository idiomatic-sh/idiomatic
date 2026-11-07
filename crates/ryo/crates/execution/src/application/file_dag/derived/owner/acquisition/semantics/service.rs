//! Adapt already-admitted finite physical data; all semantic builds cross the injected port.
use crate::domain::derived_views::types::{
    contract::value::DerivedContract, DerivedInputs, DerivedScratch, ViewRequest,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{
    derived::portable_fault, failure::FailureLatch, 
};
use crate::domain::table_model::types::representation_input::value::RepresentationInput as BorrowedRows;
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::{self as semantic};
use ryo_language::domain::symbols::types::ByteSlice;
use std::{cell::RefCell, sync::Arc};

use super::semantic_rows::{AdmittedRows, CanonicalTexts, Predicate};
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
        DerivedInputs::RawWords(keys) => port.build(
            &plan.semantic,
            semantic::DerivedInputs::RawWords(&keys.0),
            &mut scratch.semantic,
            recycled,
        ),
        DerivedInputs::Rows(inputs) => {
            if let ViewRequest::NativeLookup { .. } = &plan.request {
                let BorrowedRows::Tree(tree) = first(inputs, InputPosition(1))? else {
                    return Err(OperatingError::WrongOperation);
                };
                let form = match tree.form {
                    crate::domain::table_cells::types::TreeForm::Elements => {
                        semantic::NativeForm::Elements
                    }
                    crate::domain::table_cells::types::TreeForm::Edges => {
                        semantic::NativeForm::Edges
                    }
                };
                port.build(
                    &plan.semantic,
                    semantic::DerivedInputs::Native(form),
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
                    semantic::DerivedInputs::Rows(&source),
                    &mut scratch.semantic,
                    recycled,
                )
            }
        }
        DerivedInputs::Predicate {
            rows,
            candidates,
            program,
            symbols,
        } => {
            let mut predicate = Predicate {
                rows,
                program,
                symbols,
                needed: &plan.needed,
                row: &mut scratch.row,
                failure: &failure,
            };
            port.build(
                &plan.semantic,
                semantic::DerivedInputs::Predicate {
                    rows: &mut predicate,
                    candidates,
                },
                &mut scratch.semantic,
                recycled,
            )
        }
        DerivedInputs::Distinct { rows, positions } => {
            let source = AdmittedRows {
                rows,
                failure: &failure,
            };
            port.build(
                &plan.semantic,
                semantic::DerivedInputs::Distinct {
                    rows: &source,
                    positions,
                },
                &mut scratch.semantic,
                recycled,
            )
        }
        DerivedInputs::Postings {
            rows,
            texts,
            active,
        } => {
            let source = AdmittedRows {
                rows,
                failure: &failure,
            };
            port.build(
                &plan.semantic,
                semantic::DerivedInputs::Postings {
                    rows: &source,
                    symbols: texts.symbols,
                    source: ByteSlice(texts.source.0),
                    active,
                },
                &mut scratch.semantic,
                recycled,
            )
        }
        DerivedInputs::Text { rows, texts } => {
            let left = AdmittedRows {
                rows: first(rows, InputPosition(0))?,
                failure: &failure,
            };
            match &plan.request {
                ViewRequest::CandidatePostings { literals, .. } => port.build(
                    &plan.semantic,
                    semantic::DerivedInputs::Postings {
                        rows: &left,
                        symbols: texts.symbols,
                        source: ByteSlice(texts.source.0),
                        active: literals,
                    },
                    &mut scratch.semantic,
                    recycled,
                ),
                ViewRequest::OrderedKeys { .. } => {
                    let right = AdmittedRows {
                        rows: first(rows, InputPosition(1))?,
                        failure: &failure,
                    };
                    port.build(
                        &plan.semantic,
                        semantic::DerivedInputs::Ordered {
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
                    semantic::MembershipSpace::Values
                }
                crate::domain::derived_views::types::MembershipSpace::Dense(values) => {
                    semantic::MembershipSpace::Dense(values)
                }
                crate::domain::derived_views::types::MembershipSpace::Native(tree) => {
                    semantic::MembershipSpace::Native {
                        file: tree.file,
                        elements: tree.elements.kind.len(),
                    }
                }
            };
            port.build(
                &plan.semantic,
                semantic::DerivedInputs::Membership {
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
                semantic::DerivedInputs::Partition {
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
                semantic::DerivedInputs::Dfs {
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
                port.prepare(
                    semantic::ViewRequest::Adjacency {
                        endpoints: source.endpoints,
                        keys: keys.clone(),
                    },
                    plan.schemas.clone(),
                )
            };
            built_plan = Arc::clone(&prepared);
            port.build(
                &prepared,
                semantic::DerivedInputs::Rows(&rows),
                &mut scratch.semantic,
                recycled,
            )
        }
    };
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error);
    }
    match answer.map_err(portable_fault)? {
        semantic::BuildOutcome::Built(payload) => Ok(Some(DerivedStorage::new(
            port,
            built_plan,
            payload,
            graph_identity,
        ))),
        semantic::BuildOutcome::Declined => Ok(None),
    }
}
