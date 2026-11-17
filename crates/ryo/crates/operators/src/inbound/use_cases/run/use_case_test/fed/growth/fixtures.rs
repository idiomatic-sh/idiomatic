//! Test support of the growth harness: the generic rows every operator's instances run on, by family and by the role of each
//! input, the plan of one instance (or of many copies of it), and one measured run of it over a family's files.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::growth::types::{Bound, Degree, InputRole, WitnessColumns, Witnessed};
use crate::domain::physical_plan::service::{REWRITES, finished};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::{BoundScan, Lowering, ScanBoundNode};
use crate::domain::physical_plan::types::physical_plan::value::{PhysicalNode, PhysicalPlan};
use crate::domain::physical_plan::types::{
    EngineUses, IndexKey, KeyedIndex, NodeFlow, NodeReaders, NodeRows, PlanNodeId, RowSource,
};
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources};
use crate::inbound::use_cases::run::interface::RunUseCase as _;
use crate::inbound::use_cases::run::use_case::RunUseCaseImpl;
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Integer, Offset, Position, Relation, Span, Value,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, HostedNumber};
use ryo_values::domain::rows::types::{RowCount, Table};

/// The generic columns every instance is built over, and the rows hold: a key, an order, an entity, an edge's ends, a text, a
/// span and a state of a catalogue.
pub(super) const COLUMNS: WitnessColumns = WitnessColumns {
    key: ColumnIndex(0),
    order: ColumnIndex(1),
    id: ColumnIndex(2),
    parent: ColumnIndex(3),
    child: ColumnIndex(4),
    text: ColumnIndex(5),
    span: ColumnIndex(6),
    state: ColumnIndex(7),
};

/// The kinds of the generic rows, in `COLUMNS`' order.
const KINDS: [CellKind; 8] = [
    CellKind::Node,
    CellKind::Number,
    CellKind::Node,
    CellKind::Node,
    CellKind::Node,
    CellKind::Symbol,
    CellKind::Span,
    CellKind::Node,
];

/// How many states the catalogue of states holds.
const STATES: usize = 4;

/// The first state of the catalogue, past every key a row holds.
const STATE: usize = 1 << 30;
/// The least state of the flat lattice of the catalogue.
const BOTTOM: usize = STATE + STATES;
/// The greatest state of the flat lattice.
const TOP: usize = BOTTOM + 1;

/// The text every row's text cell names, and the file's bytes.
const TEXT: &str = crate::domain::growth::types::WITNESS_TEXT;

/// How the rows grow with the size `n`: what each family stresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Family {
    /// One file of `n` rows a side, every row of one key: a product of a shared key, and the edges a star.
    AllEqualKeys,
    /// One file: a chain of `n` edges, its head and `n` lone entities the starts, each key its own: one deep walk, then many
    /// small ones.
    DeepChain,
    /// One file of `n` rows a side, each key its own, and the edges a star of `n` children.
    WideFanOut,
    /// `n` files of four rows a side, the same keys, orders and entities in each.
    ManyFiles,
    /// `n` files of 16 rows a side, the same keys, orders and entities in each.
    ManySmallFiles,
    /// `n` copies of the instance in one plan over one file of four rows a side, every copy run.
    WidePlan,
    /// `n` copies of the instance in one plan over `n` files of four rows a side, each file running its own copy: a file's work
    /// is what it runs, not the program's size.
    WideProgram,
}

impl Family {
    /// Every family, in order.
    pub(super) const ALL: [Self; 7] = [
        Self::AllEqualKeys,
        Self::DeepChain,
        Self::WideFanOut,
        Self::ManyFiles,
        Self::ManySmallFiles,
        Self::WidePlan,
        Self::WideProgram,
    ];

    /// How many files a run of size `n` reads.
    const fn files(self, n: usize) -> usize {
        match self {
            Self::AllEqualKeys | Self::DeepChain | Self::WideFanOut | Self::WidePlan => 1,
            Self::ManyFiles | Self::ManySmallFiles | Self::WideProgram => n,
        }
    }

