//! The `presence` service: one fold over the deduplicated request graph gives every node a presence formula, a
//! necessary condition for the node to have a row in one file, and a skip formula, which also holds when a row-fallible
//! operator upstream could have rows to fail on. No rows, IO or ports.
use super::narrowing::{formula, test};
use super::types::{
    Certification, ColumnOrigin, Conditionality, FoldedNode, FormulaId, GraphPresence, NodeFacts, OccurrenceId,
    PresenceFormula, PresenceTable, PresenceTables, PresenceTest, ReadId, RegionNode, RelationScope, RelationTests,
    SideOccurrence, TestId,
};
use crate::domain::file_program::types::{NodeInput, RequestGraph, SchemaValidity};
use crate::domain::operation_schema::projection::{output_origins, tree_output};
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::presence::{InputMask, PresenceMask, RowFailure};
use ryo_relations::domain::relations::types::{Provenance, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepScope};
use std::collections::HashMap;

/// The formula every table starts with.
pub const ALWAYS: FormulaId = FormulaId(0);

fn tables() -> PresenceTables {
    PresenceTables {
        tests: Vec::new(),
        test_buckets: HashMap::new(),
        formulas: vec![PresenceFormula::Always],
        formula_index: HashMap::from([(PresenceFormula::Always, ALWAYS)]),
        reads: Vec::new(),
        occurrences: HashMap::new(),
        next_occurrence: OccurrenceId(0),
    }
}
/// Every part holds; `Always` parts drop out.
fn all(tables: &mut PresenceTables, parts: impl IntoIterator<Item = FormulaId>) -> FormulaId {
    let mut kept = Vec::new();
    for part in parts {
        if part != ALWAYS && !kept.contains(&part) {
            kept.push(part);
        }
    }
    match kept.as_slice() {
        [] => ALWAYS,
        [one] => *one,
        _ => formula(tables, PresenceFormula::All(kept.into_boxed_slice())),
    }
}
/// At least one part holds; an `Always` part makes the whole `Always`, and so does no part at all, which only an operation
/// missing its inputs has.
fn any(tables: &mut PresenceTables, parts: impl IntoIterator<Item = FormulaId>) -> FormulaId {
    let mut kept = Vec::new();
    for part in parts {
        if part == ALWAYS {
            return ALWAYS;
        }
        if !kept.contains(&part) {
            kept.push(part);
        }
    }
    match kept.as_slice() {
        [] => ALWAYS,
        [one] => *one,
        _ => formula(tables, PresenceFormula::Any(kept.into_boxed_slice())),
    }
}
fn next_occurrence(tables: &mut PresenceTables) -> OccurrenceId {
    let id = tables.next_occurrence;
    tables.next_occurrence = OccurrenceId(id.0 + 1);
    id
}
fn read(tables: &mut PresenceTables, relation: Relation) -> ColumnOrigin {
    let read = ReadId(tables.reads.len());
    tables.reads.push(relation);
    ColumnOrigin {
        read,
        occurrence: next_occurrence(tables),
        column: ColumnIndex(0),
    }
}
/// The occurrence a side of a two-input node gives an upstream occurrence: one for each node, side and occurrence.
fn occurrence(tables: &mut PresenceTables, key: SideOccurrence) -> OccurrenceId {
    if let Some(found) = tables.occurrences.get(&key) {
        return *found;
    }
    let id = next_occurrence(tables);
    let _new = tables.occurrences.insert(key, id);
    id
}
/// The formulas of the inputs a mask names.
fn masked(mask: InputMask, presences: &[FormulaId]) -> Vec<FormulaId> {
    presences
        .iter()
        .enumerate()
        .filter(|(place, _)| *place < 8 && mask.0 & (1 << place) != 0)
        .map(|(_, presence)| *presence)
        .collect()
}
/// The presence and skip formulas of every node of the graph, folded once in node order, which is topological.
pub fn of_graph(
    graph: &RequestGraph,
    scope: &dyn Fn(Relation) -> RelationScope,
    matcher: crate::domain::matching::types::PatternMatcher<'_>,
) -> GraphPresence {
    let mut tables = tables();
    let mut facts: Vec<NodeFacts> = Vec::with_capacity(graph.nodes.len());
    let mut skips = Vec::with_capacity(graph.nodes.len());
    for (at, node) in graph.nodes.iter().enumerate() {
        let mut presences = Vec::with_capacity(node.inputs.len());
        let mut failures = Vec::new();
        let mut origins: Vec<Vec<Option<ColumnOrigin>>> = Vec::with_capacity(node.inputs.len());
        let mut trees = Vec::with_capacity(node.inputs.len());
        let mut reads: Vec<Relation> = Vec::new();
        let mut conditionality = if node.schema_valid == SchemaValidity::Invalid
            || node.needs.is_err()
            || node.phase == StepScope::CrossFile
        {
            Conditionality::Unconditional
        } else {
            Conditionality::Conditional
        };
        for (place, input) in node.inputs.iter().enumerate() {
            let width = node.contract.input_kinds.get(place).map_or(0, Vec::len);
            let upstream = match input {
                NodeInput::Node(id) => facts.get(id.0),
                NodeInput::Relation(_) | NodeInput::Observation(_) => None,
            };
            match (input, upstream) {
                (NodeInput::Relation(relation), _)
                    if scope(*relation) == RelationScope::CurrentFile
                        && matches!(
                            ryo_relations::domain::relations::service::provenance_of(*relation),
                            Provenance::Trees | Provenance::Engine
                        ) =>
                {
                    trees.push(crate::domain::table_cells::service::native_form(*relation));
                    reads.push(*relation);
                    presences.push(test(&mut tables, *relation, Vec::new()));
                    let first = read(&mut tables, *relation);
                    origins.push(
                        (0..width)
                            .map(|column| {
                                Some(ColumnOrigin {
                                    column: ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)),
                                    ..first
                                })
                            })
                            .collect(),
                    );
                }
                (NodeInput::Relation(relation), _) => {
                    trees.push(crate::domain::table_cells::service::native_form(*relation));
                    presences.push(ALWAYS);
                    origins.push(vec![None; width]);
                }
                (NodeInput::Node(_), Some(upstream)) => {
                    if upstream.conditionality == Conditionality::Unconditional {
                        conditionality = Conditionality::Unconditional;
                    }
                    presences.push(upstream.presence);
                    reads.extend_from_slice(&upstream.reads);
                    failures.extend(upstream.failure);
                    origins.push(upstream.origins.clone());
                    trees.push(upstream.tree);
                }
                (NodeInput::Node(_) | NodeInput::Observation(_), _) => {
                    conditionality = Conditionality::Unconditional;
                    presences.push(ALWAYS);
                    origins.push(vec![None; width]);
                    trees.push(None);
                }
            }
        }
        reads.sort_unstable();
        reads.dedup();
        let shapes: Vec<InputShape<'_>> = node
            .contract
            .input_kinds
            .iter()
            .zip(trees.iter().copied().chain(std::iter::repeat(None)))
            .map(|(kinds, tree)| InputShape { kinds, tree })
            .collect();
        let tree = tree_output(&node.operation, shapes.first().copied());
        if let RowFailure::OnRows(mask) = node.contract.failure {
            failures.extend(masked(mask, &presences));
        }
        if let Some(mask) = super::narrowing::invalid_patterns(&node.operation, matcher) {
            failures.extend(masked(mask, &presences));
        }
        let failure = (!failures.is_empty()).then(|| any(&mut tables, failures));
        if conditionality == Conditionality::Unconditional {
            facts.push(NodeFacts {
                presence: ALWAYS,
                failure: None,
                origins: Vec::new(),
                tree,
                conditionality,
                reads,
            });
            skips.push(ALWAYS);
            continue;
        }
        let base = match crate::domain::operator::contract::service::semantics(node.contract.kind).presence {
            PresenceMask::AllOf(mask) => all(&mut tables, masked(mask, &presences)),
            PresenceMask::AnyOf(mask) => any(&mut tables, masked(mask, &presences)),
            PresenceMask::Always => ALWAYS,
        };
        let narrowing = super::narrowing::node_narrowing(&mut tables, &node.operation, &origins);
        let presence = all(&mut tables, std::iter::once(base).chain(narrowing));
        let slices: Vec<&[Option<ColumnOrigin>]> = origins.iter().map(Vec::as_slice).collect();
        let output = output_origins(&node.operation, &shapes, &slices, &mut |side, origin| {
            origin.map(|origin| ColumnOrigin {
                occurrence: occurrence(
                    &mut tables,
                    SideOccurrence {
                        node: FoldedNode(at),
                        side,
                        upstream: origin.occurrence,
                    },
                ),
                ..origin
            })
        });
        let skip = match failure {
            Some(failure) => any(&mut tables, [presence, failure]),
            None => presence,
        };
        facts.push(NodeFacts {
            presence,
            failure,
            origins: output,
            tree,
            conditionality,
            reads,
        });
        skips.push(skip);
    }
    let PresenceTables { tests, formulas, .. } = tables;
    GraphPresence {
        table: PresenceTable {
            relations: by_relation(&tests),
            tests: tests.into_boxed_slice(),
            formulas: formulas.into_boxed_slice(),
            regions: Box::new([]),
            reads: Box::new([]),
        },
        nodes: skips.into_boxed_slice(),
        reads: facts.into_iter().map(|node| node.reads.into_boxed_slice()).collect(),
    }
}

