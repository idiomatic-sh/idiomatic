//! The `presence` service: one fold over the deduplicated request graph gives every node a presence formula, a
//! necessary condition for the node to have a row in one file, and a skip formula, which also holds when a row-fallible
//! operator upstream could have rows to fail on. No rows, IO or ports.
use super::narrowing::{service::formula, service::test};
use super::types::{
    Certification, ColumnOrigin, Conditionality, FoldedNode, FormulaId, GraphPresence, LocatedId,
    NodeFacts, OccurrenceId, PresenceFormula, PresenceTable, PresenceTables, PresenceTest, ReadId,
    RegionNode, RelationScope, RelationTests, SideOccurrence, SpanCheck, SpanColumn, TestId,
};
use crate::domain::file_program::types::{NodeInput, RequestGraph, SchemaValidity};
use crate::domain::operation_schema::projection::{service::output_origins, service::tree_output};
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::presence::{
    types::InputMask, types::PresenceMask, types::RowFailure,
};
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
        located: Vec::new(),
        located_index: HashMap::new(),
        relied: std::collections::BTreeSet::new(),
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
/// The presence and skip formulas of every node of the graph, folded once in node order, which is topological. A source
/// search's pattern is compiled into its prepared slot here, so the run uses the matcher the fold checked.
pub fn of_graph(
    graph: &RequestGraph,
    scope: &dyn Fn(Relation) -> RelationScope,
    matcher: ryo_operators::domain::patterns::types::PatternMatcher<'_>,
    prepared: &ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers,
) -> GraphPresence {
    let mut tables = tables();
    let mut facts: Vec<NodeFacts> = Vec::with_capacity(graph.nodes.len());
    let mut skips = Vec::with_capacity(graph.nodes.len());
    for (at, node) in graph.nodes.iter().enumerate() {
        let mut presences = Vec::with_capacity(node.inputs.len());
        let mut failures = Vec::new();
        let mut origins: Vec<Vec<Option<ColumnOrigin>>> = Vec::with_capacity(node.inputs.len());
        let mut located: Vec<Vec<Option<LocatedId>>> = Vec::with_capacity(node.inputs.len());
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
                    let kinds = node
                        .contract
                        .input_kinds
                        .get(place)
                        .map_or(&[][..], Vec::as_slice);
                    located.push(super::narrowing::service::located_relation(
                        &mut tables,
                        *relation,
                        kinds,
                    ));
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
                    located.push(vec![None; width]);
                }
                (NodeInput::Node(_), Some(upstream)) => {
                    if upstream.conditionality == Conditionality::Unconditional {
                        conditionality = Conditionality::Unconditional;
                    }
                    presences.push(upstream.presence);
                    reads.extend_from_slice(&upstream.reads);
                    failures.extend(upstream.failure);
                    origins.push(upstream.origins.clone());
                    located.push(upstream.located.clone());
                    trees.push(upstream.tree);
                }
                (NodeInput::Node(_) | NodeInput::Observation(_), _) => {
                    conditionality = Conditionality::Unconditional;
                    presences.push(ALWAYS);
                    origins.push(vec![None; width]);
                    located.push(vec![None; width]);
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
        let mut relied = None;
        match node.contract.failure {
            RowFailure::Never => {}
            RowFailure::OnRows(mask) => failures.extend(masked(mask, &presences)),
            RowFailure::UnlessLocated(span) => {
                relied = super::narrowing::service::relied(
                    &tables,
                    &node.operation,
                    span,
                    located.first().map_or(&[][..], Vec::as_slice),
                    prepared,
                    matcher,
                );
                if relied.is_none() {
                    failures.extend(masked(InputMask(1), &presences));
                }
            }
        }
        if let Some(mask) =
            super::narrowing::service::invalid_patterns(&node.operation, matcher, prepared)
        {
            failures.extend(masked(mask, &presences));
        }
        let failure = (!failures.is_empty()).then(|| any(&mut tables, failures));
        if conditionality == Conditionality::Unconditional {
            facts.push(NodeFacts {
                presence: ALWAYS,
                failure: None,
                origins: Vec::new(),
                located: Vec::new(),
                tree,
                conditionality,
                reads,
            });
            skips.push(ALWAYS);
            continue;
        }
        let base = match crate::domain::operator::contract::service::semantics(node.contract.kind)
            .presence
        {
            PresenceMask::AllOf(mask) => all(&mut tables, masked(mask, &presences)),
            PresenceMask::AnyOf(mask) => any(&mut tables, masked(mask, &presences)),
            PresenceMask::Always => ALWAYS,
        };
        let narrowing =
            super::narrowing::service::node_narrowing(&mut tables, &node.operation, &origins);
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
        // A search that declares no failure relies on its provider spans: each file validates them when it admits them.
        tables.relied.extend(relied.into_iter().flatten());
        let located_slices: Vec<&[Option<LocatedId>]> = located.iter().map(Vec::as_slice).collect();
        let located = super::narrowing::service::located_output(
            &mut tables,
            &node.operation,
            &shapes,
            &located_slices,
        );
        let skip = match failure {
            Some(failure) => any(&mut tables, [presence, failure]),
            None => presence,
        };
        facts.push(NodeFacts {
            presence,
            failure,
            origins: output,
            located,
            tree,
            conditionality,
            reads,
        });
        skips.push(skip);
    }
    let PresenceTables {
        tests,
        formulas,
        relied,
        ..
    } = tables;
    GraphPresence {
        table: PresenceTable {
            relations: by_relation(&tests),
            tests: tests.into_boxed_slice(),
            formulas: formulas.into_boxed_slice(),
            regions: Box::new([]),
            reads: Box::new([]),
            spans: span_checks(relied),
        },
        nodes: skips.into_boxed_slice(),
        reads: facts
            .into_iter()
            .map(|node| node.reads.into_boxed_slice())
            .collect(),
    }
}

/// The relied span columns grouped by relation, both in order.
fn span_checks(relied: std::collections::BTreeSet<SpanColumn>) -> Box<[SpanCheck]> {
    let mut checks: Vec<SpanCheck> = Vec::new();
    for SpanColumn { relation, column } in relied {
        match checks.last_mut() {
            Some(check) if check.relation == relation => {
                let mut columns = std::mem::take(&mut check.columns).into_vec();
                columns.push(column);
                check.columns = columns.into_boxed_slice();
            }
            _ => checks.push(SpanCheck {
                relation,
                columns: Box::new([column]),
            }),
        }
    }
    checks.into_boxed_slice()
}

/// The tests of each relation, in the order the relations are first tested.
fn by_relation(tests: &[PresenceTest]) -> Box<[RelationTests]> {
    let mut grouped: Vec<RelationTests> = Vec::new();
    for (at, test) in tests.iter().enumerate() {
        match grouped
            .iter_mut()
            .find(|group| group.relation == test.relation)
        {
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
