//! File root activation preserves a reader multiset and independent intent masks.
use std::borrow::Cow;

use super::{service::compile, types::*};
use crate::domain::interpreted_program::types::absent_regions::value::AbsentRegions;
use crate::domain::interpreted_program::types::absent_roots::value::AbsentRoots;
use crate::domain::operator::types::ReaderMultiplicity;
use crate::domain::rule_intent::types::FrontierPurpose;
use crate::domain::{
    coverage::types::RuleIndex,
    interpreted_program::types::{
        InputSource, ObservationPlace, RegionId, RuleProgram,
        interpreted_program::value::InterpretedProgram, region::value::Region,
    },
    operating::errors::OperatingError,
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::plan_parts::types::StepScope,
};
/// The regions of a one-rule program, by what each depends on, and the regions the rule's roots and frontier read.
#[derive(Clone, Copy, Debug)]
struct Graph {
    dependencies: &'static [&'static [usize]],
    root_readers: &'static [usize],
    /// The region the rule's diagnostic frontier reads, if it has one.
    frontier: Option<usize>,
}

/// Two regions depending on nothing, read twice and once by the rule's roots, the frontier reading region 1.
const PAIR: Graph = Graph {
    dependencies: &[&[], &[]],
    root_readers: &[0, 0, 1],
    frontier: Some(1),
};

/// The same, region 0 depending on region 1 twice.
const DOUBLE_EDGE: Graph = Graph {
    dependencies: &[&[1, 1], &[]],
    root_readers: &[0, 0, 1],
    frontier: Some(1),
};

/// Rule A's root (region 1) reads X (region 2), which reads Y (region 3); A is also an input of rule F's root (region 0).
const CHAIN: Graph = Graph {
    dependencies: &[&[1], &[2], &[3], &[]],
    root_readers: &[0, 1],
    frontier: None,
};

/// The same, F also reading Y directly.
const CHAIN_SHARED: Graph = Graph {
    dependencies: &[&[1, 3], &[2], &[3], &[]],
    root_readers: &[0, 1],
    frontier: None,
};

/// What a row asks of a file's demand.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// How many readers the producer region (last) has, the roots selected with their readers, the regions whose roots are
    /// absent.
    Readers(Graph, &'static [(usize, usize)], &'static [usize], usize),
    /// What a file selects of `PAIR` under the demand, one observation reading region 0.
    Select(FileDemand),
    /// What a file selects under the demand of `PAIR` whose only observation's region is not read, and whose two observation
    /// sources are a stored observation and a fault.
    Defer(FileDemand),
}

/// What a file's demand answers.
#[derive(Debug, PartialEq)]
enum Answer {
    /// The readers of the producer.
    Readers(ReaderMultiplicity),
    /// How many of the rule's root readers were cancelled, and the roots and observations selected.
    Selected {
        cancelled: usize,
        roots: usize,
        observations: usize,
    },
    /// Each root selected, as the fault it defers (none for a root that is not deferred), and the readers and observations.
    Deferred {
        roots: Cow<'static, [Option<OperatingError>]>,
        readers: usize,
        observations: usize,
    },
}

type Input = Ask;
type Value = Answer;
type Failure = OperatingError;

