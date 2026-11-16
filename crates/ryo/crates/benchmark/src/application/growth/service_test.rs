//! The one growth service: a shape whose CPU more than doubles (2.2 times its floored half) or whose peak passes the plan's
//! memory bound fails the bench; any other is published with its ratio, its whole-size time and its peak.
use super::service::grow;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::{
    CpuFloor, GrownProgram, GrowthPlan, GrowthSize, MemoryBound, RuleModules, ShapePrograms,
    SourceFileName,
};
use crate::outbound::process_meter::fixtures::fake::ProcessMeterFake;
use crate::outbound::suite::types::ArtifactPath;
use ryo_benchmark_contracts::domain::measurements::types::ResidentBytes;
use ryo_benchmark_contracts::domain::measurements::types::{DoublingRatio, GrowthShape};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, InputDeclaration, ParserSelection, ProviderSelection,
    RuleContextPreparation, SearchRoots, WorkerRequest, WorkerState,
};
use std::collections::BTreeMap;
use std::time::Duration;

/// How a scenario's meter reads: the CPU time of each lint at the half and the whole size, the peaks at each, and the
/// whole size of the shape.
#[derive(Clone, Copy, Debug)]
struct Scenario {
    half_ms: u64,
    whole_ms: u64,
    half_peak_mb: u64,
    whole_peak_mb: u64,
    size: usize,
}
type Input = Scenario;
/// The published ratio, in hundredths, and how many scenes hold the plan's one source file.
type Value = (u64, usize);
type Failure = Refusal;
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// A shape over the CPU limit, its shape and its ratio named.
    Cpu,
    /// A shape over the memory bound, its shape and its peak named.
    Memory,
    /// Anything else.
    Unexpected(String),
}

/// A megabyte, as the plan's bound counts.
const MB: u64 = 1_000_000;
/// How many lints of each size a shape takes: the fastest is its time.
const LINTS: usize = 5;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a shape at 2.19 times its half is published with that ratio"),
        word: Word::Succeeds,
        input: || Scenario {
            half_ms: 1_000,
            whole_ms: 2_190,
            half_peak_mb: 150,
            whole_peak_mb: 200,
            size: 1_000,
        },
        expect: Expect::Value((219, 2)),
    },
    Case {
        name: CaseName(
            "a shape at 2.21 times its half fails the bench, naming the shape and its ratio",
        ),
        word: Word::Refuses,
        input: || Scenario {
            half_ms: 1_000,
            whole_ms: 2_210,
            half_peak_mb: 150,
            whole_peak_mb: 200,
            size: 1_000,
        },
        expect: Expect::Error(Refusal::Cpu),
    },
    Case {
        name: CaseName(
            "a half under the floor counts as the floor, so a fixed start-up does not decide",
        ),
        word: Word::Succeeds,
        input: || Scenario {
            half_ms: 10,
            whole_ms: 600,
            half_peak_mb: 150,
            whole_peak_mb: 200,
            size: 1_000,
        },
        expect: Expect::Value((200, 2)),
    },
    Case {
        name: CaseName(
            "a peak of 400 MB at 1,000 repetitions passes the 260 MB bound and fails the bench",
        ),
        word: Word::Refuses,
        input: || Scenario {
            half_ms: 1_000,
            whole_ms: 1_500,
            half_peak_mb: 200,
            whole_peak_mb: 400,
            size: 1_000,
        },
        expect: Expect::Error(Refusal::Memory),
    },
    Case {
        name: CaseName(
            "a peak that grows 2.5 times over the 120 MB floor fails the bench under its bound",
        ),
        word: Word::Refuses,
        input: || Scenario {
            half_ms: 1_000,
            whole_ms: 1_500,
            half_peak_mb: 100,
            whole_peak_mb: 300,
            size: 2_000,
        },
        expect: Expect::Error(Refusal::Memory),
    },
];

