//! Canonical metadata from an admitted original row view; no source admission or row scan here.
use crate::application::file_dag::workspace::GraphInput;
use crate::domain::stored_graph::types::{self as graph, GraphIdentity};
use crate::domain::table_model::{service::count_of, types::Rows};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;
/// Prove an eligible physical population and compose selected columns before sharing graph data.
pub(super) fn source(
    input: &GraphInput<'_>,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
    eligible: &[ryo_relations::domain::relations::types::Relation],
) -> Option<graph::GraphSource> {
    let (population, columns) = population(input, eligible, false)?;
    let mapped = |column: ColumnIndex| columns.get(column.0 as usize).copied();
    let first = mapped(endpoints[0])?;
    let second = mapped(endpoints[1])?;
    let forward = first <= second;
    Some(graph::GraphSource {
        identity: GraphIdentity {
            population,
            endpoints: if forward {
                [first, second]
            } else {
                [second, first]
            },
            keys: keys
                .iter()
                .map(|column| mapped(*column))
                .collect::<Option<_>>()?,
        },
        endpoints: if forward {
            endpoints
        } else {
            [endpoints[1], endpoints[0]]
        },
        orientation: if forward {
            graph::Orientation::Forward
        } else {
            graph::Orientation::Reverse
        },
    })
}

/// Exact domain provenance when stable; computed domains remain invocation-local.
pub(super) fn domain_identity(
    input: &GraphInput<'_>,
    column: ColumnIndex,
    eligible: &[ryo_relations::domain::relations::types::Relation],
) -> Option<graph::DomainIdentity> {
    let (population, columns) = population(input, eligible, true)?;
    Some(graph::DomainIdentity {
        population,
        column: *columns.get(column.0 as usize)?,
    })
}

/// Rows never acquire identity from pointers or cardinality alone; native and semantic populations differ.
fn population(
    input: &GraphInput<'_>,
    eligible: &[ryo_relations::domain::relations::types::Relation],
    native: bool,
) -> Option<(graph::Population, Vec<ColumnIndex>)> {
    use crate::domain::table_cells::types::TreeForm;
    use ryo_relations::domain::relations::{service::kinds, types::Relation};
    let relation = input.relation()?;
    if !eligible.contains(&relation) {
        return None;
    }
    let rows = input.rows();
    let (relation, kinds, count, native_file, selected, positions, columns) = match rows {
        Rows::Stored(view) => (
            relation,
            view.kinds,
            count_of(rows).0,
            None,
            None,
            None,
            (0..view.kinds.len())
                .map(|column| ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)))
                .collect(),
        ),
        Rows::Selected(view) => {
            let count = view
                .source
                .columns
                .first()
                .map_or(0, |column| column.0.len());
            (
                relation,
                view.source.kinds,
                count,
                None,
                Some(Arc::clone(&view.selection.identity)),
                Some(Arc::clone(&view.selection.rows)),
                view.selection.columns.clone(),
            )
        }
        Rows::Tree(tree) if native => {
            let native_relation = match tree.form {
                TreeForm::Elements => Relation::SyntaxTree,
                TreeForm::Edges => Relation::SyntaxChild,
            };
            if relation != native_relation {
                return None;
            }
            let schema = kinds(relation);
            (
                relation,
                schema,
                count_of(rows).0,
                Some(tree.file),
                None,
                None,
                (0..schema.len())
                    .map(|column| ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)))
                    .collect(),
            )
        }
        Rows::Tree(_) | Rows::Computed(_) | Rows::Compact(_) | Rows::Positioned(_) => return None,
    };
    Some((
        graph::Population {
            relation,
            kinds,
            rows: count,
            native_file,
            selected,
            positions,
        },
        columns,
    ))
}
