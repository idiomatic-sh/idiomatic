// ignore: idiomatic.arch/file-length - the test data of the paired verifier, its process-failure capture and its documented reference difference: they share one scenario over the fakes of its ports, and the use-case spec keeps a non-contract use case's fixtures in this one file (a `fixtures/` directory is refused by contract-only-files), so it cannot be split one fixture per file.
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
    use crate::outbound::{execution::fixtures::fake::ExecutionFake, suite::fixtures::fake::SuiteFake};
    use crate::outbound::linter::fixtures::fake::LinterFake;
    use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
    use ryo_benchmark_contracts::domain::reports::types::{
        BenchmarkFinding, CoverageAccounting, ExecutionMetadata, PairCount, ProjectContext,
        RuleCoverage, RunReport, SourceByteOffset,
    };
    use ryo_benchmark_contracts::domain::requests::types::{
        CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration, ParserSelection,
        ProviderSelection, RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
    };
    use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;
    use ryo_benchmark_contracts::domain::workloads::types::{
        CaseComparison, FileObservation, PreparedCase,
        PreparedReferenceBehavior, RuleContextProfile, Tool,
    };

    use crate::inbound::use_cases::verify_cases::service::verify_prepared_shard;
    use crate::outbound::execution::interface::ExecutionPort;
    use crate::outbound::linter::interface::LinterPort;
    use crate::outbound::receipts::fixtures::fake::ReceiptsFake;
    use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
    use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
    use std::path::Path;

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
        match &receipts.case_receipts {
            crate::outbound::receipts::types::CaseReceipts::Recorded(path) => {
                std::fs::remove_file(path).unwrap();
            }
            crate::outbound::receipts::types::CaseReceipts::Unrecorded => {
                unreachable!("fixture records receipts")
            }
        }
    }

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
        if message.contains("expected-vs-candidate") {
            Refusal::CandidateMismatch
        } else if message.contains("expected-vs-reference") {
            Refusal::ReferenceMismatch
        } else if message == "operational failure" {
            Refusal::CandidateStartup
        } else if message.contains("reference: operational failure") {
            Refusal::ReferenceStartup
        } else if message.contains("defaults lack implementation or explicit deferral") {
            Refusal::UnaccountedDefault
        } else if message == crate::domain::suite::constants::NO_CASES {
            Refusal::NoCases
        } else if message == crate::domain::diagnostics::constants::INPUT_COVERAGE {
            Refusal::InputCoverage
        } else if message.contains("fixture has no independent discrepancy artifacts") {
            Refusal::UnprovenEvidence
        } else if message.contains("executable metadata drifted") {
            Refusal::MetadataDrift
        } else if message.contains("require verification preparation") {
            Refusal::VerificationRequired
        } else {
            Refusal::Unexpected
        }
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

    /// The complete one-file report with `diagnostics`.
    fn report(diagnostics: Vec<BenchmarkFinding>) -> RunReport {
        RunReport {
            execution_metadata: ExecutionMetadata::Complete,
            project_context: ProjectContext::NotRequested,
            files: vec![ryo::linter::RelativePath("case.py".into())],
            skipped: Vec::new(),
            coverage: CoverageAccounting::Accounted(RuleCoverage {
                selected: PairCount(1),
                evaluated: PairCount(1),
                skipped: PairCount(0),
                unknown_or_unread: PairCount(0),
            }),
            diagnostics,
        }
    }

    /// A finding of `rule` over `start..end` bytes of the case file.
    pub(crate) fn finding(rule: &str, start: usize, end: usize) -> BenchmarkFinding {
        BenchmarkFinding {
            rule: ryo::linter::RuleId::try_from(rule).expect("fixture rule identity"),
            file: ryo::linter::RelativePath("case.py".into()),
            start: SourceByteOffset(start),
            end: SourceByteOffset(end),
        }
    }

    /// A request over the one case file.
    pub(crate) fn request() -> RunRequest {
        RunRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Frozen,
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Unconfigured,
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            staged_sources: StagedSources::Unlisted,
            file_metadata: FileMetadataDeclaration::Undeclared,
            context_manifest: ContextSeal::Unsealed,
            root: ".".into(),
            files: vec![ryo::linter::RelativePath("case.py".into())],
            state: "cache".into(),
        }
    }

    /// Construct authored evidence independently from either contestant response.
    pub(crate) fn prepared(shape: Shape) -> (SuiteFake, BenchmarkFinding) {
        let expected = finding("E722", 0, 6);
        let mut suite = SuiteFake::default();
        if !matches!(shape, Shape::NoCases) {
            suite.prepared.push(PreparedCase {
                reference_discrepancy: PreparedReferenceBehavior::Agrees,
                comparison: CaseComparison::Rule,
                rule: ryo::linter::RuleId::try_from("E722").unwrap(),
                name: "positive".into(),
                request: request(),
                expected: report(vec![expected.clone()]),
            });
        }
        suite.profile = Some(Tool {
            rule_observation_selection: ObservationSelectionPolicy::Declared,
            supporting_rules: Vec::new(),
            rule_observations: Default::default(),
            rule_context: RuleContextProfile::Undeclared,
            file_observation: FileObservation::Undeclared,
            project_context_mode: ProjectContextMode::default(),
            tool: "fixture".into(),
            version: "1".into(),
            sha256: vec!["pin".into()],
            defaults: vec!["E722".try_into().unwrap(), "F901".try_into().unwrap()],
            deferred: std::collections::BTreeMap::default(),
            pending: std::collections::BTreeMap::default(),
            query_modules: std::collections::BTreeMap::default(),
            rule_applicability: std::collections::BTreeMap::default(),
            parser_configuration: ParserSelection::Unspecified,
            source_language: ProviderSelection::Unspecified,
            project_search_roots: SearchRoots::Undeclared,
        });
        (suite, expected)
    }

    /// Give both contestants parser diagnostics, optionally retaining an unrelated lint as a control.
    pub(crate) fn report_scope(
        shape: Shape,
        suite: &mut SuiteFake,
        reference: &mut LinterFake,
        candidate: &mut LinterFake,
    ) {
        if matches!(
            shape,
            Shape::FullAgreement | Shape::MissingFrontend | Shape::RuleProjection
        ) {
            let frontend = finding("invalid-syntax", 9, 19);
            reference.diagnostics.push(frontend.clone());
            if !matches!(shape, Shape::MissingFrontend) {
                candidate.diagnostics.push(frontend.clone());
            }
            if matches!(shape, Shape::RuleProjection) {
                let unrelated = finding("F401", 20, 24);
                reference.diagnostics.push(unrelated.clone());
                candidate.diagnostics.push(unrelated);
            }
            let authored = suite.prepared.first_mut().expect("authored case");
            if !matches!(shape, Shape::RuleProjection) {
                authored.comparison = CaseComparison::Full;
            }
            authored.expected.diagnostics.push(frontend);
        }
    }
}

