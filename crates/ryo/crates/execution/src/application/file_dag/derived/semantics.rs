//! Adapt already-admitted finite physical data; all semantic builds cross the injected port.
use super::storage::DerivedStorage;
use crate::domain::derived_views::types::{
    DerivedContract, DerivedInputs, DerivedScratch, ViewRequest,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{
    derived::portable_fault, failure::FailureLatch, views::BorrowedRows,
};
use {crate::domain::table_texts::service::datum, crate::domain::table_texts::service::name_text, crate::domain::table_texts::service::symbol_text, crate::domain::table_texts::types::Datum, ryo_values::domain::files::types::SourceView, crate::domain::table_texts::types::Texts};
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::{self as semantic, RowCells};
use ryo_language::domain::symbols::{
    service::query_intern_derived,
    types::{ByteSlice, QuerySymbols, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CellKind, Value},
    terms::types::NameText,
};
use ryo_values::domain::{plan_parts::types::ColumnIndex, rows::types::RowIndex};
use std::{cell::RefCell, sync::Arc};

struct Rows<'a, 'f> {
    rows: BorrowedRows<'f>,
    failure: &'a RefCell<FailureLatch>,
}
impl RowCells for Rows<'_, '_> {
    fn bulk(&self) -> Option<semantic::BulkRows<'_>> {
        if self.failure.borrow().failed() {
            return None;
        }
        let rows = self.failure.borrow_mut().run(|| self.rows.bulk()).ok()?;
        match rows {
            BorrowedRows::Stored(source) => Some(semantic::BulkRows::Stored(source)),
            BorrowedRows::Selected {
                source,
                positions,
                columns,
                ..
            } => Some(semantic::BulkRows::Selected {
                source,
                positions,
                columns,
            }),
            BorrowedRows::Computed(table) => Some(semantic::BulkRows::Computed(table)),
            BorrowedRows::Tree(_) | BorrowedRows::Compact(_) | BorrowedRows::Positioned(_) => None,
        }
    }
    fn kinds(&self) -> &[CellKind] {
        self.rows.kinds()
    }
    fn count(&self) -> usize {
        self.rows.count()
    }
    fn cell(&self, row: usize, column: ColumnIndex) -> Option<Value> {
        if self.failure.borrow().failed() {
            return None;
        }
        self.rows.cell(RowIndex(row), column)
    }
    fn validate(&self) -> Result<(), semantic::DerivedFault> {
        self.failure
            .borrow_mut()
            .run(|| self.rows.validate())
            .map_err(|()| semantic::DerivedFault::Interrupted)
    }
    fn read_row<'a>(&'a self, row: usize, buffer: &'a mut Vec<Value>) -> Option<&'a [Value]> {
        if self.failure.borrow().failed() {
            return None;
        }
        self.rows.read(RowIndex(row), buffer)
    }
}
struct Predicate<'a, 'f> {
    rows: BorrowedRows<'f>,
    program: &'a mut dyn crate::domain::operator::contract::types::PredicateProgram,
    symbols: &'a Symbols,
    needed: &'a crate::domain::table_cells::types::ColumnPlaces,
    row: &'a mut Vec<Value>,
    failure: &'a RefCell<FailureLatch>,
}
impl semantic::PredicateRows for Predicate<'_, '_> {
    fn count(&self) -> usize {
        self.rows.count()
    }
    fn holds(&mut self, at: usize) -> Result<bool, semantic::DerivedFault> {
        let Self {
            rows,
            program,
            symbols,
            needed,
            row,
            failure,
        } = self;
        failure
            .borrow_mut()
            .run(|| {
                let row = if let BorrowedRows::Stored(source) = rows {
                    row.resize(
                        source.kinds.len(),
                        Value::Flag(ryo_relations::domain::relations::types::Flag::Lowered),
                    );
                    if crate::domain::table_cells::service::read_cells(
                        source,
                        RowIndex(at),
                        row,
                        needed,
                    ) != crate::domain::table_cells::types::CellsRead::Read
                    {
                        return Err(OperatingError::ShapeMismatch);
                    }
                    row.as_slice()
                } else {
                    rows.read(RowIndex(at), row)
                        .ok_or(OperatingError::ShapeMismatch)?
                };
                Ok(program.holds(row, symbols)?
                    == crate::domain::table_model::types::Verdict::Holds)
            })
            .map_err(|()| semantic::DerivedFault::Interrupted)
    }
}
impl semantic::ValueTexts for Texts<'_> {
    fn text(&self, value: Value) -> Option<&str> {
        match datum(value, self) {
            Datum::Text(text) => Some(text.0),
            _ => None,
        }
    }
}
struct CanonicalTexts<'a, 's> {
    failure: &'a RefCell<FailureLatch>,
    symbols: &'a mut QuerySymbols<'s>,
    source: SourceView<'a>,
    paths: &'a [ryo_language::domain::files::types::FilePath],
    arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
}
impl semantic::ValueTexts for CanonicalTexts<'_, '_> {
    fn text(&self, value: Value) -> Option<&str> {
        if self.failure.borrow().failed() {
            return None;
        }
        let texts = Texts {
            symbols: self.symbols.as_ref(),
            source: self.source,
            paths: self.paths,
            arenas: self.arenas,
        };
        match value {
            Value::Symbol(symbol) => symbol_text(&texts, symbol).map(|text| text.0),
            Value::Name(name) => name_text(&texts, name).map(|text| text.0),
            _ => None,
        }
    }
}
impl semantic::CanonicalTexts for CanonicalTexts<'_, '_> {
    fn intern(&mut self, text: &str) -> Result<Value, semantic::DerivedFault> {
        let Self {
            failure,
            symbols,
            source,
            ..
        } = self;
        failure
            .borrow_mut()
            .run(|| {
                Ok(Value::Symbol(query_intern_derived(
                    symbols,
                    ByteSlice(source.0),
                    NameText(text),
                )))
            })
            .map_err(|()| semantic::DerivedFault::Interrupted)
    }
}

fn first<'a>(rows: &[BorrowedRows<'a>], at: usize) -> Result<BorrowedRows<'a>, OperatingError> {
    rows.get(at)
        .copied()
        .ok_or(OperatingError::MissingInput { position: at + 1 })
}

/// Decline is a readable unsupported strategy, not an error or optional-retention decision.
pub(in crate::application::file_dag::derived) fn build(
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
                let BorrowedRows::Tree(tree) = first(inputs, 1)? else {
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
                let source = Rows {
                    rows: first(inputs, 0)?,
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
            let source = Rows {
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
            let source = Rows {
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
            let left = Rows {
                rows: first(rows, 0)?,
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
                    let right = Rows {
                        rows: first(rows, 1)?,
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
            let source = Rows {
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
            let source = Rows {
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
            let roots = Rows {
                rows: roots,
                failure: &failure,
            };
            let edges = Rows {
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
            let rows = Rows {
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
