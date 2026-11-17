//! What one worker keeps from one file to the next, through the plan and run use cases: a file's end releases only what the
//! file filled, whatever the size of the program, and the room a big file grew in the worker's scratch is not kept for the
//! files after it.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::kept_room::constants::{KEPT_ROOM, POOL_INDEXES};
use crate::domain::kept_room::types::{KeptRoom as _, PoolSize, Room};
use crate::domain::physical_plan::fixtures::{DEFINES, Step, defines_rows};
use crate::domain::physical_plan::types::Resources as _;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::worker_meters::value::EndWork;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId,
};
use crate::inbound::use_cases::run::fixtures::texts::span;
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, planned, ran};
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::Value as CellValue;
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;

/// What a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// One select run in a file, in a program of few other nodes and in one of many: the work of the file's end in each.
    EndWork,
    /// A file that runs no node, in a program of few nodes and in one of many: the work of the file's end in each.
    EmptyEndWork,
    /// A big file, then a small one: the room the shared probe indexes keep, given to the worker's pool after the last probe.
    SharedRoom,
    /// A big file, then a small one: the room a filter's decisions about the file's names keep.
    MemoRoom,
    /// A big file, then a small one: the room the tables of the rows a union and a distinct projection kept keep.
    RepeatsRoom,
    /// A streamed projection that drops repeats, its worker's row-set pool holding one emptied set.
    PooledProjection,
    /// A streamed probe of its own index, its worker's keyed pool holding one emptied table and its row-set pool one set.
    PooledProbe,
}

/// What a case observed.
#[derive(Debug, PartialEq)]
enum Observation {
    /// The work of the file's end in the program of few nodes, then in the program of many.
    EndWork(EndWork, EndWork),
    /// Whether the big file grew the room past the cap, and whether the room kept after the small file's end is within it.
    Room { grew: bool, kept: bool },
    /// The keyed-row tables and the row sets the worker's index pools keep after the file ended.
    Pooled(PoolSize, PoolSize),
}

type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_files_end_releases_what_it_filled_whatever_the_programs_size"),
        word: Word::Succeeds,
        input: || Scenario::EndWork,
        expect: Expect::Value(Observation::EndWork(EndWork(1), EndWork(1))),
    },
    Case {
        name: CaseName("an_empty_files_end_does_no_work_whatever_the_programs_size"),
        word: Word::Succeeds,
        input: || Scenario::EmptyEndWork,
        expect: Expect::Value(Observation::EndWork(EndWork(0), EndWork(0))),
    },
    Case {
        name: CaseName("a_big_files_shared_index_gives_its_room_back_in_the_file"),
        word: Word::Succeeds,
        input: || Scenario::SharedRoom,
        expect: Expect::Value(Observation::Room {
            grew: false,
            kept: true,
        }),
    },
    Case {
        name: CaseName("a_big_files_filter_decisions_room_is_not_kept_past_the_cap"),
        word: Word::Succeeds,
        input: || Scenario::MemoRoom,
        expect: Expect::Value(Observation::Room {
            grew: true,
            kept: true,
        }),
    },
    Case {
        name: CaseName("a_big_files_repeats_room_is_not_kept_past_the_cap"),
        word: Word::Succeeds,
        input: || Scenario::RepeatsRoom,
        expect: Expect::Value(Observation::Room {
            grew: true,
            kept: true,
        }),
    },
    Case {
        name: CaseName("a_prepared_stage_reuses_pooled_room"),
        word: Word::Succeeds,
        input: || Scenario::PooledProjection,
        expect: Expect::Value(Observation::Pooled(PoolSize(0), PoolSize(1))),
    },
    Case {
        name: CaseName("a_prepared_probe_of_its_own_index_reuses_pooled_room"),
        word: Word::Succeeds,
        input: || Scenario::PooledProbe,
        expect: Expect::Value(Observation::Pooled(PoolSize(1), PoolSize(1))),
    },
];

/// The most room each kind of a worker's scratch keeps.
#[derive(Clone, Copy, Debug, Default)]
struct RetainedRoom {
    /// Of a shared probe index.
    shared: Room,
    /// Of a filter's decisions about the names of a file.
    memo: Room,
    /// Of a table of the rows a union or a stage that drops repeats kept.
    repeats: Room,
}