fn build(graph: Graph, observed: Box<[Result<RegionId, OperatingError>]>) -> InterpretedProgram {
    let regions = graph
        .dependencies
        .iter()
        .map(|dependencies| {
            let mut region = Region::new(
                RuleIndex(0),
                Ok(Box::new([])),
                StepScope::PerFile,
                Box::new([]),
                Box::new([]),
                ReaderMultiplicity(1),
            );
            region.set_dependencies(dependencies.iter().copied().map(RegionId).collect());
            region
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let frontier = graph
        .frontier
        .map(|region| Ok(InputSource::Region(RegionId(region))))
        .into_iter()
        .collect();
    let rules = vec![RuleProgram {
        root_readers: graph.root_readers.iter().copied().map(RegionId).collect(),
        original_steps: Box::new([]),
        emit: Ok(RegionId(0)),
        frontier,
    }]
    .into_boxed_slice();
    let file_readers = super::service::readers(&regions, &rules).unwrap();
    InterpretedProgram::new(regions, file_readers, rules, Box::new([]), observed)
}

fn readers(
    graph: Graph,
    selected: &[(usize, usize)],
    absent: &[usize],
    producer: usize,
) -> Result<Value, Failure> {
    let program = build(graph, Box::new([]));
    let mut roots: Vec<SelectedRoot> = selected
        .iter()
        .map(|(region, readers)| SelectedRoot {
            region: RegionId(*region),
            readers: ReaderMultiplicity(*readers),
        })
        .collect();
    roots.sort_unstable_by_key(|root| root.region.0);
    let absent = if absent.is_empty() {
        AbsentRoots::default()
    } else {
        let mut regions = AbsentRegions::none(
            crate::domain::interpreted_program::types::RegionCount(graph.dependencies.len()),
        );
        for region in absent {
            regions.insert(RegionId(*region));
        }
        AbsentRoots::decided(regions)
    };
    let table = super::service::table(program.regions(), &roots, &absent)?;
    Ok(Answer::Readers(super::service::reader_count(
        &table,
        RegionId(producer),
    )?))
}

fn select(demand: FileDemand) -> Result<Value, Failure> {
    let program = build(PAIR, Box::new([]));
    let prepared = compile(
        &program,
        RuleIndex(0),
        &[ObservationDemandSource {
            source: Ok(InputSource::Region(RegionId(0))),
            phase: StepScope::PerFile,
        }],
        &[FrontierPurpose::Diagnostic],
    )?;
    let selected = prepared.selected(demand);
    Ok(Answer::Selected {
        cancelled: program.rules()[0].root_readers.len() - selected.readers.len(),
        roots: selected.roots.len(),
        observations: selected.observations.len(),
    })
}

fn defer(demand: FileDemand) -> Result<Value, Failure> {
    let program = build(PAIR, vec![Err(OperatingError::NotRead)].into_boxed_slice());
    let prepared = compile(
        &program,
        RuleIndex(0),
        &[
            ObservationDemandSource {
                source: Ok(InputSource::Observation(ObservationPlace(0))),
                phase: StepScope::PerFile,
            },
            ObservationDemandSource {
                source: Err(OperatingError::WrongOperation),
                phase: StepScope::PerFile,
            },
        ],
        &[FrontierPurpose::Diagnostic],
    )?;
    let selected = prepared.selected(demand);
    Ok(Answer::Deferred {
        roots: selected
            .roots
            .iter()
            .map(|root| match root {
                FileRootDemand::Deferred(fault) => Some(fault.clone()),
                FileRootDemand::Ordinary(_) | FileRootDemand::Observation { .. } => None,
            })
            .collect(),
        readers: selected.readers.len(),
        observations: selected.observations.len(),
    })
}

fn subject(ask: Input) -> Result<Value, Failure> {
    match ask {
        Ask::Readers(graph, selected, absent, producer) => {
            readers(graph, selected, absent, producer)
        }
        Ask::Select(demand) => select(demand),
        Ask::Defer(demand) => defer(demand),
    }
}

/// A demand of the diagnostic roots, the observation roots and the frontiers captured.
const fn demand(diagnostic: bool, observations: bool, capture: FrontierSelection) -> FileDemand {
    const fn root(required: bool) -> RootSelection {
        if required {
            RootSelection::Required
        } else {
            RootSelection::Undemanded
        }
    }
    FileDemand {
        diagnostic: root(diagnostic),
        observations: root(observations),
        capture,
    }
}

/// The root readers cancelled, and the roots and observations selected.
const fn selected(cancelled: usize, roots: usize, observations: usize) -> Answer {
    Answer::Selected {
        cancelled,
        roots,
        observations,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("stored observation faults remain deferred under compiled demand"),
        word: Word::Succeeds,
        input: || Ask::Defer(super::constants::NO_FILE_DEMAND),
        expect: Expect::Value(Answer::Deferred {
            roots: Cow::Borrowed(&[]),
            readers: 0,
            observations: 0,
        }),
    },
    Case {
        name: CaseName("a named observation demand defers each faulty source"),
        word: Word::Succeeds,
        input: || {
            Ask::Defer(FileDemand {
                observations: RootSelection::Required,
                ..super::constants::NO_FILE_DEMAND
            })
        },
        expect: Expect::Value(Answer::Deferred {
            roots: Cow::Borrowed(&[
                Some(OperatingError::NotRead),
                Some(OperatingError::WrongOperation),
            ]),
            readers: 0,
            observations: 2,
        }),
    },
    Case {
        name: CaseName(
            "selected reverse readers preserve duplicate edges and reject unknown roots",
        ),
        word: Word::Succeeds,
        input: || Ask::Readers(DOUBLE_EDGE, &[(0, 2)], &[], 0),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(2))),
    },
    Case {
        name: CaseName("the same dependency is consumed once per declared edge"),
        word: Word::Succeeds,
        input: || Ask::Readers(DOUBLE_EDGE, &[(0, 2)], &[], 1),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(2))),
    },
    Case {
        name: CaseName("a source's external reader is independent of consumer edges"),
        word: Word::Succeeds,
        input: || Ask::Readers(DOUBLE_EDGE, &[(0, 2), (1, 1)], &[], 1),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(3))),
    },
    Case {
        name: CaseName("with no root selected a dependency has no reader"),
        word: Word::Succeeds,
        input: || Ask::Readers(DOUBLE_EDGE, &[], &[], 1),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(0))),
    },
    Case {
        name: CaseName("an unknown selected root is refused"),
        word: Word::Refuses,
        input: || Ask::Readers(DOUBLE_EDGE, &[(99, 1)], &[], 1),
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("an absent root's edges are never counted, at any depth"),
        word: Word::Succeeds,
        input: || Ask::Readers(CHAIN, &[(0, 1), (1, 1)], &[1], 1),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(2))),
    },
    Case {
        name: CaseName("an absent root's first dependency gets no count"),
        word: Word::Succeeds,
        input: || Ask::Readers(CHAIN, &[(0, 1), (1, 1)], &[1], 2),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(0))),
    },
    Case {
        name: CaseName("an absent root's deeper dependency gets no count"),
        word: Word::Succeeds,
        input: || Ask::Readers(CHAIN, &[(0, 1), (1, 1)], &[1], 3),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(0))),
    },
    Case {
        name: CaseName("with no absent root, the walk is today's table"),
        word: Word::Succeeds,
        input: || Ask::Readers(CHAIN, &[(0, 1), (1, 1)], &[], 3),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(1))),
    },
    Case {
        name: CaseName(
            "an absent root's dependency another root shares still gets no count through it",
        ),
        word: Word::Succeeds,
        input: || Ask::Readers(CHAIN_SHARED, &[(0, 1), (1, 1)], &[1], 2),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(0))),
    },
    Case {
        name: CaseName("a dependency an admitted root reads directly is counted once"),
        word: Word::Succeeds,
        input: || Ask::Readers(CHAIN_SHARED, &[(0, 1), (1, 1)], &[1], 3),
        expect: Expect::Value(Answer::Readers(ReaderMultiplicity(1))),
    },
    Case {
        name: CaseName("undeclared readers all cancel"),
        word: Word::Succeeds,
        input: || Ask::Select(demand(false, false, FrontierSelection::Disabled)),
        expect: Expect::Value(selected(3, 0, 0)),
    },
    Case {
        name: CaseName("diagnostic keeps one alias"),
        word: Word::Succeeds,
        input: || Ask::Select(demand(true, false, FrontierSelection::Disabled)),
        expect: Expect::Value(selected(2, 1, 0)),
    },
    Case {
        name: CaseName("named reader stays independent"),
        word: Word::Succeeds,
        input: || Ask::Select(demand(false, true, FrontierSelection::Disabled)),
        expect: Expect::Value(selected(2, 1, 1)),
    },
    Case {
        name: CaseName("observation capture excludes unrelated frontier"),
        word: Word::Succeeds,
        input: || Ask::Select(demand(false, false, FrontierSelection::Observations)),
        expect: Expect::Value(selected(3, 0, 0)),
    },
    Case {
        name: CaseName("full capture includes unrelated frontier"),
        word: Word::Succeeds,
        input: || Ask::Select(demand(false, false, FrontierSelection::All)),
        expect: Expect::Value(selected(2, 1, 0)),
    },
    Case {
        name: CaseName("all root aliases have distinct obligations"),
        word: Word::Succeeds,
        input: || Ask::Select(demand(true, true, FrontierSelection::All)),
        expect: Expect::Value(selected(0, 3, 1)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