    /// How many copies of the instance a run of size `n` plans.
    pub(super) const fn copies(self, n: usize) -> usize {
        match self {
            Self::WidePlan | Self::WideProgram => n,
            Self::AllEqualKeys | Self::DeepChain | Self::WideFanOut | Self::ManyFiles | Self::ManySmallFiles => 1,
        }
    }

    /// The size the family is measured at: `rows` a side for one file of growing rows, `files` for many files or copies.
    pub(super) const fn size(self, rows: usize, files: usize) -> usize {
        match self {
            Self::AllEqualKeys | Self::DeepChain | Self::WideFanOut => rows,
            Self::ManyFiles | Self::ManySmallFiles | Self::WidePlan | Self::WideProgram => files,
        }
    }

    /// The copies file `file` runs, of `copies`: its own for a wide program, every one otherwise.
    fn runs(self, file: usize, copies: &[PlanNodeId]) -> &[PlanNodeId] {
        match self {
            Self::WideProgram => copies.get(file..=file).unwrap_or(&[]),
            Self::AllEqualKeys
            | Self::DeepChain
            | Self::WideFanOut
            | Self::ManyFiles
            | Self::ManySmallFiles
            | Self::WidePlan => copies,
        }
    }

    /// How many rows each file holds for one input, at size `n`.
    const fn rows(self, n: usize) -> usize {
        match self {
            Self::AllEqualKeys | Self::DeepChain | Self::WideFanOut => n,
            Self::ManyFiles | Self::WidePlan | Self::WideProgram => 4,
            Self::ManySmallFiles => 16,
        }
    }

    /// The degree in `n` of a bound over this family: a single file's rows, depth and repeats each grow with `n` as the family
    /// grows them; across many files, or many copies, of a fixed size every bound is a constant per file or copy, so linear in
    /// `n`.
    pub(super) fn degree(self, bound: Bound) -> Degree {
        let [rows, depth, repeats] = match self {
            Self::AllEqualKeys => [1, 0, 1],
            Self::DeepChain => [1, 1, 0],
            Self::WideFanOut => [1, 0, 0],
            Self::ManyFiles | Self::ManySmallFiles | Self::WidePlan | Self::WideProgram => return Degree(1),
        };
        Degree(
            bound
                .rows
                .0
                .saturating_mul(rows)
                .saturating_add(bound.depth.0.saturating_mul(depth))
                .saturating_add(bound.repeats.0.saturating_mul(repeats)),
        )
    }
}

/// An entity of the file: the same place in every file, of that file.
fn entity(file: usize, at: usize) -> Value {
    Value::Node(node_id(
        FileId(u32::try_from(file).unwrap_or(u32::MAX)),
        Position(u32::try_from(at).unwrap_or(u32::MAX)),
    ))
}

/// A key or a lattice state: the same value in every file, so a key kept past its file meets the next file's rows.
fn key(at: usize) -> Value {
    Value::Node(node_id(
        FileId(0),
        Position(u32::try_from(at).unwrap_or(u32::MAX)),
    ))
}

/// A state of the catalogue: the same in every file.
fn state(at: usize) -> Value {
    key(STATE.saturating_add(at))
}

/// A number.
fn number(at: usize) -> Value {
    Value::Number(Integer(i64::try_from(at).unwrap_or(i64::MAX)))
}

/// The first key of the keys only one input holds, past every shared key.
const UNIQUE: usize = 1 << 20;