/// A documented reference difference keeps the intended contract and the pinned reference apart.
#[cfg(test)]
pub(crate) mod discrepancy {
    use super::shard::{Shape, prepared};
    use crate::outbound::{
        linter::{fixtures::fake::LinterFake, interface::LinterPort},
        suite::fixtures::fake::SuiteFake,
    };
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
    use ryo_benchmark_contracts::domain::measurements::types::{Invocation, ProcessOutput};
    use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
    use ryo_benchmark_contracts::domain::reports::types::{CoverageAccounting, PairCount, RuleCoverage, RunReport};
    use ryo_benchmark_contracts::domain::requests::types::{
        ParserConfigurationName, ParserSelection, ProviderSelection, RunRequest, SearchRoots,
    };
    use crate::outbound::linter::types::CanonicalText;
    use ryo_benchmark_contracts::domain::context::types::{ObservationSelectionPolicy, PreparedRuleContext};
    use ryo_benchmark_contracts::domain::workloads::types::{
        CaseComparison, DiscrepancyBasis, PreparedCase, DiscrepancyReason, FileObservation,
        MetadataDigestDeclaration, PreparedReferenceBehavior,
        PreparedReferenceDiscrepancy, ReferenceDiscrepancyIdentity, RuleContextProfile, Tool,
        discrepancy_evidence::value::{
            DiscrepancyEvidence, DiscrepancyEvidenceKind, LanguageVersion, ParserTarget, Rationale,
        },
    };

