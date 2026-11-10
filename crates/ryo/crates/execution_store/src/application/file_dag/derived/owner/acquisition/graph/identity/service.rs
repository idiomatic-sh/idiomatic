//! Canonical metadata from an admitted original row view; no source admission or row scan here.
use super::types::{PopulationScope, ProvenPopulation};
use crate::application::file_dag::workspace::GraphInput;
use ryo_execution_domain::domain::stored_graph::types::{self as graph, GraphIdentity};
use ryo_execution_domain::domain::table_model::{service::count_of, types::Rows};
use ryo_values::domain::plan_parts::types::ColumnIndex;
/// Prove an eligible physical population and compose selected columns before sharing graph data.
pub(crate) fn source(
    input: &GraphInput<'_>,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
    eligible: &[ryo_relations::domain::relations::types::Relation],
) -> Option<graph::GraphSource> {
    let ProvenPopulation {
        population,
        columns,
    } = population(input, eligible, PopulationScope::Stored)?;
    let mapped =
        |column: ColumnIndex| usize::try_from(column.0).map_or(None, |at| columns.get(at).copied());
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
pub(crate) fn domain_identity(
    input: &GraphInput<'_>,
    column: ColumnIndex,
    eligible: &[ryo_relations::domain::relations::types::Relation],
) -> Option<graph::DomainIdentity> {
    let ProvenPopulation {
        population,
        columns,
    } = population(input, eligible, PopulationScope::Native)?;
    Some(graph::DomainIdentity {
        population,
        column: *usize::try_from(column.0).map_or(None, |at| columns.get(at))?,
    })
}

/// Rows never acquire identity from pointers or cardinality alone; native and semantic populations differ.
fn population(
    input: &GraphInput<'_>,
    eligible: &[ryo_relations::domain::relations::types::Relation],
    scope: PopulationScope,
) -> Option<ProvenPopulation> {
    use ryo_execution_domain::domain::table_cells::types::TreeForm;
    use ryo_relations::domain::relations::{service::kinds, types::Relation};
    let relation = input.relation()?;
    if !eligible.contains(&relation) {
        return None;
    }
    let rows = input.rows();
    let (relation, kinds, count, native_file, columns) = match rows {
        Rows::Stored(view) => (
            relation,
            view.kinds,
            count_of(rows),
            None,
            (0..view.kinds.len())
                .map(|column| ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)))
                .collect(),
        ),
        Rows::Tree(tree) if scope == PopulationScope::Native => {
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
                count_of(rows),
                Some(tree.file),
                (0..schema.len())
                    .map(|column| ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)))
                    .collect(),
            )
        }
        Rows::Tree(_) | Rows::Computed(_) | Rows::Compact(_) => return None,
    };
    Some(ProvenPopulation {
        population: graph::RelationPopulation {
            relation,
            kinds,
            rows: count,
            native_file,
        },
        columns,
    })
}
