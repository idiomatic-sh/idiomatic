//! The whole-row index's cases, through the plan and run use cases: differences of one right side share one row-set index
//! and answer what each difference's own index answers, file after file; a difference's own index is a row set, readied
//! from the worker's row-set pool and given back to it.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::kept_room::constants::POOL_INDEXES;
use crate::domain::kept_room::types::{PoolSize, Room};
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::physical_plan::types::worker_meters::value::IndexBuilds;
use crate::domain::physical_plan::types::{PlanInput, PlanInputs, PlanNodeId, Resources};
use crate::inbound::use_cases::run::fixtures::cases::{DEFINES, defines, input_of, named, select};
use crate::inbound::use_cases::run::fixtures::unshared::emitted_unshared;
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, emitted};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

/// What a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The two differences over one file, planned as the planner plans them.
    Differences,
    /// The two differences over two files on one worker's buffers, shared against each with its own index.
    DifferencesAgainstOwn,
    /// One difference over one file, the worker's row-set pool holding two emptied sets; then the file ends.
    OwnRowsIndex,
}

/// What a case observed.
#[derive(Debug, PartialEq)]
enum Observation {
    /// How many probe indexes the run built.
    Builds(IndexBuilds),
    /// Whether the two plans answered the same rows, and how many rows the shared plan answered.
    Same(bool, RowCount),
    /// The keyed-row tables and the row sets the worker's index pools keep after the file ended.
    Pooled(PoolSize, PoolSize),
}

type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two_differences_of_one_right_share_one_rows_index"),
        word: Word::Succeeds,
        input: || Scenario::Differences,
        expect: Expect::Value(Observation::Builds(IndexBuilds(1))),
    },
    Case {
        name: CaseName("a_shared_rows_index_answers_what_its_own_index_answers"),
        word: Word::Succeeds,
        input: || Scenario::DifferencesAgainstOwn,
        expect: Expect::Value(Observation::Same(true, RowCount(8))),
    },
    Case {
        name: CaseName("a_whole_row_probe_readies_a_row_set_from_its_pool"),
        word: Word::Succeeds,
        input: || Scenario::OwnRowsIndex,
        expect: Expect::Value(Observation::Pooled(PoolSize(0), PoolSize(2))),
    },
];

/// The texts of every file's cells.
const TEXTS: &[&str] = &["kind", "r", "n1", "n2", "a"];

/// The names of each file's rows of `DEFINES`, in order.
const FILES: [&[&str]; 2] = [&["n1", "n2", "n1", "a"], &["n2", "a", "n2"]];

/// The differences, the nodes they emit: `DEFINES` less its rows named `n1`, then its rows named `n2` less them.
const DIFFERENCES: [PlanNodeId; 2] = [PlanNodeId(2), PlanNodeId(3)];

/// The program of the first `count` differences: two selects of `DEFINES` by name, then each difference, a whole-row probe of
/// the rows named `n1`.
fn steps(count: usize) -> Vec<Step> {
    let rows = PlanInput::Relation(DEFINES);
    let ones = PlanInput::Node(PlanNodeId(0));
    let less = |left: PlanInput| Step {
        operation: Operation::Difference {
            left: input_of(left),
            right: input_of(ones),
        },
        inputs: vec![left, ones],
    };
    [
        select(rows, vec![named("n1")]),
        select(rows, vec![named("n2")]),
        less(rows),
        less(PlanInput::Node(PlanNodeId(1))),
    ]
    .into_iter()
    .take(2 + count)
    .collect()
}

/// A run of a program's emitted nodes: as planned, or with every probe on an index of its own.
type Emit = fn(
    &[Step],
    &[PlanNodeId],
    &dyn PlanInputs,
    &mut dyn Resources,
    &mut FileBuffers,
) -> Result<Vec<Table>, PortableOperatorFault>;

/// Each file's answers of the first `count` differences, in order, run by `emit` over `buffers`, each file ended after them.
///
/// # Errors
///
/// The first fault of a difference.
fn answers(
    emit: Emit,
    count: usize,
    files: &[&[&str]],
    buffers: &mut FileBuffers,
) -> Result<Vec<Table>, Failure> {
    let program = steps(count);
    let emits = DIFFERENCES.get(..count).unwrap_or_default();
    let mut answered = Vec::new();
    for names in files {
        let mut resources = TextResources::new(TEXTS);
        let pairs: Vec<_> = names.iter().map(|name| ("r", *name)).collect();
        let table = defines(&pairs, &resources);
        let inputs = TableInputs::new(&[(DEFINES, &table)]);
        answered.extend(emit(&program, emits, &inputs, &mut resources, buffers)?);
        buffers.ended();
    }
    Ok(answered)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut buffers = FileBuffers::default();
    Ok(match scenario {
        Scenario::Differences => {
            let _answered = answers(emitted, 2, &FILES[..1], &mut buffers)?;
            Observation::Builds(buffers.builds)
        }
        Scenario::DifferencesAgainstOwn => {
            let shared = answers(emitted, 2, &FILES, &mut buffers)?;
            let own = answers(emitted_unshared, 2, &FILES, &mut FileBuffers::default())?;
            let rows = shared.iter().map(|table| table.count.0).sum();
            Observation::Same(shared == own, RowCount(rows))
        }
        Scenario::OwnRowsIndex => {
            for _ in 0..2 {
                let set = RowSet::with_room(Room(1 << 10));
                buffers.pools.rows.pooled(set, PoolSize(POOL_INDEXES));
            }
            let _answered = answers(emitted, 1, &FILES[..1], &mut buffers)?;
            Observation::Pooled(buffers.pools.keyed.len(), buffers.pools.rows.len())
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