    /// Candidate fixture exposes complete execution accounting independently of its findings.
    pub(crate) struct CoveredCandidate(pub LinterFake);
    impl LinterPort for CoveredCandidate {
        fn rule_configuration(
            &self,
        ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
            ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Unsupported
        }
        fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
            LinterPort::prepare(&self.0, request)
        }
        fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
            let mut report = LinterPort::decode(&self.0, request, output)?;
            report.coverage = CoverageAccounting::Accounted(RuleCoverage {
                selected: PairCount(1),
                evaluated: PairCount(1),
                skipped: PairCount(0),
                unknown_or_unread: PairCount(0),
            });
            Ok(report)
        }
        fn settings_text(
            &self,
            normalized: &Invocation,
            case: &PreparedCase,
            profile: &Tool,
        ) -> Result<CanonicalText, Error> {
            LinterPort::settings_text(&self.0, normalized, case, profile)
        }
        fn rule_context_text(&self, prepared: &PreparedRuleContext) -> CanonicalText {
            LinterPort::rule_context_text(&self.0, prepared)
        }
    }

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
        let profile = Tool {
            rule_observation_selection: ObservationSelectionPolicy::Declared,
            supporting_rules: Vec::new(),
            rule_observations: Default::default(),
            rule_context: RuleContextProfile::Undeclared,
            file_observation: FileObservation::Undeclared,
            project_context_mode: ProjectContextMode::Isolated,
            tool: "fixture".into(),
            version: "1".into(),
            sha256: vec![binary_hash.0.clone().into()],
            defaults: vec![case.rule.clone()],
            deferred: std::collections::BTreeMap::default(),
            pending: std::collections::BTreeMap::default(),
            query_modules: std::collections::BTreeMap::default(),
            rule_applicability: std::collections::BTreeMap::default(),
            parser_configuration: ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName(
                "python-target=3.10".into(),
            )),
            source_language: ProviderSelection::Unspecified,
            project_search_roots: SearchRoots::Undeclared,
        };
        let invocation = LinterFake::default()
            .prepare(&case.request)
            .expect("fixture invocation");
        let settings = crate::domain::canonical_text::service::text_digest(
            &crate::application::discrepancy_validation::service::settings_text(
                &crate::outbound::linter::fixtures::fake::LinterFake::default(),
                &invocation,
                case,
                &profile,
            )
            .expect("fixture settings"),
        );
        case.reference_discrepancy = PreparedReferenceBehavior::Differs(PreparedReferenceDiscrepancy {
            basis: DiscrepancyBasis::LanguageDefect,
            identity: ReferenceDiscrepancyIdentity {
                file_metadata_sha256: MetadataDigestDeclaration::Undeclared,
                tool: "fixture".into(),
                version: "1".into(),
                observed_executable_sha256: binary_hash.clone(),
                profile_sha256: profile_hash.clone(),
                settings_sha256: settings,
                source_sha256: ArtifactDigest("c".repeat(64)),
                intended_output_sha256: crate::domain::discrepancy::service::output_digest(
                    &case.expected,
                ),
                reference_output_sha256: crate::domain::discrepancy::service::output_digest(&reference),
            },
            expected:
                ryo_benchmark_contracts::domain::process::types::ReferenceExpectation::Diagnostics(
                    reference,
                ),
            reason: DiscrepancyReason(
                "independent language contract differs from the pinned reference".into(),
            ),
            evidence: vec![DiscrepancyEvidence::Language {
                path: "evidence.json".into(),
                sha256: ArtifactDigest("d".repeat(64)),
                kind: DiscrepancyEvidenceKind::IndependentLanguageRun,
                language_version: LanguageVersion("3.10".into()),
                target_configuration: ParserTarget("python-target=3.10".into()),
                version_rationale: Rationale("exact target independently evaluated".into()),
            }],
        });
        suite.profile = Some(profile);
        let _previous_profile = suite.artifact_digests.insert(
            std::path::Path::new("/profile")
                .join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST),
            profile_hash,
        );
        let _previous_binary = suite.artifact_digests.insert("fixture".into(), binary_hash);
        suite.discrepancy_artifacts_valid = true;
        let authored_case = suite.prepared.first_mut().expect("authored case");
        let PreparedReferenceBehavior::Differs(item) = std::mem::replace(
            &mut authored_case.reference_discrepancy,
            PreparedReferenceBehavior::Agrees,
        ) else {
            unreachable!("fixture canonical certificate")
        };
        let identity = item.identity;
        let discrepancy = {
            use crate::outbound::suite::types::{
                ClaimedDiscrepancy, ClaimedEvidence, ClaimedIdentity, ClaimedMetadata, ClaimedProcess,
            };
            use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
            use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::{
                LanguageVersion, ParserTarget, Rationale,
            };
            ClaimedDiscrepancy {
                process: ClaimedProcess::Absent,
                expected: Vec::new(),
                identity: ClaimedIdentity {
                    file_metadata_sha256: ClaimedMetadata::Undeclared,
                    tool: identity.tool,
                    version: identity.version,
                    observed_executable_sha256: Sha256Text(identity.observed_executable_sha256.0),
                    profile_sha256: Sha256Text(identity.profile_sha256.0),
                    settings_sha256: Sha256Text(identity.settings_sha256.0),
                    source_sha256: Sha256Text(identity.source_sha256.0),
                    reference_output_sha256: Sha256Text(identity.reference_output_sha256.0),
                    intended_output_sha256: Sha256Text(identity.intended_output_sha256.0),
                },
                evidence: vec![ClaimedEvidence::IndependentLanguageRun {
                    path: "evidence.json".into(),
                    sha256: Sha256Text("d".repeat(64)),
                    language_version: LanguageVersion("3.10".into()),
                    target_configuration: ParserTarget("python-target=3.10".into()),
                    version_rationale: Rationale("exact target independently evaluated".into()),
                }],
                reason: item.reason,
            }
        };
        suite.reference_claims.push(
            crate::outbound::suite::types::CaseClaim {
                rule: ryo_benchmark_contracts::domain::documents::types::text::value::RuleName(authored_case.rule.to_string()),
                name: ryo_benchmark_contracts::domain::documents::types::text::value::CaseName(authored_case.name.to_string()),
                basis: ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis::LanguageDefect,
                discrepancy,
            },
        );
        suite
    }
}

