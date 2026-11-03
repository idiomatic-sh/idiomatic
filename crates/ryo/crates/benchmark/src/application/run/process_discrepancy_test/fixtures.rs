//! Small real-file fixtures; only child execution is substituted, not artifact or comparison policy.
use super::*;
use crate::domain::suite::process::types::ReferenceExpectation;
use crate::outbound::{
    inventory::types::{CapturedObservation, Registration},
    suite::{fixtures::fake::SuiteFake, types::ArtifactPath},
};
use sha2::Digest as _;
use std::path::PathBuf;

pub(super) struct Fixture {
    pub root: tempfile::TempDir,
    pub suite: SuiteFake,
    pub prepared: PreparedCase,
    pub rules: PathBuf,
    pub case: Value,
    receipt: PathBuf,
}
fn pin(bytes: &[u8]) -> String {
    format!("{:x}", sha2::Sha256::digest(bytes))
}
impl Fixture {
    pub fn new(name: &str) -> Self {
        Self::build(name, false)
    }
    pub fn with_paths(name: &str) -> Self {
        Self::build(name, true)
    }
    fn build(name: &str, paths: bool) -> Self {
        let prefix = if paths && cfg!(unix) {
            "日本-quote\"-slash\\-"
        } else if paths {
            "日本-"
        } else {
            "process-"
        };
        let root = tempfile::Builder::new().prefix(prefix).tempdir().unwrap();
        let source = root.path().canonicalize().unwrap();
        std::fs::write(source.join("case.py"), b"x = 1\n").unwrap();
        let rules = source.join("rules");
        let receipt = source.join("cases.jsonl");
        let profile = Tool {
            project_context_mode: ProjectContextMode::Isolated,
            tool: name.into(),
            version: "1".into(),
            sha256: vec!["a".repeat(64).into()],
            defaults: vec![ryo::linter::RuleId::try_from("A001").unwrap()],
            deferred: BTreeMap::new(),
            pending: BTreeMap::new(),
            query_modules: BTreeMap::new(),
            rule_applicability: BTreeMap::new(),
            parser_configuration: Some("python3.10".into()),
            source_language: None,
        };
        let expected = RunReport {
            execution_metadata: ExecutionMetadata::Complete,
            project_context: None,
            diagnostics: vec![],
            skipped: vec![],
            coverage: Some(RuleCoverage {
                selected: 1,
                evaluated: 1,
                skipped: 0,
                unknown_or_unread: 0,
            }),
            files: vec![ryo::linter::RelativePath("case.py".into())],
        };
        let mut prepared = PreparedCase {
            comparison: CaseComparison::Full,
            rule: ryo::linter::RuleId::try_from("A001").unwrap(),
            name: name.into(),
            request: RunRequest {
                root: source.clone(),
                state: source.join("state"),
                files: expected.files.clone(),
                context_manifest: None,
            },
            expected,
            reference_discrepancy: None,
        };
        let mut suite = SuiteFake {
            case_receipt: Some(receipt.clone()),
            profile: Some(profile.clone()),
            discrepancy_artifacts_valid: true,
            ..Default::default()
        };
        suite
            .artifact_digests
            .insert(PathBuf::from("fixture"), ArtifactDigest("a".repeat(64)));
        suite.artifact_digests.insert(
            source.join("benchmark.toml"),
            ArtifactDigest("c".repeat(64)),
        );
        let linter = crate::outbound::linter::fixtures::fake::LinterFake::default();
        let input = source.join("case.py");
        let (stdout, stderr, stdout_parts, stderr_parts) = if paths {
            let quoted = serde_json::to_string(input.to_str().unwrap()).unwrap();
            (
                format!("{{\"file\":{quoted},\"provider\":\"different\"}}").into_bytes(),
                format!("open {}\n", input.display()).into_bytes(),
                json!([{"kind":"literal","value":"{\"file\":\""},{"kind":"input-path","file":"case.py","encoding":"json-string-content"},{"kind":"literal","value":"\",\"provider\":\"different\"}"}]),
                json!([{"kind":"literal","value":"open "},{"kind":"input-path","file":"case.py","encoding":"utf8-bytes"},{"kind":"literal","value":"\n"}]),
            )
        } else {
            (
                b"payload".to_vec(),
                b"issue".to_vec(),
                json!([{"kind":"literal","value":"payload"}]),
                json!([{"kind":"literal","value":"issue"}]),
            )
        };
        let execution = Streams { stdout, stderr };
        let mut capture = case_execution::run(&suite, &linter, &execution, &prepared, &rules)
            .capture
            .unwrap();
        let source_digest = suite
            .case_source_digest(ArtifactPath(&source), &["case.py".into()])
            .unwrap();
        capture.invocation["source_sha256"] = json!(source_digest);
        let settings = capture.invocation["settings_sha256"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut observations = Vec::new();
        for attempt in 0..2 {
            let directory = source.join(format!("archive-{attempt}"));
            std::fs::create_dir(&directory).unwrap();
            let mut row = json!({"executable-sha256":"a".repeat(64)});
            for (label, file, bytes) in [
                (
                    "invocation",
                    "invocation.json",
                    serde_json::to_vec(&capture.invocation).unwrap(),
                ),
                ("stdout", "stdout.bin", capture.output.stdout.0.clone()),
                ("stderr", "stderr.bin", capture.output.stderr.0.clone()),
            ] {
                std::fs::write(directory.join(file), &bytes).unwrap();
                row[label] =
                    json!({"path":format!("archive-{attempt}/{file}"),"sha256":pin(&bytes)});
            }
            observations.push(row);
        }
        let wire = json!({"kind":"process-failure","exit":2,"stdout":stdout_parts,"stderr":stderr_parts,"observations":observations});
        let process =
            process_discrepancy::convert(serde_json::from_value(wire.clone()).unwrap()).unwrap();
        let identity = ReferenceDiscrepancyIdentity {
            tool: profile.tool.clone(),
            version: profile.version.clone(),
            observed_executable_sha256: ArtifactDigest("a".repeat(64)),
            profile_sha256: ArtifactDigest("c".repeat(64)),
            settings_sha256: ArtifactDigest(settings.clone()),
            source_sha256: ArtifactDigest(source_digest.clone()),
            reference_output_sha256: process_discrepancy::output_digest(&process),
            intended_output_sha256: discrepancy::output_digest(&prepared.expected),
        };
        let proof = json!({
            "actual_executable_sha256":"a".repeat(64),"settings_sha256":settings,"profile_sha256":identity.profile_sha256.0,
            "source_sha256":source_digest,"reference_output_sha256":identity.reference_output_sha256.0,"intended_output_sha256":identity.intended_output_sha256.0,
            "reference_expectation_kind":"process-failure","process_observation_group_sha256":process_discrepancy::group_digest(&process,&"a".repeat(64)).unwrap().0,
        });
        prepared.reference_discrepancy = Some(PreparedReferenceDiscrepancy {
            expected: ReferenceExpectation::ProcessFailure(process),
            identity: identity.clone(),
            evidence: vec![],
            reason: "independent toy language expectation".into(),
        });
        process_discrepancy::validate_archives(
            &suite,
            &source,
            &profile,
            &prepared,
            &identity,
            &ArtifactDigest("a".repeat(64)),
        )
        .unwrap();
        process_discrepancy::matches(&prepared, &capture, &proof, &suite, &rules).unwrap();
        case_observations::write(
            &suite,
            &prepared,
            "reference",
            None,
            None,
            true,
            Some(&proof),
            Some(&capture),
        )
        .unwrap();
        let candidate_execution =
            Observer(crate::outbound::execution::fixtures::fake::ExecutionFake {
                status: 0,
                ..Default::default()
            });
        let candidate =
            case_execution::run(&suite, &linter, &candidate_execution, &prepared, &rules);
        assert!(candidate.capture.is_none());
        case_observations::write(
            &suite,
            &prepared,
            "candidate",
            Some(&candidate.result.unwrap().report),
            None,
            true,
            Some(&proof),
            None,
        )
        .unwrap();
        let case = json!({
            "rule":"A001","name":name,"files":["case.py"],"expected":[],"comparison":"full","expectation_contract":"diagnostics","project_context_mode":"isolated","source_sha256":source_digest,
            "reference_discrepancy":{
                "intended_expected":[],"process":wire,"approved_executable_sha256":["a".repeat(64)],
                "identity":{"tool":name,"version":"1","settings-sha256":identity.settings_sha256.0,"profile-sha256":identity.profile_sha256.0,"source-sha256":source_digest,"reference-output-sha256":identity.reference_output_sha256.0,"intended-output-sha256":identity.intended_output_sha256.0},
            }
        });
        Self {
            root,
            suite,
            prepared,
            rules,
            case,
            receipt,
        }
    }
    pub fn load(&self) -> CapturedObservation {
        crate::outbound::inventory::factory::local()
            .observations(&self.receipt)
            .unwrap()
    }
    pub fn registration(&self) -> Registration {
        serde_json::from_value(json!({"provider":"toy","defaults":["A001"],"rules":[{"rule_id":"A001","command":null,"reason":null,"cases":[self.case],"instance":null}]})).unwrap()
    }
}

/// Different observed byte streams, independent of any certificate matching implementation.
struct Streams {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
impl ExecutionPort for Streams {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn observes_completed_processes_without_deadline(&self) -> bool {
        true
    }
    fn run(
        &self,
        _: Invocation,
    ) -> Result<ProcessOutput, crate::domain::diagnostics::errors::DiagnosticsError> {
        Ok(ProcessOutput {
            status: ProcessExitCode(2),
            signal: None,
            stdout: CapturedBytes(self.stdout.clone()),
            stderr: CapturedBytes(self.stderr.clone()),
            wall: std::time::Duration::ZERO,
            resources: ResourceObservation::WallOnly,
        })
    }
}