/// Ruff's floor and bound, every language's: 0.3 s, and 160 MB plus 100 MB a thousand, doubling at most 2.4 over 120 MB.
fn plan(scenes: &std::path::Path, size: usize) -> GrowthPlan {
    let rule = ryo::linter::RuleId::try_from("F401").unwrap_or_else(|_| unreachable!("a rule id"));
    GrowthPlan {
        floor: CpuFloor(Duration::from_millis(300)),
        memory: MemoryBound {
            base: ResidentBytes(160 * MB),
            per_thousand: ResidentBytes(100 * MB),
            doubling: DoublingRatio(2.4),
            floor: ResidentBytes(120 * MB),
        },
        file: SourceFileName("case.py".into()),
        request: WorkerRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Explicit(vec![rule.clone()]),
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Unconfigured,
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode: Default::default(),
            context_manifest: ContextSeal::Unsealed,
            root: scenes.to_path_buf(),
            rules: scenes.join("rules"),
            query_modules: BTreeMap::new(),
            rule_applicability: BTreeMap::new(),
            parser_configuration: ParserSelection::Unspecified,
            source_language: ProviderSelection::Unspecified,
            files: Vec::new(),
            state: WorkerState::Fresh,
        },
        shapes: BTreeMap::from([(
            GrowthShape("F401/module/assign".into()),
            ShapePrograms {
                rule,
                modules: RuleModules::default(),
                half: GrownProgram("v = 1\n".repeat(size / 2)),
                whole: GrownProgram("v = 1\n".repeat(size)),
                size: GrowthSize(size),
            },
        )]),
    }
}

/// The meter's script: each lint reads the clock before and after it and the peak once, the half size's lints first.
fn meter(scenario: Scenario) -> ProcessMeterFake {
    let lint = |milliseconds: u64| [Duration::ZERO, Duration::from_millis(milliseconds)];
    let cpu = std::iter::repeat_n(lint(scenario.half_ms), LINTS)
        .chain(std::iter::repeat_n(lint(scenario.whole_ms), LINTS))
        .flatten()
        .collect();
    let peaks = std::iter::repeat_n(ResidentBytes(scenario.half_peak_mb * MB), LINTS)
        .chain(std::iter::repeat_n(
            ResidentBytes(scenario.whole_peak_mb * MB),
            LINTS,
        ))
        .collect();
    ProcessMeterFake::new(cpu, peaks)
}

/// How many directories below `scenes` hold a file named `case.py`.
fn scenes_holding_the_source(scenes: &std::path::Path) -> Result<usize, Refusal> {
    let mut holding = 0;
    for entry in std::fs::read_dir(scenes).map_err(|error| Refusal::Unexpected(error.to_string()))? {
        let entry = entry.map_err(|error| Refusal::Unexpected(error.to_string()))?;
        if entry.path().join("case.py").is_file() {
            holding += 1;
        }
    }
    Ok(holding)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let scratch = tempfile::tempdir().map_err(|error| Refusal::Unexpected(error.to_string()))?;
    let scenes = scratch.path().join("scenes");
    std::fs::create_dir_all(&scenes).map_err(|error| Refusal::Unexpected(error.to_string()))?;
    let grown = grow(
        &|_request: &WorkerRequest| Ok(()),
        &crate::outbound::suite::factory::local(),
        &meter(scenario),
        ArtifactPath(&scenes),
        &plan(&scenes, scenario.size),
    )
    .map_err(|error| {
        let said = error.to_string();
        if said.contains("F401/module/assign") && said.contains("ratio") {
            Refusal::Cpu
        } else if said.contains("F401/module/assign") && said.contains("peak") {
            Refusal::Memory
        } else {
            Refusal::Unexpected(said)
        }
    })?;
    let ratio = grown
        .0
        .get(&GrowthShape("F401/module/assign".into()))
        .map(|shape| (shape.ratio.0 * 100.0).round())
        .ok_or_else(|| Refusal::Unexpected("the shape was not published".into()))?;
    Ok((
        format!("{ratio}")
            .parse()
            .map_err(|_unparsed| Refusal::Unexpected(format!("ratio {ratio}")))?,
        scenes_holding_the_source(&scenes)?,
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