/// Raw invocation records, registered documents and the byte streams a fake process observes.
#[cfg(test)]
pub(crate) mod process_identity {
    use crate::outbound::execution::interface::ExecutionPort;
    use crate::outbound::inventory::{interface::InventoryPort as _, types::Registration};
    use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
    use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{
        ArgumentText, CompleteInvocation, EnvironmentName, EnvironmentValue, InvocationDocument,
    };
    use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal, YesNo};
    use ryo_benchmark_contracts::domain::measurements::types::{
        CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
        ResourceObservation, TerminationSignal,
    };
    use crate::outbound::suite::{
        interface::SuitePort as _,
        types::{ArtifactMetadata, ArtifactPath, ArtifactSet, CaseArtifacts},
    };
    use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
    use ryo_benchmark_contracts::domain::requests::types::StagedSources;
    use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
    use serde_json::{Value, json};
    use std::collections::BTreeMap;

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

    fn pin_text(pin: &Pin) -> Value {
        match pin {
            Pin::Absent => Value::Null,
            Pin::Sha256(digest) => json!(digest.0),
        }
    }

    /// The raw invocation record file a capture keeps beside its process bytes.
    pub(crate) fn invocation_bytes(invocation: &InvocationDocument) -> Vec<u8> {
        let argument_texts = |values: &[ArgumentText]| {
            values.iter().map(|value| value.0.clone()).collect::<Vec<_>>()
        };
        let environment_texts = |values: &BTreeMap<EnvironmentName, EnvironmentValue>| {
            values
                .iter()
                .map(|(name, value)| (name.0.clone(), value.0.clone()))
                .collect::<BTreeMap<_, _>>()
        };
        let signal = |signal: &Signal| match signal {
            Signal::Absent => Value::Null,
            Signal::Number(number) => json!(number.0),
        };
        let record = match invocation {
            InvocationDocument::Complete(complete) => json!({
                "actual_request_root": complete.actual_request_root.0,
                "files": complete.files.iter().map(|file| file.0.clone()).collect::<Vec<_>>(),
                "executable_sha256": complete.executable_sha256.0,
                "settings_sha256": complete.settings_sha256.0,
                "normalized_settings": serde_json::from_str::<Value>(&complete.normalized_settings.canonical.0).unwrap(),
                "profile_sha256": complete.profile_sha256.0,
                "tool": complete.tool.0, "version": complete.version.0, "state": complete.state.0,
                "arguments": argument_texts(&complete.arguments),
                "environment": environment_texts(&complete.environment),
                "directory": complete.directory.0, "exit": complete.exit.0,
                "signal": signal(&complete.signal),
                "observer_no_deadline": complete.observer_no_deadline == YesNo::Yes,
                "source_sha256": pin_text(&complete.source_sha256),
                "file_metadata_sha256": pin_text(&complete.file_metadata_sha256),
            }),
            InvocationDocument::Failed(failed) => json!({
                "actual_request_root": failed.actual_request_root.0,
                "files": failed.files.iter().map(|file| file.0.clone()).collect::<Vec<_>>(),
                "arguments": argument_texts(&failed.arguments),
                "environment": environment_texts(&failed.environment),
                "directory": failed.directory.0, "exit": failed.exit.0,
                "signal": signal(&failed.signal),
                "observer_no_deadline": failed.observer_no_deadline == YesNo::Yes,
                "metadata_error": failed.metadata_error.0,
                "source_sha256": pin_text(&failed.source_sha256),
                "file_metadata_sha256": pin_text(&failed.file_metadata_sha256),
            }),
        };
        serde_json::to_vec(&record).unwrap()
    }

    /// The complete variant of a capture whose process identity was recorded.
    pub(crate) fn complete_of(invocation: &mut InvocationDocument) -> &mut CompleteInvocation {
        match invocation {
            InvocationDocument::Complete(complete) => complete,
            InvocationDocument::Failed(_) => unreachable!("fixture capture records its identity"),
        }
    }

    /// Read a registration the way the offline command does: through the adapter's schema.
    pub(crate) fn registration_of(case: &Value) -> Registration {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("registration.json");
        std::fs::write(
            &path,
            json!({"authority":{"kind":"receipt-only"},"provider":"toy","defaults":["A001"],"rules":[{"rule_id":"A001","definition":null,"modules":{},"artifacts":{},"command":null,"reason":null,"cases":[case],"instance":null}]}).to_string(),
        )
        .unwrap();
        crate::outbound::inventory::factory::local()
            .load(&path)
            .unwrap()
    }

    /// One registered case read through the adapter's schema.
    pub(crate) fn document_of(case: &Value) -> CaseDocument {
        registration_of(case).rules.remove(0).cases.remove(0)
    }

    /// Different observed byte streams, independent of any certificate matching implementation.
    pub(crate) struct Streams {
        pub stdout: Vec<u8>,
        pub stderr: Vec<u8>,
    }
    impl ExecutionPort for Streams {
        fn profile(&self) -> ExecutionProfile {
            ExecutionProfile::Wall
        }
        fn exit_observation(&self) -> crate::outbound::execution::types::ExitObservationMode {
            crate::outbound::execution::types::ExitObservationMode::Direct
        }
        fn run(
            &self,
            _: Invocation,
        ) -> Result<ProcessOutput, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
            Ok(ProcessOutput {
                status: ProcessExitCode(2),
                signal: TerminationSignal::Exited,
                stdout: CapturedBytes(self.stdout.clone()),
                stderr: CapturedBytes(self.stderr.clone()),
                wall: std::time::Duration::ZERO,
                resources: ResourceObservation::WallOnly,
            })
        }
    }

    /// Explicit no-deadline capability; the underlying fixture retains actual invocation history.
    pub(crate) struct Observer(pub crate::outbound::execution::fixtures::fake::ExecutionFake);
    impl ExecutionPort for Observer {
        fn profile(&self) -> ExecutionProfile {
            self.0.profile()
        }
        fn exit_observation(&self) -> crate::outbound::execution::types::ExitObservationMode {
            crate::outbound::execution::types::ExitObservationMode::Direct
        }
        fn run(
            &self,
            invocation: Invocation,
        ) -> Result<ProcessOutput, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
            self.0.run(invocation)
        }
    }

    /// Decode the process-failure certificate a test authored as JSON into its contract value.
    pub(crate) fn process_failure(
        value: &Value,
    ) -> ryo_benchmark_contracts::domain::process::types::ProcessFailure {
        use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
        use ryo_benchmark_contracts::domain::process::types::{
            OutputPart, OutputText, PathEncoding, ProcessArtifact, ProcessFailure, ProcessObservation,
        };
        let text = |value: &Value| value.as_str().expect("certificate text").to_owned();
        let parts = |value: &Value| -> Vec<OutputPart> {
            value
                .as_array()
                .expect("output parts")
                .iter()
                .map(|part| match part["kind"].as_str().expect("part kind") {
                    "literal" => OutputPart::Literal(OutputText(text(&part["value"]))),
                    _ => OutputPart::InputPath {
                        file: text(&part["file"]).into(),
                        encoding: match part["encoding"].as_str().expect("encoding") {
                            "utf8-bytes" => PathEncoding::Utf8Bytes,
                            _ => PathEncoding::JsonStringContent,
                        },
                    },
                })
                .collect()
        };
        let artifact = |value: &Value| ProcessArtifact {
            path: text(&value["path"]).into(),
            sha256: ArtifactDigest(text(&value["sha256"])),
        };
        ProcessFailure {
            exit: ProcessExitCode(i32::try_from(value["exit"].as_i64().expect("exit")).expect("exit")),
            stdout: parts(&value["stdout"]),
            stderr: parts(&value["stderr"]),
            observations: value["observations"]
                .as_array()
                .expect("observations")
                .iter()
                .map(|row| ProcessObservation {
                    executable_sha256: ArtifactDigest(text(&row["executable-sha256"])),
                    invocation: artifact(&row["invocation"]),
                    stdout: artifact(&row["stdout"]),
                    stderr: artifact(&row["stderr"]),
                })
                .collect(),
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
    use super::process_identity::{
        Observer, Streams, Verdict, complete_of, document_of, invocation_bytes, process_failure,
        registration_of, verdict,
    };
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
        receipts::{
            fixtures::fake::ReceiptsFake, interface::ReceiptsPort as _, types::ReceiptPath,
        },
        suite::{
            fixtures::fake::SuiteFake,
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
    use ryo_benchmark_contracts::domain::reports::types::{
        ExecutionMetadata, RuleCoverage, RunReport,
    };
    use ryo_benchmark_contracts::domain::requests::types::RunRequest;
    use ryo_benchmark_contracts::domain::workloads::types::{
        CaseComparison, PreparedCase, PreparedReferenceDiscrepancy, ReferenceDiscrepancyIdentity, Tool,
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
            let profile = Tool {
                rule_observation_selection:
                    ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
                supporting_rules: Vec::new(),
                rule_observations: Default::default(),
                rule_context: ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile::Undeclared,
                file_observation: ryo_benchmark_contracts::domain::workloads::types::FileObservation::Undeclared,
                project_context_mode: ProjectContextMode::Isolated,
                tool: name.into(),
                version: "1".into(),
                sha256: vec!["a".repeat(64).into()],
                defaults: vec![ryo::linter::RuleId::try_from("A001").unwrap()],
                deferred: BTreeMap::new(),
                pending: BTreeMap::new(),
                query_modules: BTreeMap::new(),
                rule_applicability: BTreeMap::new(),
                parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName("python3.10".into())),
                source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
                project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            };
            let expected = RunReport {
                execution_metadata: ExecutionMetadata::Complete,
                project_context: ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                diagnostics: vec![],
                skipped: vec![],
                coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(RuleCoverage {
                    selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                    evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                    skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                    unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                }),
                files: vec![ryo::linter::RelativePath("case.py".into())],
            };
            let mut prepared = PreparedCase {
                comparison: CaseComparison::Full,
                rule: ryo::linter::RuleId::try_from("A001").unwrap(),
                name: name.into(),
                request: RunRequest {
                    performance: Default::default(),
                    selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
                    rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
                    rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
                    project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
                    given_project_inventory: Default::default(),
                    staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
                    file_metadata: ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
                    root: source.clone(),
                    state: source.join("state"),
                    files: expected.files.clone(),
                    context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
                },
                expected,
                reference_discrepancy: ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Agrees,
            };
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
