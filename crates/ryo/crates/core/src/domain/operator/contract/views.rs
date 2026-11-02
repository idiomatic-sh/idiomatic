//! Read-only physical inputs expose no owned cache or retained selection handle.
use crate::domain::table_cells::types::{RowIndex, TreeRows};
use crate::domain::table_model::types::{KeyIndex, Rows, Table};
use ryo_language::domain::relations::types::{CellKind, ColumnsView, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
pub(crate) enum BorrowedRows<'a> {
    Stored(ColumnsView<'a>),
    Selected {
        source: ColumnsView<'a>,
        positions: &'a [u32],
        columns: &'a [ColumnIndex],
        kinds: &'a [CellKind],
    },
    Tree(TreeRows<'a>),
    Computed(&'a Table),
}
impl<'a> From<&'a Rows<'a>> for BorrowedRows<'a> {
    fn from(rows: &'a Rows<'a>) -> Self {
        match rows {
            Rows::Stored(view) => Self::Stored(*view),
            Rows::Selected(view) => Self::Selected {
                source: view.source,
                positions: &view.selection.rows,
                columns: &view.selection.columns,
                kinds: &view.selection.kinds,
            },
            Rows::Tree(tree) => Self::Tree(*tree),
            Rows::Computed(table) => Self::Computed(table),
        }
    }
}
impl<'f> BorrowedRows<'f> {
    /// Admission proof for a complete typed population. Invalid declared rows
    /// cannot be silently published as a successful smaller population.
    pub(crate) fn validate(self) -> Result<(), crate::domain::operating::errors::OperatingError> {
        let valid = match self {
            Self::Stored(view) => crate::domain::table_model::service::rectangular(view),
            Self::Computed(table) => {
                table.count.0.checked_mul(table.kinds.len()) == Some(table.cells.len())
            }
            Self::Tree(_) => true,
            Self::Selected {
                source,
                positions,
                columns,
                kinds,
            } => {
                crate::domain::table_model::service::rectangular(source)
                    && columns.len() == kinds.len()
                    && columns
                        .iter()
                        .zip(kinds)
                        .all(|(column, kind)| source.kinds.get(column.0 as usize) == Some(kind))
                    && positions.iter().all(|position| {
                        (*position as usize)
                            < source.columns.first().map_or(0, |column| column.0.len())
                    })
            }
        };
        if valid {
            Ok(())
        } else {
            Err(crate::domain::operating::errors::OperatingError::ShapeMismatch)
        }
    }
    pub(crate) fn count(self) -> usize {
        match self {
            Self::Stored(view) => view.columns.first().map_or(0, |column| column.0.len()),
            Self::Selected { positions, .. } => positions.len(),
            Self::Tree(tree) => crate::domain::table_model::service::count_of(&Rows::Tree(tree)).0,
            Self::Computed(table) => table.count.0,
        }
    }
    pub(crate) fn kinds(&self) -> &[CellKind] {
        match self {
            Self::Stored(view) => view.kinds,
            Self::Selected { kinds, .. } => kinds,
            Self::Tree(tree) => ryo_language::domain::relations::service::kinds(match tree.form {
                crate::domain::table_cells::types::TreeForm::Elements => {
                    ryo_language::domain::relations::types::Relation::SyntaxTree
                }
                crate::domain::table_cells::types::TreeForm::Edges => {
                    ryo_language::domain::relations::types::Relation::SyntaxChild
                }
            }),
            Self::Computed(table) => &table.kinds,
        }
    }
    pub(crate) fn cell(&self, row: RowIndex, column: ColumnIndex) -> Option<Value> {
        use ryo_language::domain::relations::{service::from_bits, types::Bits};
        let index = column.0 as usize;
        match self {
            Self::Stored(view) => Some(from_bits(
                *view.kinds.get(index)?,
                Bits(*view.columns.get(index)?.0.get(row.0)?),
            )),
            Self::Selected {
                source,
                positions,
                columns,
                ..
            } => {
                let physical = *positions.get(row.0)? as usize;
                let index = columns.get(index)?.0 as usize;
                Some(from_bits(
                    *source.kinds.get(index)?,
                    Bits(*source.columns.get(index)?.0.get(physical)?),
                ))
            }
            Self::Computed(table) => crate::domain::table_model::service::row_in(table, row)?
                .get(index)
                .copied(),
            Self::Tree(tree) => {
                use crate::domain::table_cells::types::TreeForm;
                let place =
                    u32::try_from(row.0 + usize::from(tree.form == TreeForm::Edges)).ok()?;
                if place as usize >= tree.elements.kind.len() {
                    return None;
                }
                let at = ryo_language::domain::relations::types::Position(place);
                let values = match tree.form {
                    TreeForm::Elements => crate::domain::table_cells::service::element_of(tree, at),
                    TreeForm::Edges => crate::domain::table_cells::service::edge_of(tree, at),
                };
                values.get(index).copied()
            }
        }
    }
    pub(crate) fn read<'a>(&self, row: RowIndex, buffer: &'a mut Vec<Value>) -> Option<&'a [Value]>
    where
        'f: 'a,
    {
        buffer.clear();
        match self {
            Self::Stored(view) => {
                for (kind, column) in view.kinds.iter().zip(view.columns) {
                    buffer.push(ryo_language::domain::relations::service::from_bits(
                        *kind,
                        ryo_language::domain::relations::types::Bits(*column.0.get(row.0)?),
                    ));
                }
            }
            Self::Selected {
                source,
                positions,
                columns,
                ..
            } => {
                let physical = *positions.get(row.0)? as usize;
                for column in *columns {
                    let index = column.0 as usize;
                    buffer.push(ryo_language::domain::relations::service::from_bits(
                        *source.kinds.get(index)?,
                        ryo_language::domain::relations::types::Bits(
                            *source.columns.get(index)?.0.get(physical)?,
                        ),
                    ));
                }
            }
            Self::Tree(tree) => {
                crate::domain::table_model::service::read_row(&Rows::Tree(*tree), row, buffer)?;
            }
            Self::Computed(table) => {
                return crate::domain::table_model::service::row_in(*table, row);
            }
        }
        Some(buffer)
    }
}

/// Exact typed key lookup prepared by the engine; primitives only probe it.
#[derive(Clone, Copy)]
pub(crate) struct IndexedRows<'a> {
    pub rows: BorrowedRows<'a>,
    pub columns: &'a [ColumnIndex],
    pub index: &'a KeyIndex,
}
impl IndexedRows<'_> {
    pub(crate) fn contains(&self, key: &[Value]) -> bool {
        self.matching(key).next().is_some()
    }
    pub(crate) fn matching<'a>(&'a self, key: &'a [Value]) -> MatchingRows<'a> {
        let hash = crate::domain::table_model::service::hash_key(key.iter().map(Some));
        let bucket =
            usize::try_from(hash.0).unwrap_or(0) & self.index.heads.len().saturating_sub(1);
        MatchingRows {
            view: self,
            key,
            hash,
            at: self
                .index
                .heads
                .get(bucket)
                .copied()
                .unwrap_or(crate::domain::table_model::types::ChainLink(0)),
        }
    }
}
pub(crate) struct MatchingRows<'a> {
    view: &'a IndexedRows<'a>,
    key: &'a [Value],
    hash: crate::domain::table_model::types::KeyCellsHash,
    at: crate::domain::table_model::types::ChainLink,
}
impl Iterator for MatchingRows<'_> {
    type Item = RowIndex;
    fn next(&mut self) -> Option<RowIndex> {
        use crate::domain::table_model::types::ChainLink;
        while self.at.0 != 0 {
            let position = usize::try_from(self.at.0 - 1).unwrap_or(usize::MAX);
            self.at = self
                .view
                .index
                .next
                .get(position)
                .copied()
                .unwrap_or(ChainLink(0));
            let row = self.view.index.rows.get(position).copied()?;
            if self.view.index.hashes.get(position) == Some(&self.hash)
                && self
                    .view
                    .columns
                    .iter()
                    .zip(self.key)
                    .all(|(column, expected)| self.view.rows.cell(row, *column) == Some(*expected))
            {
                return Some(row);
            }
        }
        None
    }
}
