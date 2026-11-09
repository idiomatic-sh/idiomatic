//! The `presence` service: one fold over the deduplicated request graph gives every node a presence formula, a
//! necessary condition for the node to have a row in one file, and a skip formula, which also holds when a row-fallible
//! operator upstream could have rows to fail on. No rows, IO or ports.
use super::types::{
    ComparisonGroup, Conditionality, FoldedNode, OriginComparison, FormulaId, GraphPresence, NodeFacts, OccurrenceId, ColumnOrigin, PresenceFormula,
    PresenceTable, PresenceTest, ReadId, RelationTests, SideOccurrence, PresenceTables, TestBucket, TestId,
};
use crate::domain::executable_program::types::{PartitionRequirement, ProducerPartitions, RegionId};
use crate::domain::file_program::types::{NodeInput, ProgramNodeId, RequestGraph, SchemaValidity};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::projection::{output_origins, tree_output};
use crate::domain::operation_schema::types::InputShape;
use crate::domain::presence::types::{InputMask, PresenceMask, RowFailure};
use ryo_relations::domain::relations::types::{Provenance, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Operand, PlanComparison, QueryLiteral, StepScope};
use ryo_values::domain::plans::types::Operation;
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
fn formula(tables: &mut PresenceTables, formula: PresenceFormula) -> FormulaId {
    if let Some(found) = tables.formula_index.get(&formula) {
        return *found;
    }
    let id = FormulaId(tables.formulas.len());
    tables.formulas.push(formula.clone());
    let _new = tables.formula_index.insert(formula, id);
    id
}
fn test(tables: &mut PresenceTables, relation: Relation, comparisons: Vec<PlanComparison>) -> FormulaId {
    use std::hash::{Hash, Hasher};
    let mut state = std::hash::DefaultHasher::new();
    relation.hash(&mut state);
    comparisons.hash(&mut state);
    let key = TestBucket(state.finish());
    let found = tables.test_buckets.get(&key).and_then(|bucket| {
        bucket.iter().copied().find(|id| {
            tables
                .tests
                .get(id.0)
                .is_some_and(|test| test.relation == relation && *test.comparisons == *comparisons)
        })
    });
    let id = found.unwrap_or_else(|| {
        let id = TestId(tables.tests.len());
        tables.tests.push(PresenceTest {
            relation,
            comparisons: comparisons.into_boxed_slice(),
            kinds: ryo_relations::domain::relations::service::kinds(relation).into(),
        });
        tables.test_buckets.entry(key).or_default().push(id);
        id
    });
    formula(tables, PresenceFormula::Test(id))
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
/// One test per occurrence of a read, AND-ed: each `Column <test> Literal(text)` comparison whose column has an origin,
/// renumbered to the relation's own column. Any other comparison only drops out, which weakens the test.
fn narrowing<'c>(
    tables: &mut PresenceTables,
    comparisons: impl IntoIterator<Item = OriginComparison<'c>>,
) -> Vec<FormulaId> {
    let mut groups: Vec<ComparisonGroup> = Vec::new();
    for OriginComparison { comparison, origin } in comparisons {
        let (Operand::Column(_), Operand::Literal(QueryLiteral::Text(_)), Some(origin)) =
            (&comparison.left, &comparison.right, origin)
        else {
            continue;
        };
        let renumbered = PlanComparison {
            left: Operand::Column(origin.column),
            ..comparison.clone()
        };
        match groups
            .iter_mut()
            .find(|group| group.read == origin.read && group.occurrence == origin.occurrence)
        {
            Some(group) => group.comparisons.push(renumbered),
            None => groups.push(ComparisonGroup {
                read: origin.read,
                occurrence: origin.occurrence,
                comparisons: vec![renumbered],
            }),
        }
    }
    groups
        .into_iter()
        .filter_map(|group| {
            let relation = *tables.reads.get(group.read.0)?;
            Some(test(tables, relation, group.comparisons))
        })
        .collect()
}
fn pick(origins: &[Option<ColumnOrigin>], column: ColumnIndex) -> Option<ColumnOrigin> {
    origins.get(column.0 as usize).copied().flatten()
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
/// The narrowing tests of a select's comparisons, or of a join's filter over its raw right input. A reach join's filter
/// reads the closure's columns, which have no origin.
fn node_narrowing(tables: &mut PresenceTables, operation: &Operation, origins: &[Vec<Option<ColumnOrigin>>]) -> Vec<FormulaId> {
    let (comparisons, input) = match operation {
        Operation::Select { comparisons, .. } => (comparisons, 0),
        Operation::Join {
            filter, reach: None, ..
        } => (filter, 1),
        _ => return Vec::new(),
    };
    let columns = origins.get(input).map_or(&[][..], Vec::as_slice);
    narrowing(
        tables,
        comparisons.iter().map(|comparison| {
            let origin = match comparison.left {
                Operand::Column(column) => pick(columns, column),
                _ => None,
            };
            OriginComparison { comparison, origin }
        }),
    )
}

/// The presence and skip formulas of every node of the graph, folded once in node order, which is topological.
pub fn of_graph(graph: &RequestGraph, partitions: &dyn ProducerPartitions) -> GraphPresence {
    let mut tables = tables();
    let mut facts: Vec<NodeFacts> = Vec::with_capacity(graph.nodes.len());
    let mut skips = Vec::with_capacity(graph.nodes.len());
    for (at, node) in graph.nodes.iter().enumerate() {
        let mut presences = Vec::with_capacity(node.inputs.len());
        let mut failures = Vec::new();
        let mut origins: Vec<Vec<Option<ColumnOrigin>>> = Vec::with_capacity(node.inputs.len());
        let mut trees = Vec::with_capacity(node.inputs.len());
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
                    if partitions.relation(*relation) == PartitionRequirement::CurrentFile
                        && matches!(
                            ryo_relations::domain::relations::service::provenance_of(*relation),
                            Provenance::Trees | Provenance::Engine
                        ) =>
                {
                    trees.push(crate::domain::table_cells::service::native_form(*relation));
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
        let failure = (!failures.is_empty()).then(|| any(&mut tables, failures));
        if conditionality == Conditionality::Unconditional {
            facts.push(NodeFacts {
                presence: ALWAYS,
                failure: None,
                origins: Vec::new(),
                tree,
                conditionality,
            });
            skips.push(ALWAYS);
            continue;
        }
        let base = match crate::domain::operator::contract::service::semantics(node.contract.kind).presence {
            PresenceMask::AllOf(mask) => all(&mut tables, masked(mask, &presences)),
            PresenceMask::AnyOf(mask) => any(&mut tables, masked(mask, &presences)),
            PresenceMask::Always => ALWAYS,
        };
        let narrowing = node_narrowing(&mut tables, &node.operation, &origins);
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
        },
        nodes: skips.into_boxed_slice(),
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
pub fn for_regions(graph: GraphPresence, last: impl IntoIterator<Item = Option<ProgramNodeId>>) -> PresenceTable {
    let GraphPresence { mut table, nodes } = graph;
    table.regions = last
        .into_iter()
        .map(|node| node.and_then(|node| nodes.get(node.0).copied()).unwrap_or(ALWAYS))
        .collect();
    table
}

/// The skip formula of a region. The table holds one for every region of its program.
///
/// # Errors
///
/// [`OperatingError::NotRun`] for a region the program does not have.
pub fn of_region(table: &PresenceTable, region: RegionId) -> Result<FormulaId, OperatingError> {
    table.regions.get(region.0).copied().ok_or(OperatingError::NotRun)
}
