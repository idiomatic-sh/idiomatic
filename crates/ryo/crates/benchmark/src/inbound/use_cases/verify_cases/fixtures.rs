//! Independent source inputs through the actual public use case and real process boundary.
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;

/// The public operation under test, given the ports, the rules and the scratch directory.
pub type Operate = fn(
    &dyn SuitePort,
    &dyn LinterPort,
    &dyn LinterPort,
    &dyn ExecutionPort,
    &Path,
    &Path,
) -> Result<usize, ReportsError>;
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    Complete,
    Missing,
    Unaccounted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    MissingSource,
    Unaccounted,
    Unexpected,
}
/// Run one public operation over independently authored source facts.
pub fn exercise(scenario: Scenario, operate: Operate) -> Result<usize, Refusal> {
    crate::inbound::use_cases::prepare_cases::fixtures::with_verification_sources(
        |suite, reference, candidate, execution, rules, scratch, launches| {
            if matches!(scenario, Scenario::Unaccounted) {
                let profile = rules.parent().unwrap().join("benchmark.toml");
                let text = std::fs::read_to_string(&profile)
                    .unwrap()
                    .replace("defaults=['A']", "defaults=['A','B']");
                std::fs::write(profile, text).unwrap();
                std::fs::create_dir(rules.join("B")).unwrap();
                std::fs::write(
                    rules.join("B/cases.toml"),
                    "[[cases]]\nname='oracle'\nfiles={'case.py'='x'}\nexpected=[]\n",
                )
                .unwrap();
            }
            let input = match scenario {
                Scenario::Complete | Scenario::Unaccounted => rules.to_owned(),
                Scenario::Missing => rules.join("missing"),
            };
            let result = operate(suite, reference, candidate, execution, &input, scratch);
            match scenario {
                Scenario::Complete => {
                    let actual = std::fs::read_to_string(launches);
                    assert_eq!(actual.unwrap().lines().count(), 4);
                }
                Scenario::Missing | Scenario::Unaccounted => assert_eq!(
                    std::fs::symlink_metadata(launches).unwrap_err().kind(),
                    std::io::ErrorKind::NotFound
                ),
            }
            result.map_err(|error| match error {
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(error)
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    Refusal::MissingSource
                }
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(
                    message,
                ) if message.contains("defaults lack implementation or explicit deferral") => {
                    Refusal::Unaccounted
                }
                _ => Refusal::Unexpected,
            })
        },
    )
}

/// Independent authored evidence, constructed apart from either contestant response.
#[cfg(test)]
pub(crate) mod shard {
    use crate::inbound::use_cases::verify_cases::service::verify_prepared_shard;
    use crate::outbound::execution::{fixtures::fake::ExecutionFake, interface::ExecutionPort};
    use crate::outbound::linter::{fixtures::fake::LinterFake, interface::LinterPort};
    use crate::outbound::receipts::{fixtures::fake::ReceiptsFake, types::CaseReceipts};
    use crate::outbound::suite::fixtures::{authored, fake::SuiteFake};
    use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
    use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
    use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
    use ryo_benchmark_contracts::domain::workloads::types::CaseComparison;
    use std::path::Path;

    /// The shape of the authored suite and of the findings both contestants report.
    #[derive(Clone, Copy, Debug)]
    pub(crate) enum Shape {
        Agreement,
        MissingCandidate,
        MissingReference,
        NoCases,
        StartupFailure,
        FullAgreement,
        MissingFrontend,
        RuleProjection,
    }

    /// Names what an error means by its message, so rows state the refusal and not its wording.
    #[derive(Debug, PartialEq, Eq)]
    pub(crate) enum Refusal {
        CandidateMismatch,
        ReferenceMismatch,
        CandidateStartup,
        ReferenceStartup,
        UnaccountedDefault,
        NoCases,
        InputCoverage,
        UnprovenEvidence,
        MetadataDrift,
        VerificationRequired,
        Unexpected,
    }

