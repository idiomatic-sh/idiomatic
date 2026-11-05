//! File root activation preserves a reader multiset and independent intent masks.
use super::{service::compile, types::*};
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{
        ExecutableProgram, InputSource, ReaderMultiplicity, Region, RegionId, RuleProgram,
    },
    operating::errors::OperatingError,
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::plans::types::StepScope,
};
fn program() -> ExecutableProgram {
    let regions = (0..2)
        .map(|at| Region {
            id: RegionId(at),
            producer: RuleIndex(0),
            needs: Ok(Box::new([])),
            phase: StepScope::PerFile,
            instructions: Box::new([]),
            inputs: Box::new([]),
            source_partitions: Box::new([]),
            consumers: 1,
            dependencies: Box::new([]),
            ordered_existence: None,
            column: None,
            selection: None,
            native_positions: None,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let rules = vec![RuleProgram {
        root_readers: vec![RegionId(0), RegionId(0), RegionId(1)].into_boxed_slice(),
        original_steps: Box::new([]),
        emit: Ok(RegionId(0)),
        frontier: vec![Ok(InputSource::Region(RegionId(1)))].into_boxed_slice(),
    }]
    .into_boxed_slice();
    let file_readers = super::service::readers(&regions, &rules).unwrap();
    ExecutableProgram {
        file_readers,
        regions,
        rules,
        logical_regions: Box::new([]),
        observation_regions: Box::new([]),
    }
}
fn selected_reader_counts_keep_duplicate_edges_without_disabled_siblings() {
    let mut program = program();
    program.regions[0].dependencies = vec![RegionId(1), RegionId(1)].into_boxed_slice();
    let graph = super::service::readers(&program.regions, &program.rules).unwrap();
    let mut selected = std::collections::HashMap::from([(RegionId(0), ReaderMultiplicity(2))]);
    assert_eq!(
        super::service::remaining(&graph, &selected, RegionId(0)).unwrap(),
        ReaderMultiplicity(2)
    );
    assert_eq!(
        super::service::remaining(&graph, &selected, RegionId(1)).unwrap(),
        ReaderMultiplicity(2),
        "the same dependency is consumed once per declared edge"
    );
    selected.insert(RegionId(1), ReaderMultiplicity(1));
    assert_eq!(
        super::service::remaining(&graph, &selected, RegionId(1)).unwrap(),
        ReaderMultiplicity(3),
        "a source's external reader is independent of consumer edges"
    );
    selected.clear();
    assert_eq!(
        super::service::remaining(&graph, &selected, RegionId(1)).unwrap(),
        ReaderMultiplicity(0)
    );
    selected.insert(RegionId(99), ReaderMultiplicity(1));
    assert_eq!(
        super::service::remaining(&graph, &selected, RegionId(1)),
        Err(OperatingError::NotRun)
    );
}
enum Scenario {
    Mode(bool, bool, usize, usize, usize),
    Readers,
    DeferredSources,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("stored observation faults remain deferred under compiled demand"),
        word: Word::Succeeds,
        input: || Scenario::DeferredSources,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "selected reverse readers preserve duplicate edges and reject unknown roots",
        ),
        word: Word::Succeeds,
        input: || Scenario::Readers,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("undeclared readers all cancel"),
        word: Word::Succeeds,
        input: || Scenario::Mode(false, false, 0, 3, 0),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("diagnostic keeps one alias"),
        word: Word::Succeeds,
        input: || Scenario::Mode(true, false, 0, 2, 1),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("named reader stays independent"),
        word: Word::Succeeds,
        input: || Scenario::Mode(false, true, 0, 2, 1),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("observation capture excludes unrelated frontier"),
        word: Word::Succeeds,
        input: || Scenario::Mode(false, false, 1, 3, 0),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("full capture includes unrelated frontier"),
        word: Word::Succeeds,
        input: || Scenario::Mode(false, false, 2, 2, 1),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("all root aliases have distinct obligations"),
        word: Word::Succeeds,
        input: || Scenario::Mode(true, true, 2, 0, 3),
        expect: Expect::Value(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let (diagnostic, observations, capture, cancelled, roots) = match input {
        Scenario::Readers => {
            selected_reader_counts_keep_duplicate_edges_without_disabled_siblings();
            return Ok(());
        }
        Scenario::DeferredSources => {
            return deferred_sources();
        }
        Scenario::Mode(diagnostic, observations, capture, cancelled, roots) => {
            (diagnostic, observations, capture, cancelled, roots)
        }
    };
    let program = program();
    let prepared = compile(
        &program,
        RuleIndex(0),
        &[ObservationDemandSource {
            source: Ok(InputSource::Region(RegionId(0))),
            phase: StepScope::PerFile,
        }],
        &[FrontierPurpose::Diagnostic],
    )?;
    let selected = prepared.selected(FileDemand {
        diagnostic: if diagnostic {
            RootSelection::Required
        } else {
            RootSelection::Undemanded
        },
        observations: if observations {
            RootSelection::Required
        } else {
            RootSelection::Undemanded
        },
        capture: match capture {
            0 => FrontierSelection::Disabled,
            1 => FrontierSelection::Observations,
            _ => FrontierSelection::All,
        },
    });
    assert_eq!(
        program.rules()[0].root_readers.len() - selected.readers.len(),
        cancelled
    );
    assert_eq!(selected.roots.len(), roots);
    assert_eq!(selected.observations.len(), usize::from(observations));
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn deferred_sources() -> Result<(), OperatingError> {
    let mut program = program();
    program.observation_regions = vec![Err(OperatingError::NotRead)].into_boxed_slice();
    let prepared = compile(
        &program,
        RuleIndex(0),
        &[
            ObservationDemandSource {
                source: Ok(InputSource::Observation(0)),
                phase: StepScope::PerFile,
            },
            ObservationDemandSource {
                source: Err(OperatingError::WrongOperation),
                phase: StepScope::PerFile,
            },
        ],
        &[FrontierPurpose::Diagnostic],
    )?;
    assert!(prepared.selected(FileDemand::NONE).roots.is_empty());
    let named = prepared.selected(FileDemand {
        observations: RootSelection::Required,
        ..FileDemand::NONE
    });
    assert!(matches!(
        &named.roots[..],
        [
            FileRootDemand::Deferred(OperatingError::NotRead),
            FileRootDemand::Deferred(OperatingError::WrongOperation)
        ]
    ));
    assert!(named.readers.is_empty());
    assert_eq!(named.observations.len(), 2);
    Ok(())
}