/// The tests of each relation, in the order the relations are first tested.
fn by_relation(tests: &[PresenceTest]) -> Box<[RelationTests]> {
    let mut grouped: Vec<RelationTests> = Vec::new();
    for (at, test) in tests.iter().enumerate() {
        match grouped.iter_mut().find(|group| group.relation == test.relation) {
            Some(group) => {
                let mut members = std::mem::take(&mut group.tests).into_vec();
                members.push(TestId(at));
                group.tests = members.into_boxed_slice();
            }
            None => grouped.push(RelationTests {
                relation: test.relation,
                tests: Box::new([TestId(at)]),
            }),
        }
    }
    grouped.into_boxed_slice()
}

/// The table of a compiled program: each region takes the skip formula of its last instruction's node. A synthesized
/// region, whose instruction is no node of the graph, is `Always`.
pub fn for_regions(
    graph: GraphPresence,
    last: impl IntoIterator<Item = RegionNode>,
) -> PresenceTable {
    let GraphPresence {
        mut table,
        nodes,
        reads,
    } = graph;
    let (regions, region_reads): (Vec<FormulaId>, Vec<Box<[Relation]>>) = last
        .into_iter()
        .map(|region| {
            let read = region
                .node
                .and_then(|node| reads.get(node.0))
                .cloned()
                .unwrap_or_default();
            let formula = region
                .node
                .filter(|_| region.certified == Certification::Free)
                .and_then(|node| nodes.get(node.0).copied())
                .unwrap_or(ALWAYS);
            (formula, read)
        })
        .unzip();
    table.regions = regions.into_boxed_slice();
    table.reads = region_reads.into_boxed_slice();
    table
}

