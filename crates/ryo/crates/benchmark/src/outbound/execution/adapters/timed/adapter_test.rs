//! Built collector regression: resources belong to the actual child, not its large parent.
use ryo_benchmark::cases::Case;
use ryo_benchmark::cases::CaseName;
use ryo_benchmark::cases::Expect;
use ryo_benchmark::cases::Word;
use ryo_benchmark::cases::run;
use ryo_benchmark::outbound::execution;
use ryo_benchmark::outbound::execution::fixtures::contract::check as check_contract;
use ryo_benchmark::outbound::execution::interface::ExecutionPort as _;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use sha2::{Digest as _, Sha256};
use std::{path::PathBuf, time::Duration};
#[derive(Clone, Copy, Debug)]
enum Probe {
    Contract,
    ParentMemory,
    CpuAndMemory,
    Sleep,
    ChangedCollector,
}
type Input = Probe;
type Value = ();
type Failure = String;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("sleeping child wall time is not fabricated CPU time"),
        word: Word::Succeeds,
        input: || Probe::Sleep,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("isolated collector preserves launch semantics and both streams"),
        word: Word::Succeeds,
        input: || Probe::Contract,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("tiny child does not inherit256MiB resident parent"),
        word: Word::Succeeds,
        input: || Probe::ParentMemory,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("busy allocated child has measured CPU and resident bytes"),
        word: Word::Succeeds,
        input: || Probe::CpuAndMemory,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("changed collector bytes refuse measurement"),
        word: Word::Succeeds,
        input: || Probe::ChangedCollector,
        expect: Expect::Value(()),
    },
];
fn settings() -> Result<(PathBuf, ArtifactDigest), std::io::Error> {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_ryo-benchmark-collector"));
    let digest = ArtifactDigest(format!("{:x}", Sha256::digest(std::fs::read(&executable)?)));
    Ok((executable, digest))
}
fn subject(probe: Input) -> Result<Value, Failure> {
    check(probe).map_err(|error| error.to_string())
}
fn check(probe: Probe) -> Result<(), Box<dyn std::error::Error>> {
    let (executable, mut digest) = settings()?;
    if matches!(probe, Probe::ChangedCollector) {
        digest = ArtifactDigest("incorrect".into());
    }
    let execution = execution::factory::timed(executable, digest);
    if matches!(probe, Probe::Contract) {
        check_contract(&execution);
        return Ok(());
    }
    let mut invocation = Invocation {
        executable: PathBuf::from("/usr/bin/true"),
        arguments: Vec::new(),
        environment: std::collections::BTreeMap::default(),
        directory: PathBuf::from("/tmp"),
    };
    if matches!(probe, Probe::ChangedCollector) {
        assert!(
            matches!(execution.run(invocation), Err(ReportsError::Invalid(_))),
            "changed collector digest must refuse execution"
        );
        return Ok(());
    }
    let parent = if matches!(probe, Probe::ParentMemory) {
        vec![1_u8; 256 * 1024 * 1024]
    } else {
        Vec::new()
    };
    let _resident = std::hint::black_box(&parent);
    if matches!(probe, Probe::CpuAndMemory) {
        invocation.executable = PathBuf::from("/usr/bin/python3");
        invocation.arguments = vec![
            "-c".into(),
            "import time; allocation=bytearray(64*1024*1024); end=time.process_time()+0.2
while time.process_time()<end: pass"
                .into(),
        ];
    }
    if matches!(probe, Probe::Sleep) {
        invocation.executable = PathBuf::from("/usr/bin/python3");
        invocation.arguments = vec!["-c".into(), "import time; time.sleep(0.25)".into()];
    }
    let output = execution.run(invocation)?;
    assert_eq!(
        output.status.0, 0_i32,
        "resource probe must exit successfully"
    );
    assert!(
        output.wall > Duration::ZERO,
        "actual child must have elapsed time"
    );
    let ResourceObservation::Collected(resources) = output.resources else {
        return Err("collector omitted resources".into());
    };
    match probe {
        Probe::ParentMemory => assert!(
            resources.peak_rss_bytes.0 > 0 && resources.peak_rss_bytes.0 < 64 * 1024 * 1024,
            "tiny child RSS={} inherited parent",
            resources.peak_rss_bytes.0
        ),
        Probe::CpuAndMemory => {
            assert!(
                resources.cpu >= Duration::from_millis(180),
                "CPU={:?}",
                resources.cpu
            );
            assert!(
                resources.cpu <= output.wall + Duration::from_millis(50),
                "CPU exceeds wall for single-thread child"
            );
            assert!(
                resources.peak_rss_bytes.0 > 60 * 1024 * 1024
                    && resources.peak_rss_bytes.0 < 160 * 1024 * 1024,
                "RSS={}",
                resources.peak_rss_bytes.0
            );
        }
        Probe::Sleep => {
            assert!(
                output.wall >= Duration::from_millis(200),
                "sleep probe must retain elapsed wall time"
            );
            assert!(
                resources.cpu < output.wall / 2,
                "sleep CPU={:?} wall={:?}",
                resources.cpu,
                output.wall
            );
        }
        Probe::Contract | Probe::ChangedCollector => {
            return Err("early-return probe reached resource assertions".into());
        }
    }
    drop(std::hint::black_box(parent));
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