    /// The refusal an error message states.
    pub(crate) fn category(message: &str) -> Refusal {
        let by_wording = [
            ("expected-vs-candidate", Refusal::CandidateMismatch),
            ("expected-vs-reference", Refusal::ReferenceMismatch),
        ];
        let by_content = [
            ("reference: operational failure", Refusal::ReferenceStartup),
            ("defaults lack implementation or explicit deferral", Refusal::UnaccountedDefault),
            ("fixture has no independent discrepancy artifacts", Refusal::UnprovenEvidence),
            ("executable metadata drifted", Refusal::MetadataDrift),
            ("require verification preparation", Refusal::VerificationRequired),
        ];
        if message == "operational failure" {
            return Refusal::CandidateStartup;
        }
        if message == crate::domain::suite::constants::NO_CASES {
            return Refusal::NoCases;
        }
        if message == crate::domain::diagnostics::constants::INPUT_COVERAGE {
            return Refusal::InputCoverage;
        }
        by_wording
            .into_iter()
            .chain(by_content)
            .find_map(|(wording, refusal)| message.contains(wording).then_some(refusal))
            .unwrap_or(Refusal::Unexpected)
    }

    /// The number of launches an execution fake recorded.
    pub(crate) fn history_len(execution: &ExecutionFake) -> Option<usize> {
        execution.history.lock().ok().map(|history| history.len())
    }

    /// An execution that exits cleanly.
    pub(crate) fn succeeding() -> ExecutionFake {
        ExecutionFake {
            status: 0,
            ..Default::default()
        }
    }

    /// Paired verification of every prepared case of `suite`, recording through `receipts`.
    pub(crate) fn verify_shard(
        suite: &SuiteFake,
        receipts: &ReceiptsFake,
        reference: &dyn LinterPort,
        candidate: &dyn LinterPort,
        execution: &dyn ExecutionPort,
        rules: &Path,
    ) -> Result<usize, ReportsError> {
        verify_prepared_shard(
            suite,
            receipts,
            &crate::outbound::context::factory::local(),
            reference,
            candidate,
            execution,
            &crate::outbound::inventory::factory::local(),
            rules,
            suite.prepared.clone(),
            CaseSelection::default(),
        )
        .map(|count| count.0)
    }

    /// Sets the permission bits of `path`; a file system with no modes has none to set.
    #[cfg(unix)]
    pub(crate) fn set_mode(path: &Path, mode: u32) {
        std::fs::set_permissions(
            path,
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(mode),
        )
        .unwrap();
    }
    #[cfg(not(unix))]
    pub(crate) fn set_mode(_path: &Path, _mode: u32) {}

    /// A directory holding the case file `case.py`, with the permission bits `mode`.
    pub(crate) fn staged_case(mode: u32) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("case.py"), "same bytes").unwrap();
        set_mode(&root.path().join("case.py"), mode);
        root
    }

    /// Forgets the receipt file the fixture recorded, so a later verification records afresh.
    pub(crate) fn forget_receipts(receipts: &ReceiptsFake) {
        let CaseReceipts::Recorded(path) = &receipts.case_receipts else {
            unreachable!("fixture records receipts")
        };
        std::fs::remove_file(path).unwrap();
    }

    /// Construct authored evidence independently from either contestant response.
    pub(crate) fn prepared(shape: Shape) -> (SuiteFake, BenchmarkFinding) {
        let expected = authored::finding("E722", 0, 6);
        let mut suite = SuiteFake::default();
        if !matches!(shape, Shape::NoCases) {
            suite.prepared.push(authored::case(&expected));
        }
        suite.profile = Some(authored::profile());
        (suite, expected)
    }

    /// Give both contestants parser diagnostics, optionally retaining an unrelated lint as a control.
    pub(crate) fn report_scope(
        shape: Shape,
        suite: &mut SuiteFake,
        reference: &mut LinterFake,
        candidate: &mut LinterFake,
    ) {
        if matches!(shape, Shape::FullAgreement | Shape::MissingFrontend | Shape::RuleProjection) {
            let frontend = authored::finding("invalid-syntax", 9, 19);
            reference.diagnostics.push(frontend.clone());
            if !matches!(shape, Shape::MissingFrontend) {
                candidate.diagnostics.push(frontend.clone());
            }
            if matches!(shape, Shape::RuleProjection) {
                let unrelated = authored::finding("F401", 20, 24);
                reference.diagnostics.push(unrelated.clone());
                candidate.diagnostics.push(unrelated);
            }
            let case = suite.prepared.first_mut().expect("authored case");
            if !matches!(shape, Shape::RuleProjection) {
                case.comparison = CaseComparison::Full;
            }
            case.expected.diagnostics.push(frontend);
        }
    }
}