/// The most room each kind of `buffers`' scratch keeps now.
fn retained(buffers: &FileBuffers) -> RetainedRoom {
    RetainedRoom {
        shared: buffers
            .shared
            .indexes
            .iter()
            .map(|shared| shared.rows.room())
            .max()
            .unwrap_or_default(),
        memo: buffers
            .decisions
            .iter()
            .map(|decisions| decisions.memo.room())
            .max()
            .unwrap_or_default(),
        repeats: buffers
            .stream
            .levels
            .iter()
            .flat_map(|level| &level.stages)
            .map(|stage| stage.own.repeats.room())
            .chain([buffers.repeats.room()])
            .max()
            .unwrap_or_default(),
    }
}

/// The names of the big file: more than the cap, each its own text.
const BIG: u32 = 10_000;

/// What the file of the release-cost cases runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Runs {
    /// The select.
    Select,
    /// No node, after a file that ran the select: a file with nothing to fill.
    Nothing,
}

/// The names of the small file.
const SMALL: u32 = 3;

/// The other nodes of the program of few nodes, and of the program of many.
const OTHERS: [u32; 2] = [4, 64];

/// A step of `operation` reading `inputs`.
fn step(operation: Operation, inputs: &[PlanInput]) -> Step {
    Step {
        operation,
        inputs: inputs.to_vec(),
    }
}

/// `DEFINES`, as a step reads it and as the plan does.
fn defines() -> (StepInput, PlanInput) {
    (StepInput::Relation(DEFINES), PlanInput::Relation(DEFINES))
}

/// Node `at`, as a step reads it and as the plan does.
fn node(at: u32) -> (StepInput, PlanInput) {
    (StepInput::Step(StepId(at)), PlanInput::Node(PlanNodeId(at)))
}