/// The rows of input `input`, of `role`, in file `file` of a run of `family` at size `n`. Even rows hold the keys every input
/// shares, odd rows a key of this input alone, so a probe both meets and misses; rows 0 mod 4 are the same in every input, so
/// a whole-row intersection meets; at rows 2 mod 4 a later input's order follows the first's, so a band or an ordered match
/// pairs; the edges end with one edge back to the first entity, a cycle.
fn role_rows(family: Family, role: InputRole, input: usize, file: usize, n: usize, text: &Value) -> Table {
    let rows = family.rows(n);
    let span = Value::Span(Span {
        start: Offset(0),
        end: Offset(1),
    });
    let mut cells = Vec::new();
    let mut push = |key: Value, order: usize, id: Value, parent: Value, child: Value| {
        // Rows 2 mod 4 of a later input come just after the first input's: rows 0 mod 4 are the same row in every input.
        let later = usize::from(input > 0 && order % 4 == 2);
        cells.extend([
            key,
            number(order.saturating_mul(2).saturating_add(later)),
            id,
            parent,
            child,
            text.clone(),
            span.clone(),
            state(order % STATES),
        ]);
    };
    let count = match role {
        InputRole::Rows => {
            for at in 0..rows {
                let shared = match family {
                    _ if at % 2 == 1 => key(UNIQUE.saturating_mul(input.saturating_add(1)).saturating_add(at)),
                    Family::AllEqualKeys | Family::ManyFiles | Family::WidePlan | Family::WideProgram => key(0),
                    Family::DeepChain | Family::WideFanOut | Family::ManySmallFiles => key(at),
                };
                // A deep chain's starts: its head, then lone entities past it.
                let start = match (family, at) {
                    (Family::DeepChain, 0) => 0,
                    (Family::DeepChain, _) => rows.saturating_add(1).saturating_add(at),
                    _ => at,
                };
                push(
                    shared,
                    at,
                    entity(file, start),
                    entity(file, at),
                    entity(file, at.saturating_add(1)),
                );
            }
            rows
        }
        InputRole::Edges => {
            for at in 0..rows {
                // A chain for the deep family, a star from the first entity for every other.
                let (from, state) = match family {
                    Family::DeepChain => (at, at),
                    _ => (0, 0),
                };
                // The last edge closes a cycle: from the entity the edge before it reached, back to the first.
                let (from, to) = if at + 1 == rows && at > 0 { (at, 0) } else { (from, at.saturating_add(1)) };
                push(key(state), at, entity(file, at), entity(file, from), entity(file, to));
            }
            rows
        }
        InputRole::Domain => {
            // Every entity the other roles name, and the catalogue's states as a flat lattice: a least state below every
            // state and a greatest above, each pair once, then the last pair again.
            let all = rows.saturating_mul(2).saturating_add(2).max(STATES * 2);
            for at in 0..all {
                let pair = at.min(STATES * 2 - 1);
                let (below, above) = if pair < STATES {
                    (key(BOTTOM), state(pair))
                } else {
                    (state(pair - STATES), key(TOP))
                };
                push(key(at), at, entity(file, at), below, above);
            }
            all
        }
    };
    Table {
        kinds: KINDS.to_vec(),
        cells,
        count: RowCount(count),
    }
}

/// The plan of `witness` over a family, planned as the planner plans: one bound scan per input it reads, then `copies` copies
/// of it, each reading the scans, each kept by the engine; a lookup's index is the facts' text index; every rewrite applied.
pub(super) fn plan(witness: &Witnessed<Lowering>, copies: usize) -> (PhysicalPlan, Vec<PlanNodeId>) {
    let inputs = witness.roles.len();
    let mut nodes: Vec<PhysicalNode> = (0..inputs)
        .map(|at| PhysicalNode {
            lowering: Lowering::ScanBound(ScanBoundNode {
                scan: BoundScan::Hosted(HostedNumber(u32::try_from(at).unwrap_or(u32::MAX))),
            }),
            readers: NodeReaders(0),
            engine: EngineUses(0),
            flow: NodeFlow::Held,
        })
        .collect();
    let first = nodes.len();
    for _ in 0..copies {
        nodes.push(PhysicalNode {
            lowering: witness.node.clone(),
            readers: NodeReaders(0),
            engine: EngineUses(1),
            flow: NodeFlow::Held,
        });
    }
    let reads: Vec<PlanNodeId> = nodes
        .iter()
        .flat_map(|node| node.lowering.sources())
        .filter_map(|source| match source {
            RowSource::Node(read) => Some(read),
            RowSource::Facts(_) | RowSource::Tree(_) | RowSource::Frontier(_) | RowSource::Observation(_) => None,
        })
        .collect();
    for read in reads {
        if let Some(node) = nodes.get_mut(usize::try_from(read.0).unwrap_or(usize::MAX)) {
            node.readers = NodeReaders(node.readers.0.saturating_add(1));
        }
    }
    let ran: Vec<PlanNodeId> = (first..nodes.len())
        .map(|at| PlanNodeId(u32::try_from(at).unwrap_or(u32::MAX)))
        .collect();
    let indexes = vec![KeyedIndex {
        source: RowSource::Facts(Relation::Defines),
        key: IndexKey::Text(ColumnIndex(3)),
        readers: ran.clone(),
    }];
    (finished(nodes, indexes, REWRITES), ran)
}

