//! A run is admitted only when each contestant's command launches what its certified probe's command launched.
use super::super::super::service::admit;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::{audit::value::Audit, certificate::value::Certificate};
use crate::outbound::linter::fixtures::fake::LinterFake;
use crate::outbound::linter::interface::LinterPort;
use crate::outbound::linter::types::CanonicalText;
use crate::outbound::suite::interface::SuitePort as _;
use benchmark_contracts::domain::context::types::PreparedRuleContext;
use benchmark_contracts::domain::measurements::types::{
    Invocation, MeasurementMode, ProcessOutput,
};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::RunReport;
use benchmark_contracts::domain::requests::types::{CertificationCases, RunRequest};
use benchmark_contracts::domain::workloads::types::version_line::value::VersionLine;
use benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};
use std::path::{Path, PathBuf};

/// Whether the candidate's command for a run launches another program than its command for the probe.
type Input = bool;
/// The run was admitted.
type Value = ();
/// The refusal's text.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a run launching what its probe launched is admitted"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a run launching another program than its probe is refused"),
        word: Word::Refuses,
        input: || true,
        expect: Expect::Error(super::super::super::constants::CHANGED_EXECUTABLE),
    },
];

/// A contestant whose command launches `probe` for the probe's root and `run` for any other.
#[derive(Debug)]
struct Launching {
    /// The probe request's root.
    probe_root: PathBuf,
    /// What the command launches for the probe.
    probe: PathBuf,
    /// What the command launches for a run.
    run: PathBuf,
    /// Everything else.
    inner: LinterFake,
}
impl LinterPort for Launching {
    fn rule_configuration(
        &self,
    ) -> benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        self.inner.rule_configuration()
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        let mut invocation = self.inner.prepare(request)?;
        invocation.executable = if request.root == self.probe_root {
            self.probe.clone()
        } else {
            self.run.clone()
        };
        Ok(invocation)
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        self.inner.decode(request, output)
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        case: &PreparedCase,
        profile: &Tool,
    ) -> Result<CanonicalText, Error> {
        self.inner.settings_text(normalized, case, profile)
    }
    fn rule_context_text(&self, prepared: &PreparedRuleContext) -> CanonicalText {
        self.inner.rule_context_text(prepared)
    }
    fn version_line(&self, tool: &Tool) -> VersionLine {
        self.inner.version_line(tool)
    }
}

fn subject(swapped: Input) -> Result<Value, Failure> {
    admitted(swapped).map_err(|error| &*Box::leak(error.into_boxed_str()))
}

/// Admit a run of the two contestants, the candidate's run launching another program than its probe when `swapped`.
fn admitted(swapped: bool) -> Result<(), String> {
    let suite = crate::outbound::suite::factory::local();
    let execution = crate::outbound::execution::fixtures::fake::ExecutionFake::default();
    let probe_root = Path::new("probe");
    let launching = |run: &str| Launching {
        probe_root: probe_root.to_owned(),
        probe: PathBuf::from("certified"),
        run: PathBuf::from(run),
        inner: LinterFake::default(),
    };
    let (reference, candidate) = (
        launching("certified"),
        launching(if swapped { "another" } else { "certified" }),
    );
    let probe = crate::outbound::suite::fixtures::samples::request(probe_root);
    let certificate = Certificate {
        case_source: CertificationCases::Profile,
        coverage: Audit {
            implemented: Vec::new(),
            missing: Vec::new(),
        },
        paths: Vec::new(),
        fingerprint: suite
            .fingerprint(crate::outbound::suite::types::ArtifactSet(&[]))
            .map_err(|error| error.to_string())?,
        profiles: [
            reference
                .prepare(&probe)
                .map_err(|error| error.to_string())?,
            candidate
                .prepare(&probe)
                .map_err(|error| error.to_string())?,
        ],
        probe,
        execution: crate::outbound::execution::interface::ExecutionPort::profile(&execution),
        cases: benchmark_contracts::domain::workloads::types::case_count::value::CaseCount(1),
    };
    let run_root = tempfile::tempdir().map_err(|error| error.to_string())?;
    admit(
        &certificate,
        &MeasurementMode::Diagnostic,
        &suite,
        &execution,
        &reference,
        &candidate,
        &crate::outbound::suite::fixtures::samples::request(run_root.path()),
    )
    .map(|_baseline| ())
    .map_err(|error| error.to_string())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