/// The rows of `DEFINES` whose name `test`s `text`.
fn select(test: Test, text: &str) -> Step {
    step(
        Operation::Select {
            from: defines().0,
            comparisons: vec![PlanComparison {
                test,
                left: Operand::Column(ColumnIndex(3)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: Vec::new(),
        },
        &[defines().1],
    )
}

/// The names of `from`'s rows of `DEFINES`, their repeats `distinct`.
fn names(from: (StepInput, PlanInput), distinct: RepeatedRows) -> Step {
    step(
        Operation::Project {
            from: from.0,
            columns: vec![ColumnIndex(3)],
            distinct,
        },
        &[from.1],
    )
}

/// A join of `left` with `DEFINES` on their names, answering the left row's first column: every such probe of a program
/// shares one index of `DEFINES`.
fn probe(left: (StepInput, PlanInput)) -> Step {
    step(
        Operation::Join {
            left: left.0,
            right: StepInput::Relation(DEFINES),
            left_on: vec![ColumnIndex(3)],
            right_on: vec![ColumnIndex(3)],
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            }],
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
        &[left.1, PlanInput::Relation(DEFINES)],
    )
}

/// The plan of `steps`, each of `emits` a rule's emit.
fn plan_of(steps: &[Step], emits: &[PlanNodeId]) -> Result<PhysicalPlan, Failure> {
    let mut program = LogicalProgram::new();
    for each in steps {
        let _id = program.node(LogicalNode {
            operation: &each.operation,
            inputs: PlanInputList(&each.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    for (rule, emitted) in (0..).map(RuleOrdinal).zip(emits) {
        program.emit(rule, *emitted);
    }
    planned(&program)
}

/// The resources and the `DEFINES` rows of a file of `count` names, each its own text.
fn file(count: u32) -> (TextResources, ryo_values::domain::rows::types::Table) {
    let texts: Vec<String> = (0..count).map(|at| format!("n{at}")).collect();
    let borrowed: Vec<&str> = texts.iter().map(String::as_str).collect();
    let resources = TextResources::new(&borrowed);
    let texts = resources.texts();
    // Each text is a symbol of the file, numbered in its order from the first.
    let rows: Vec<_> = (0..u32::MAX)
        .map(Symbol)
        .map_while(|symbol| {
            text_of(texts.symbols, ByteSlice(texts.source.0), symbol)
                .map(|_| (CellValue::Symbol(symbol), span(0, 1)))
        })
        .collect();
    let table = defines_rows(&rows);
    (resources, table)
}

/// The work of the end of a file that runs only a select, or `runs` nothing, in a program of the select, `others` selects no rule of the file
/// runs, and two probes sharing an index. A select of a prefix is no lookup: a lookup's run answers every lookup of its
/// index, so the file would fill them all.
///
/// # Errors
///
/// The select's fault.
fn end_work(others: u32, runs: Runs) -> Result<EndWork, Failure> {
    let mut steps = vec![select(Test::StartsWith, "n0")];
    steps.extend((0..others).map(|_| select(Test::StartsWith, "n1")));
    steps.extend([probe(defines()), probe(defines())]);
    let emits: Vec<_> = (0..u32::try_from(steps.len()).unwrap_or(u32::MAX))
        .map(PlanNodeId)
        .collect();
    let plan = plan_of(&steps, &emits)?;
    let mut buffers = FileBuffers::default();
    let (mut resources, table) = file(SMALL);
    let inputs = TableInputs::new(&[(DEFINES, &table)]);
    ran(&plan, PlanNodeId(0), &mut buffers, &inputs, &mut resources)?;
    if runs == Runs::Nothing {
        // A file before it sized the buffers for the program; the empty file's end is the one counted.
        buffers.ended();
        buffers.ending = EndWork::default();
    }
    buffers.ended();
    Ok(buffers.ending)
}

/// The room the worker's scratch keeps after the big file's rows, before its end, and after the small file's end, of a
/// program of a filter of the names, two projections of them and their union, and two probes sharing an index.
///
/// # Errors
///
/// The first fault of a node.
fn rooms() -> Result<[RetainedRoom; 2], Failure> {
    let steps = [
        select(Test::StartsWith, "n"),
        names(defines(), RepeatedRows::Dropped),
        names(defines(), RepeatedRows::Kept),
        step(
            Operation::Union {
                left: node(1).0,
                right: node(2).0,
            },
            &[node(1).1, node(2).1],
        ),
        probe(defines()),
        probe(node(0)),
    ];
    let emits = [PlanNodeId(3), PlanNodeId(4), PlanNodeId(5)];
    let plan = plan_of(&steps, &emits)?;
    let mut buffers = FileBuffers::default();
    let mut rooms = [RetainedRoom::default(); 2];
    for (count, room) in [BIG, SMALL].into_iter().zip(&mut rooms) {
        let (mut resources, table) = file(count);
        let inputs = TableInputs::new(&[(DEFINES, &table)]);
        for emitted in emits {
            ran(&plan, emitted, &mut buffers, &inputs, &mut resources)?;
        }
        if count == BIG {
            *room = retained(&buffers);
        }
        buffers.ended();
        if count == SMALL {
            *room = retained(&buffers);
        }
    }
    Ok(rooms)
}

/// Whether the room `of` reads grew past the cap in the big file, and is within it after the small file's end.
fn room(of: fn(RetainedRoom) -> Room) -> Result<Observation, Failure> {
    let [big, small] = rooms()?;
    Ok(Observation::Room {
        grew: of(big) > Room(KEPT_ROOM),
        kept: of(small) <= Room(KEPT_ROOM),
    })
}

/// The tables and sets the worker's index pools keep after the small file runs a select of its names and `stage` over it and
/// ends, the pools holding `[keyed, sets]` of 1 Ki rows' room before: as many when each stage readied its buffers from them.
fn pooled(stage: Step, [keyed, sets]: [usize; 2]) -> Result<Observation, Failure> {
    let plan = plan_of(&[select(Test::StartsWith, "n"), stage], &[PlanNodeId(1)])?;
    let mut buffers = FileBuffers::default();
    let (pools, room, most) = (&mut buffers.pools, Room(1 << 10), PoolSize(POOL_INDEXES));
    (0..keyed).for_each(|_| pools.keyed.pooled(KeyedRows::with_room(room), most));
    (0..sets).for_each(|_| pools.rows.pooled(RowSet::with_room(room), most));
    let (mut resources, table) = file(SMALL);
    let inputs = TableInputs::new(&[(DEFINES, &table)]);
    ran(&plan, PlanNodeId(1), &mut buffers, &inputs, &mut resources)?;
    buffers.ended();
    let pools = &buffers.pools;
    Ok(Observation::Pooled(pools.keyed.len(), pools.rows.len()))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::EndWork => Ok(Observation::EndWork(
            end_work(OTHERS[0], Runs::Select)?,
            end_work(OTHERS[1], Runs::Select)?,
        )),
        Scenario::EmptyEndWork => Ok(Observation::EndWork(
            end_work(OTHERS[0], Runs::Nothing)?,
            end_work(OTHERS[1], Runs::Nothing)?,
        )),
        Scenario::SharedRoom => room(|room| room.shared),
        Scenario::MemoRoom => room(|room| room.memo),
        Scenario::RepeatsRoom => room(|room| room.repeats),
        Scenario::PooledProjection => pooled(names(node(0), RepeatedRows::Dropped), [0, 1]),
        Scenario::PooledProbe => pooled(probe(node(0)), [1, 1]),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
