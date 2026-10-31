//! Certification seals executed cases, workload patches and contestant identity.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::{
        errors::Error,
        types::{Diagnostic, RunReport},
    },
    measurements::types::{Invocation, ProcessOutput, RunOrder},
    suite::types::{CoverageScope, RunRequest},
};
use crate::outbound::linter::interface::LinterPort;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
#[derive(Clone, Copy)]
enum Change {
    None,
    Patch,
    Manifest,
    Query,
    Executable,
    Command,
}
const CASES: &[Case<Change, usize, ()>] = &[
    Case {
        name: CaseName("unchanged certified workload measures"),
        word: Word::Succeeds,
        input: || Change::None,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("patch mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Patch,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("project pin mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Manifest,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("query mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Query,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("executable mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Executable,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("invocation change refuses measurement"),
        word: Word::Refuses,
        input: || Change::Command,
        expect: Expect::Error(()),
    },
];
#[derive(Debug)]
struct Contestant {
    executable: PathBuf,
    arguments: Vec<std::ffi::OsString>,
}
impl LinterPort for Contestant {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        Ok(Invocation {
            executable: self.executable.clone(),
            arguments: self.arguments.clone(),
            environment: Default::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, _: ProcessOutput) -> Result<RunReport, Error> {
        let diagnostics = request
            .files
            .iter()
            .filter_map(|file| {
                let source = std::fs::read_to_string(request.root.join(file)).unwrap();
                (source == "x").then(|| Diagnostic {
                    rule: "A".into(),
                    file: file.clone(),
                    start: 0,
                    end: 1,
                })
            })
            .collect();
        Ok(RunReport {
            files: request.files.clone(),
            diagnostics,
        })
    }
}
#[derive(Debug, Default)]
struct Observations(crate::outbound::execution::fixtures::fake::ExecutionFake);
impl crate::outbound::execution::interface::ExecutionPort for Observations {
    fn run(&self, mut invocation: Invocation) -> Result<ProcessOutput, Error> {
        invocation.executable = "fixture".into();
        crate::outbound::execution::interface::ExecutionPort::run(&self.0, invocation)
    }
}
#[cfg(unix)]
fn subject(change: Change) -> Result<usize, ()> {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("profile");
    let rules = directory.join("rules");
    std::fs::create_dir_all(rules.join("A")).unwrap();
    let executable = root.path().join("tool");
    let bytes = b"#!/bin/sh\necho fixture 1\n";
    std::fs::write(&executable, bytes).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(
        directory.join("benchmark.toml"),
        format!(
            "tool = 'fixture'\nversion = '1'\nsha256 = '{:x}'\ndefaults = ['A']\n",
            Sha256::digest(bytes)
        ),
    )
    .unwrap();
    std::fs::write(rules.join("A/query.ryoql"), "query").unwrap();
    std::fs::write(rules.join("A/cases.toml"), "[[cases]]\nname='positive'\nfiles={'case.py'='x'}\nexpected=[{file='case.py', start=0, end=1}]\n[[cases]]\nname='negative'\nfiles={'case.py'='y'}\nexpected=[]\n").unwrap();
    let workload = root.path().join("workload");
    std::fs::create_dir(&workload).unwrap();
    std::fs::write(workload.join("benchmark.toml"), "pinned project").unwrap();
    std::fs::write(workload.join("patch.diff"), "pinned patch").unwrap();
    let suite = crate::outbound::suite::factory::local();
    let reference = Contestant {
        executable: executable.clone(),
        arguments: vec![],
    };
    let mut candidate = Contestant {
        executable: executable.clone(),
        arguments: vec![],
    };
    let execution = Observations(crate::outbound::execution::fixtures::fake::ExecutionFake {
        status: 0,
        ..Default::default()
    });
    let certificate = super::service::certify(
        &suite,
        &reference,
        &candidate,
        &execution,
        &directory,
        &workload,
        &executable,
        &executable,
        &rules,
        &root.path().join("cases"),
        CoverageScope::Accounted,
    )
    .unwrap();
    assert!(!certificate.fingerprint().is_empty());
    let measured_root = root.path().join("sources");
    std::fs::create_dir(&measured_root).unwrap();
    std::fs::write(measured_root.join("case.py"), "x").unwrap();
    let request = RunRequest {
        root: measured_root,
        files: vec!["case.py".into()],
        cache: root.path().join("cache"),
    };
    let changed = match change {
        Change::None => None,
        Change::Patch => Some(workload.join("patch.diff")),
        Change::Manifest => Some(workload.join("benchmark.toml")),
        Change::Query => Some(rules.join("A/query.ryoql")),
        Change::Executable => Some(executable),
        Change::Command => {
            candidate.arguments.push("different defaults".into());
            None
        }
    };
    if let Some(path) = changed {
        std::fs::write(path, "changed").unwrap();
    }
    let prior = execution.0.history.lock().unwrap().len();
    let result = crate::inbound::use_cases::measure::service::measure_certified(
        &certificate,
        &suite,
        &reference,
        &candidate,
        &execution,
        &request,
        RunOrder::ReferenceFirst,
    );
    if result.is_err() {
        assert_eq!(
            prior,
            execution.0.history.lock().unwrap().len(),
            "refusal must precede process launch"
        );
    }
    result.map(|_| certificate.cases()).map_err(|_| ())
}
#[test]
#[cfg(unix)]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
#[cfg(unix)]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
