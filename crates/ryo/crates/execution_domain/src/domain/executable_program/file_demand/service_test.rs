//! File root activation preserves a reader multiset and independent intent masks.
use super::{service::compile, types::*};
use crate::domain::rule_intent::types::FrontierPurpose;
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{
        InputSource, ObservationPlace, ReaderMultiplicity, RegionId, RuleProgram,
        executable_program::value::ExecutableProgram, region::value::Region,
    },
    operating::errors::OperatingError,
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::plan_parts::types::StepScope,
};
/// Two per-file regions, each depending on what `dependencies` names, over one rule whose roots read region 0 twice and
/// region 1 once.
fn build(
    dependencies: [Box<[RegionId]>; 2],
    observation_regions: Box<[Result<RegionId, OperatingError>]>,
) -> ExecutableProgram {
    let regions = dependencies
        .into_iter()
        .enumerate()
        .map(|(at, dependencies)| {
            let mut region = Region::new(
                RuleIndex(0),
                Ok(Box::new([])),
                StepScope::PerFile,
                Box::new([]),
                Box::new([]),
                ReaderMultiplicity(1),
            );
            region.set_dependencies(dependencies);
            region
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
    ExecutableProgram::new(
        regions,
        file_readers,
        rules,
        Box::new([]),
        observation_regions,
    )
}
fn program() -> ExecutableProgram {
    build([Box::new([]), Box::new([])], Box::new([]))
}
fn counted(
    program: &ExecutableProgram,
    selected: &std::collections::HashMap<RegionId, ReaderMultiplicity>,
    producer: RegionId,
) -> Result<ReaderMultiplicity, OperatingError> {
    walked(program, selected, &AbsentRoots::default(), producer)
}
fn walked(
    program: &ExecutableProgram,
    selected: &std::collections::HashMap<RegionId, ReaderMultiplicity>,
    absent: &AbsentRoots,
    producer: RegionId,
) -> Result<ReaderMultiplicity, OperatingError> {
    let mut roots: Vec<SelectedRoot> = selected
        .iter()
        .map(|(region, readers)| SelectedRoot {
            region: *region,
            readers: *readers,
        })
        .collect();
    roots.sort_unstable_by_key(|root| root.region.0);
    super::service::table(program.regions(), &roots, absent)
        .and_then(|table| super::service::reader_count(&table, producer))
}
fn selected_reader_counts_keep_duplicate_edges_without_disabled_siblings() {
    let program = build(
        [vec![RegionId(1), RegionId(1)].into_boxed_slice(), Box::new([])],
        Box::new([]),
    );
    let mut selected = std::collections::HashMap::from([(RegionId(0), ReaderMultiplicity(2))]);
    assert_eq!(
        counted(&program, &selected, RegionId(0)).unwrap(),
        ReaderMultiplicity(2)
    );
    assert_eq!(
        counted(&program, &selected, RegionId(1)).unwrap(),
        ReaderMultiplicity(2),
        "the same dependency is consumed once per declared edge"
    );
    selected.insert(RegionId(1), ReaderMultiplicity(1));
    assert_eq!(
        counted(&program, &selected, RegionId(1)).unwrap(),
        ReaderMultiplicity(3),
        "a source's external reader is independent of consumer edges"
    );
    selected.clear();
    assert_eq!(
        counted(&program, &selected, RegionId(1)).unwrap(),
        ReaderMultiplicity(0)
    );
    selected.insert(RegionId(99), ReaderMultiplicity(1));
    assert_eq!(
        counted(&program, &selected, RegionId(1)),
        Err(OperatingError::NotRun)
    );
}
/// Rule A's absent root (region 1) reads X (region 2), which reads Y (region 3); A is also an input of admitted rule F's root
/// (region 0). The walk counts A for F's edge and its own selection, and never follows A's edges: X and Y get no count. When
/// F also reads Y directly, Y is counted once, through F.
fn absent_roots_stop_the_walk_at_any_depth() {
    let program = |shared: bool| {
        let f: Box<[RegionId]> = if shared {
            vec![RegionId(1), RegionId(3)].into_boxed_slice()
        } else {
            vec![RegionId(1)].into_boxed_slice()
        };
        let regions = [f, vec![RegionId(2)].into_boxed_slice(), vec![RegionId(3)].into_boxed_slice(), Box::new([])]
            .into_iter()
            .map(|dependencies| {
                let mut region = Region::new(
                    RuleIndex(0),
                    Ok(Box::new([])),
                    StepScope::PerFile,
                    Box::new([]),
                    Box::new([]),
                    ReaderMultiplicity(1),
                );
                region.set_dependencies(dependencies);
                region
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let rules = vec![RuleProgram {
            root_readers: vec![RegionId(0), RegionId(1)].into_boxed_slice(),
            original_steps: Box::new([]),
            emit: Ok(RegionId(0)),
            frontier: Box::new([]),
        }]
        .into_boxed_slice();
        let file_readers = super::service::readers(&regions, &rules).unwrap();
        ExecutableProgram::new(regions, file_readers, rules, Box::new([]), Box::new([]))
    };
    let selected = std::collections::HashMap::from([
        (RegionId(0), ReaderMultiplicity(1)),
        (RegionId(1), ReaderMultiplicity(1)),
    ]);
    let mut regions = AbsentRegions::none(crate::domain::readiness::types::RegionCount(4));
    regions.insert(RegionId(1));
    let absent = AbsentRoots::decided(regions);
    let alone = program(false);
    assert_eq!(walked(&alone, &selected, &absent, RegionId(1)).unwrap(), ReaderMultiplicity(2));
    assert_eq!(walked(&alone, &selected, &absent, RegionId(2)).unwrap(), ReaderMultiplicity(0));
    assert_eq!(walked(&alone, &selected, &absent, RegionId(3)).unwrap(), ReaderMultiplicity(0));
    assert_eq!(
        walked(&alone, &selected, &AbsentRoots::default(), RegionId(3)).unwrap(),
        ReaderMultiplicity(1),
        "with no absent root, the walk is today's table"
    );
    let shared = program(true);
    assert_eq!(walked(&shared, &selected, &absent, RegionId(2)).unwrap(), ReaderMultiplicity(0));
    assert_eq!(walked(&shared, &selected, &absent, RegionId(3)).unwrap(), ReaderMultiplicity(1));
}
enum Scenario {
    Mode(bool, bool, usize, usize, usize),
    Readers,
    DeferredSources,
    AbsentDepth,
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
        name: CaseName("an absent root's edges are never counted, at any depth"),
        word: Word::Succeeds,
        input: || Scenario::AbsentDepth,
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
        Scenario::AbsentDepth => {
            absent_roots_stop_the_walk_at_any_depth();
            return Ok(());
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
    let program = build(
        [Box::new([]), Box::new([])],
        vec![Err(OperatingError::NotRead)].into_boxed_slice(),
    );
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