/// A documented reference difference keeps the intended contract and the pinned reference apart.
#[cfg(test)]
pub(crate) mod discrepancy {
    use super::shard::{Shape, prepared};
    use crate::outbound::linter::{fixtures::fake::LinterFake, interface::LinterPort};
    use crate::outbound::suite::fixtures::fake::SuiteFake;
    use crate::outbound::suite::types::{
        CaseClaim, ClaimedDiscrepancy, ClaimedEvidence, ClaimedIdentity, ClaimedMetadata, ClaimedProcess,
    };
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
    use ryo_benchmark_contracts::domain::documents::types::text::value::{CaseName, RuleName, Sha256Text};
    use ryo_benchmark_contracts::domain::requests::types::{ParserConfigurationName, ParserSelection};
    use ryo_benchmark_contracts::domain::workloads::types::{
        CaseComparison, DiscrepancyBasis, DiscrepancyReason,
        discrepancy_evidence::value::{LanguageVersion, ParserTarget, Rationale},
    };

    /// A suite whose one case documents how the pinned reference differs from the intended behavior.
    pub(crate) fn discrepancy_suite() -> SuiteFake {
        let (mut suite, _) = prepared(Shape::Agreement);
        let case = suite.prepared.first_mut().expect("authored case");
        case.comparison = CaseComparison::Full;
        case.request.root = "/case".into();
        let mut reference = case.expected.clone();
        reference.diagnostics.clear();
        let profile_hash = ArtifactDigest("a".repeat(64));
        let binary_hash = ArtifactDigest("b".repeat(64));
        let mut profile = suite.profile.take().expect("authored profile");
        profile.project_context_mode = ProjectContextMode::Isolated;
        profile.sha256 = vec![binary_hash.0.clone().into()];
        profile.defaults = vec![case.rule.clone()];
        profile.parser_configuration =
            ParserSelection::Named(ParserConfigurationName("python-target=3.10".into()));
        let invocation = LinterFake::default().prepare(&case.request).expect("fixture invocation");
        let settings = crate::domain::canonical_text::service::text_digest(
            &crate::application::discrepancy_validation::service::settings_text(
                &LinterFake::default(),
                &invocation,
                case,
                &profile,
            )
            .expect("fixture settings"),
        );
        let sha = |digest: &ArtifactDigest| Sha256Text(digest.0.clone());
        let evidence_digest = Sha256Text("d".repeat(64));
        let claim = CaseClaim {
            rule: RuleName(case.rule.to_string()),
            name: CaseName(case.name.to_string()),
            basis: DiscrepancyBasis::LanguageDefect,
            discrepancy: ClaimedDiscrepancy {
                process: ClaimedProcess::Absent,
                expected: Vec::new(),
                identity: ClaimedIdentity {
                    file_metadata_sha256: ClaimedMetadata::Undeclared,
                    tool: "fixture".into(),
                    version: "1".into(),
                    observed_executable_sha256: sha(&binary_hash),
                    profile_sha256: sha(&profile_hash),
                    settings_sha256: sha(&settings),
                    source_sha256: Sha256Text("c".repeat(64)),
                    reference_output_sha256: sha(&crate::domain::discrepancy::service::output_digest(&reference)),
                    intended_output_sha256: sha(&crate::domain::discrepancy::service::output_digest(&case.expected)),
                },
                evidence: vec![ClaimedEvidence::IndependentLanguageRun {
                    path: "evidence.json".into(),
                    sha256: evidence_digest,
                    language_version: LanguageVersion("3.10".into()),
                    target_configuration: ParserTarget("python-target=3.10".into()),
                    version_rationale: Rationale("exact target independently evaluated".into()),
                }],
                reason: DiscrepancyReason(
                    "independent language contract differs from the pinned reference".into(),
                ),
            },
        };
        suite.reference_claims.push(claim);
        suite.profile = Some(profile);
        let manifest = std::path::Path::new("/profile")
            .join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST);
        suite.artifact_digests.insert(manifest, profile_hash);
        suite.artifact_digests.insert("fixture".into(), binary_hash);
        suite.discrepancy_artifacts_valid = true;
        suite
    }
}