/// What one run of a family at one size did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Measured {
    /// The rows the instance read, every input of every file summed.
    pub(super) read: usize,
    /// The rows the instance wrote, every file summed.
    pub(super) wrote: usize,
    /// The most rows the node tables held at once.
    pub(super) peak: usize,
    /// The work the worker counted: the slots its walks' resets emptied, the nodes its file ends released, the lookups its
    /// shared indexes answered.
    pub(super) work: usize,
}

/// One file's inputs: its host rows by role, the facts a lookup reads, and its text.
pub(super) struct FileInputs {
    /// The host rows, by the role of each input.
    hosted: Vec<Table>,
    /// The facts a source-less instance reads.
    facts: Table,
}

/// The inputs of every file of `family` at size `n` for an instance of `roles`.
pub(super) fn inputs(family: Family, roles: &[InputRole], n: usize) -> Vec<FileInputs> {
    let text = TextResources::new(&[TEXT]).symbol(TEXT);
    (0..family.files(n))
        .map(|file| {
            // A source-less instance (a bound scan) reads the first host rows.
            let hosted = if roles.is_empty() {
                vec![role_rows(family, InputRole::Rows, 0, file, n, &text)]
            } else {
                roles
                    .iter()
                    .enumerate()
                    .map(|(input, role)| role_rows(family, *role, input, file, n, &text))
                    .collect()
            };
            let defines: Vec<(Value, Span)> = (0..family.rows(n))
                .map(|at| {
                    (
                        text.clone(),
                        Span {
                            start: Offset(0),
                            end: Offset(u32::try_from(at.min(1)).unwrap_or(1)),
                        },
                    )
                })
                .collect();
            FileInputs {
                hosted,
                facts: crate::domain::physical_plan::fixtures::defines_rows(&defines),
            }
        })
        .collect()
}

/// One run of every file, each file running the copies the family runs in it, then ended as the engine ends it.
pub(super) fn once(
    plan: &PhysicalPlan,
    (family, ran): (Family, &[PlanNodeId]),
    files: &[FileInputs],
    buffers: &mut FileBuffers,
) -> Result<Measured, PortableOperatorFault> {
    let run = RunUseCaseImpl::new(std::sync::Arc::new(
        crate::outbound::matching::factory::use_case(),
    ));
    let mut measured = Measured::default();
    for (at, file) in files.iter().enumerate() {
        let inputs = TableInputs::new(&[(Relation::Defines, &file.facts)])
            .with_hosted(file.hosted.clone());
        // A rendering answers each request with the witness text.
        let mut resources = TextResources::new(&[TEXT]).rendering(vec![
            ryo_language::domain::semantics::types::RenderOutcome::Known(
                ryo_language::domain::semantics::types::RenderedNode(TEXT.into()),
            );
            file.hosted.first().map_or(1, |rows| rows.count.0.max(1))
        ]);
        for node in family.runs(at, ran) {
            run.run(plan, *node, buffers, &inputs, &mut resources)?;
            measured.read = measured.read.saturating_add(
                crate::domain::growth::service::fed_rows(&buffers.fed(*node).0).0,
            );
            if let NodeRows::Materialized(table) = buffers.rows(*node) {
                measured.wrote = measured.wrote.saturating_add(table.count.0);
            }
        }
        buffers.ended();
    }
    measured.peak = buffers.meter.peak_rows.0;
    measured.work = buffers
        .ending
        .0
        .saturating_add(buffers.walk.visited.reset_work().0)
        .saturating_add(buffers.shared.answered.0);
    Ok(measured)
}