/// What a validator answers over a replayed receipt, and the case whose unselected context stays bound.
#[cfg(test)]
pub(crate) mod process_identity {
    use crate::outbound::suite::{
        interface::SuitePort as _,
        types::{ArtifactMetadata, ArtifactPath, ArtifactSet, CaseArtifacts},
    };
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    use ryo_benchmark_contracts::domain::requests::types::StagedSources;
    use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;

    /// What a validator answers over a replayed receipt: it admits the receipt or refuses it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum Verdict {
        /// The receipt passed.
        Accepted,
        /// The receipt was refused, however it was refused.
        Refused,
    }

    /// The verdict a result states.
    pub(crate) fn verdict<T, E>(result: Result<T, E>) -> Verdict {
        match result {
            Ok(_) => Verdict::Accepted,
            Err(_) => Verdict::Refused,
        }
    }

    /// A case whose staged context file is no diagnostic input, over the local suite boundary.
    pub(crate) struct ContextProbe {
        root: tempfile::TempDir,
        prepared: PreparedCase,
        files: Vec<std::path::PathBuf>,
        digest: ArtifactDigest,
    }
    impl ContextProbe {
        /// The case `case.py` staged beside the context file `local.py`, both listed.
        pub(crate) fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            std::fs::write(root.path().join("case.py"), "import local\n").unwrap();
            std::fs::write(root.path().join("local.py"), "value = 1\n").unwrap();
            let mut prepared = crate::domain::discrepancy::fixtures::case();
            prepared.request.root = root.path().into();
            prepared.request.staged_sources = StagedSources::Listed(
                vec!["case.py", "local.py"]
                    .into_iter()
                    .map(|path| ryo::linter::RelativePath(path.into()))
                    .collect(),
            );
            let files = crate::domain::suite::service::materialized_sources(&prepared.request);
            let digest = ArtifactDigest(
                crate::outbound::suite::factory::local()
                    .case_source_digest(ArtifactPath(root.path()), ArtifactSet(&files))
                    .unwrap()
                    .0,
            );
            Self {
                root,
                prepared,
                files,
                digest,
            }
        }
        /// How many files the case selects as diagnostic inputs.
        pub(crate) fn diagnostic_files(&self) -> usize {
            self.prepared.request.files.len()
        }
        /// What the suite boundary answers when it verifies the evidence of the listed sources.
        pub(crate) fn check(&self) -> Verdict {
            verdict(
                crate::outbound::suite::factory::local().verify_discrepancy_artifacts(CaseArtifacts {
                    profile: self.root.path(),
                    root: self.root.path(),
                    files: &self.files,
                    source_sha256: &self.digest,
                    file_metadata_files: &[],
                    file_metadata: ArtifactMetadata::Undeclared,
                    evidence: &[],
                }),
            )
        }
        /// Rewrites the context file `local.py`.
        pub(crate) fn write_context(&self, text: &str) {
            std::fs::write(self.root.path().join("local.py"), text).unwrap();
        }
        /// The sources the case materializes once none is listed.
        pub(crate) fn unlisted_sources(&mut self) -> Vec<std::path::PathBuf> {
            self.prepared.request.staged_sources = StagedSources::Unlisted;
            crate::domain::suite::service::materialized_sources(&self.prepared.request)
        }
    }
}

/// Small real-file fixtures; only child execution is substituted, not artifact or comparison policy.
#[cfg(test)]
pub(crate) mod process_capture {
    use super::process_identity::{Verdict, verdict};
    use crate::application::case_execution::service::run as run_case;
    use crate::application::case_observation::service::write as record;
    use crate::domain::discrepancy::types::{
        Contestant, OptionalDigest, ReferenceKind, ReferenceProof,
    };
    use crate::domain::process_failure::types::CaptureRecord;
    use crate::outbound::{
        inventory::{
            interface::InventoryPort as _,
            types::{RawArtifacts, Registration},
        },
        execution::fixtures::observers::{Observer, Streams},
        inventory::fixtures::registrations::{
            complete_of, document_of, invocation_bytes, process_failure, registration_of,
        },
        receipts::{
            fixtures::fake::ReceiptsFake, interface::ReceiptsPort as _, types::ReceiptPath,
        },
        suite::{
            fixtures::{authored, fake::SuiteFake},
            interface::SuitePort as _,
            types::ArtifactPath,
        },
    };
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
    use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
    use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
    use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
        ObservationDocument, ProcessArtifactsRecord, ReferenceProcess,
    };
    use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Sha256Text, YesNo};
    use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
    use ryo_benchmark_contracts::domain::process::types::{ProcessFailure, ReferenceExpectation};
    use ryo_benchmark_contracts::domain::requests::types::{ParserConfigurationName, ParserSelection};
    use ryo_benchmark_contracts::domain::workloads::types::{
        CaseComparison, PreparedCase, PreparedReferenceDiscrepancy, ReferenceDiscrepancyIdentity,
    };
    use serde_json::{Value, json};
    use sha2::Digest as _;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// The context port the verifier is given.
    pub(crate) fn context() -> impl crate::outbound::context::interface::ContextPort {
        crate::outbound::context::factory::local()
    }

    /// The rows of one receipt file, with the raw bytes and invocation records they point at.
    pub(crate) struct Loaded {
        pub records: Vec<ObservationDocument>,
        pub artifacts: BTreeMap<PathBuf, CapturedBytes>,
        pub invocations: BTreeMap<PathBuf, InvocationDocument>,
    }
    impl Loaded {
        /// The raw artifacts as the offline audit takes them.
        pub fn raw(&self) -> RawArtifacts {
            RawArtifacts {
                bytes: self.artifacts.clone(),
                invocations: self.invocations.clone(),
            }
        }
        /// The raw bytes the first row's observed reference process wrote to stdout and stderr.
        pub fn reference_streams(&self) -> (&CapturedBytes, &CapturedBytes) {
            let ReferenceProcess::Observed(observed) = &self.records[0].process_observation else {
                panic!("the reference process is observed");
            };
            let ProcessArtifactsRecord::Attached(pointers) = &observed.artifacts else {
                panic!("the observed process points at artifacts");
            };
            (
                &self.artifacts[std::path::Path::new(&pointers.stdout.path.0)],
                &self.artifacts[std::path::Path::new(&pointers.stderr.path.0)],
            )
        }
    }

    /// The `file` member of the JSON document the bytes spell.
    pub(crate) fn file_member(bytes: &CapturedBytes) -> String {
        serde_json::from_slice::<Value>(&bytes.0).unwrap()["file"]
            .as_str()
            .expect("the document names a file")
            .to_owned()
    }

    pub(crate) struct Fixture {
        pub root: tempfile::TempDir,
        pub suite: SuiteFake,
        pub prepared: PreparedCase,
        pub rules: PathBuf,
        case: Value,
        pub document: CaseDocument,
        pub proof: ReferenceProof,
        receipt: PathBuf,
        pub receipts: ReceiptsFake,
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
            let rule = ryo::linter::RuleId::try_from("A001").unwrap();
            let mut profile = authored::profile();
            profile.tool = name.into();
            profile.project_context_mode = ProjectContextMode::Isolated;
            profile.sha256 = vec!["a".repeat(64).into()];
            profile.defaults = vec![rule.clone()];
            profile.parser_configuration =
                ParserSelection::Named(ParserConfigurationName("python3.10".into()));
            let mut prepared = authored::case(&authored::finding("A001", 0, 0));
            prepared.comparison = CaseComparison::Full;
            prepared.rule = rule;
            prepared.name = name.into();
            prepared.expected.diagnostics.clear();
            prepared.request.root = source.clone();
            prepared.request.state = source.join("state");
            let mut suite = SuiteFake {
                profile: Some(profile.clone()),
                discrepancy_artifacts_valid: true,
                ..Default::default()
            };
            suite
                .artifact_digests
                .insert(PathBuf::from("fixture"), ArtifactDigest("a".repeat(64)));
            suite.artifact_digests.insert(
                source.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST),
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
            let CaptureRecord::Kept(mut capture) = run_case(
                &suite,
                &crate::outbound::context::factory::local(),
                &linter,
                &execution,
                &prepared,
                &rules,
            )
            .capture
            else {
                unreachable!("undecodable output keeps its raw bytes")
            };
            let source_digest = suite
                .case_source_digest(
                    ArtifactPath(&source),
                    crate::outbound::suite::types::ArtifactSet(&["case.py".into()]),
                )
                .unwrap()
                .0;
            complete_of(&mut capture.invocation).source_sha256 =
                Pin::Sha256(Sha256Text(source_digest.clone()));
            let settings = complete_of(&mut capture.invocation).settings_sha256.0.clone();
            let mut observations = Vec::new();
            for attempt in 0..2 {
                let directory = source.join(format!("archive-{attempt}"));
                std::fs::create_dir(&directory).unwrap();
                let mut row = json!({"executable-sha256":"a".repeat(64)});
                for (label, file, bytes) in [
                    (
                        "invocation",
                        "invocation.json",
                        invocation_bytes(&capture.invocation),
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
            let process = process_failure(&wire);
            let identity = ReferenceDiscrepancyIdentity {
                file_metadata_sha256: ryo_benchmark_contracts::domain::workloads::types::MetadataDigestDeclaration::Undeclared,
                tool: profile.tool.clone(),
                version: profile.version.clone(),
                observed_executable_sha256: ArtifactDigest("a".repeat(64)),
                profile_sha256: ArtifactDigest("c".repeat(64)),
                settings_sha256: ArtifactDigest(settings.clone()),
                source_sha256: ArtifactDigest(source_digest.clone()),
                reference_output_sha256: crate::domain::process_failure::service::output_digest(
                    &process,
                ),
                intended_output_sha256: crate::domain::discrepancy::service::output_digest(
                    &prepared.expected,
                ),
            };
            prepared.reference_discrepancy = ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Differs(PreparedReferenceDiscrepancy {
                basis:
                    ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::LanguageDefect,
                expected: ReferenceExpectation::ProcessFailure(process),
                identity: identity.clone(),
                evidence: vec![],
                reason: ryo_benchmark_contracts::domain::workloads::types::DiscrepancyReason("independent toy language expectation".into()),
            });
            crate::application::discrepancy_validation::service::validate_archives(
                &suite,
                &crate::outbound::inventory::factory::local(),
                &crate::outbound::linter::fixtures::fake::LinterFake::default(),
                &source,
                &profile,
                &prepared,
                match &differing(&prepared).expected {
                    ReferenceExpectation::ProcessFailure(process) => process,
                    ReferenceExpectation::Diagnostics(_) => unreachable!("registered process"),
                },
                &ArtifactDigest("a".repeat(64)),
            )
            .unwrap();
            let typed_proof = ReferenceProof {
                basis:
                    ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::LanguageDefect,
                process_group: OptionalDigest::Pinned(
                    crate::domain::process_failure::service::group_digest(
                        match &differing(&prepared).expected {
                            ReferenceExpectation::ProcessFailure(process) => process,
                            ReferenceExpectation::Diagnostics(_) => unreachable!("registered process"),
                        },
                        &ArtifactDigest("a".repeat(64)),
                    )
                    .unwrap(),
                ),
                kind: ReferenceKind::ProcessFailure,
                settings: identity.settings_sha256.clone(),
                executable: ArtifactDigest("a".repeat(64)),
                profile: identity.profile_sha256.clone(),
                source: identity.source_sha256.clone(),
                reference_output: identity.reference_output_sha256.clone(),
                intended_output: identity.intended_output_sha256.clone(),
                file_metadata: OptionalDigest::Absent,
            };
            crate::application::process_discrepancy::service::matches(
                &prepared,
                &capture.invocation,
                &capture.output,
                &typed_proof,
                &suite,
                &rules,
            )
            .unwrap();
            let context = crate::outbound::context::factory::local();
            let receipts = crate::outbound::receipts::fixtures::fake::ReceiptsFake {
                case_receipts: crate::outbound::receipts::types::CaseReceipts::Recorded(receipt.clone()),
            };
            record(
                &suite,
                &receipts,
                &context,
                &prepared,
                Contestant::Reference,
                None,
                None,
                YesNo::Yes,
                Some(&typed_proof),
                &CaptureRecord::Kept(capture),
            )
            .unwrap();
            let candidate_execution =
                Observer(crate::outbound::execution::fixtures::fake::ExecutionFake {
                    status: 0,
                    ..Default::default()
                });
            let candidate =
                run_case(&suite, &context, &linter, &candidate_execution, &prepared, &rules);
            assert!(matches!(candidate.capture, CaptureRecord::Absent));
            record(
                &suite,
                &receipts,
                &context,
                &prepared,
                Contestant::Candidate,
                Some(&candidate.result.unwrap().report),
                None,
                YesNo::Yes,
                Some(&typed_proof),
                &CaptureRecord::Absent,
            )
            .unwrap();
            let case = json!({
                "rule":"A001","name":name,"case_sha256":"case-pin","files":["case.py"],"expected":[],"comparison":"full","expectation_contract":"diagnostics","project_context_mode":"isolated","source_sha256":source_digest,
                "reference_discrepancy":{
                    "intended_expected":[],"reason":"independent toy language expectation","process":wire,"approved_executable_sha256":["a".repeat(64)],
                    "identity":{"tool":name,"version":"1","settings-sha256":identity.settings_sha256.0,"profile-sha256":identity.profile_sha256.0,"source-sha256":source_digest,"reference-output-sha256":identity.reference_output_sha256.0,"intended-output-sha256":identity.intended_output_sha256.0},
                }
            });
            let document = document_of(&case);
            Self {
                root,
                suite,
                prepared,
                rules,
                case,
                document,
                proof: typed_proof,
                receipt,
                receipts,
            }
        }
        pub fn load(&self) -> Loaded {
            self.read().expect("the fixture receipts decode")
        }
        /// The rows of another receipt file, with the raw bytes and invocation records they point at.
        pub fn load_file(&self, receipt: &std::path::Path) -> Loaded {
            self.read_at(receipt).expect("the receipt decodes")
        }
        fn read(&self) -> Option<Loaded> {
            self.read_at(&self.receipt)
        }
        fn read_at(&self, receipt: &std::path::Path) -> Option<Loaded> {
            let records = self.receipts.case_observations(ReceiptPath(receipt)).ok()?;
            let raw = crate::outbound::inventory::factory::local()
                .raw_artifacts(receipt, &records)
                .ok()?;
            Some(Loaded {
                records,
                artifacts: raw.bytes,
                invocations: raw.invocations,
            })
        }
        pub fn registration(&self) -> Registration {
            registration_of(&self.case)
        }
        /// The process failure the registered case declares.
        pub fn declared_failure(&self) -> ProcessFailure {
            process_failure(&self.case["reference_discrepancy"]["process"])
        }
        /// What the offline validator answers over `loaded`, against the registered case.
        pub fn replay(&self, loaded: &Loaded) -> Verdict {
            verdict(self.validate(&self.document, loaded))
        }
        /// The refusal of the offline validator over `loaded`.
        pub fn refusal(&self, loaded: &Loaded) -> String {
            self.validate(&self.document, loaded).unwrap_err().to_string()
        }
        fn validate(
            &self,
            document: &CaseDocument,
            loaded: &Loaded,
        ) -> Result<
            crate::domain::inventory_receipts::types::ReceiptCounts,
            crate::domain::inventory_receipts::errors::InventoryReceiptsError,
        > {
            crate::domain::inventory_receipts::service::validate_with_artifacts(
                std::slice::from_ref(document),
                &loaded.records,
                &loaded.artifacts,
                &loaded.invocations,
            )
        }
        /// What the validator answers after row `row` reports one finding of `rule` over `start..end` of `file`.
        pub fn replay_finding(&self, row: usize, rule: &str, file: &str, start: u64, end: u64) -> Verdict {
            let finding = json!([{"rule": rule, "file": file, "start": start, "end": end}]);
            self.replay_tampered(row, "/diagnostics", &finding.to_string())
        }
        /// What the validator answers when the registered case spells another expectation contract.
        pub fn replay_contract(&self, contract: &str) -> Verdict {
            let mut case = self.case.clone();
            case["expectation_contract"] = json!(contract);
            verdict(self.validate(&document_of(&case), &self.load()))
        }
        /// What the validator answers after `value` (JSON text) replaces `pointer` on one row, or a refusal when the schema refuses the row.
        pub fn replay_tampered(&self, row: usize, pointer: &str, value: &str) -> Verdict {
            let original = std::fs::read_to_string(&self.receipt).unwrap();
            let mut lines: Vec<Value> = original
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            *lines[row].pointer_mut(pointer).unwrap() = serde_json::from_str(value).unwrap();
            let edited = lines
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(&self.receipt, edited + "\n").unwrap();
            let loaded = self.read();
            std::fs::write(&self.receipt, original).unwrap();
            loaded.map_or(Verdict::Refused, |loaded| self.replay(&loaded))
        }
        /// The rows with the first row's stdout bytes replaced by forged ones.
        pub fn forged_stdout(&self) -> Loaded {
            let mut loaded = self.load();
            let path = {
                let ReferenceProcess::Observed(observed) = &loaded.records[0].process_observation else {
                    panic!("the reference process is observed");
                };
                let ProcessArtifactsRecord::Attached(artifacts) = &observed.artifacts else {
                    panic!("the observed process points at artifacts");
                };
                PathBuf::from(&artifacts.stdout.path.0)
            };
            loaded.artifacts.insert(path, CapturedBytes(b"forged".to_vec()));
            loaded
        }
        /// The rows whose first invocation record spells `flag` as its only argument, under a digest that follows the edit.
        pub fn rearmed_invocation(&self, flag: &str) -> Loaded {
            let mut loaded = self.load();
            let ReferenceProcess::Observed(observed) = &mut loaded.records[0].process_observation else {
                panic!("the reference process is observed");
            };
            let ProcessArtifactsRecord::Attached(artifacts) = &mut observed.artifacts else {
                panic!("the observed process points at artifacts");
            };
            let pointer = &mut artifacts.invocation;
            let path = PathBuf::from(&pointer.path.0);
            let mut invocation: Value = serde_json::from_slice(&loaded.artifacts[&path].0).unwrap();
            invocation["arguments"] = json!([flag]);
            let bytes = serde_json::to_vec(&invocation).unwrap();
            pointer.sha256 = Sha256Text(pin(&bytes));
            let captured = CapturedBytes(bytes);
            loaded.invocations.insert(
                path.clone(),
                crate::outbound::inventory::factory::local()
                    .decode_invocation(&captured)
                    .unwrap(),
            );
            loaded.artifacts.insert(path, captured);
            loaded
        }
    }

    /// The prepared discrepancy of a case the fixture made differ.
    fn differing(prepared: &PreparedCase) -> &PreparedReferenceDiscrepancy {
        match &prepared.reference_discrepancy {
            ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Differs(discrepancy) => discrepancy,
            ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Agrees => panic!("fixture discrepancy"),
        }
    }
}
