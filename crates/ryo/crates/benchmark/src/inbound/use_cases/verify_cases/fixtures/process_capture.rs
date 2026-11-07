//! Small real-file fixtures; only child execution is substituted, not artifact or comparison policy.
use crate::application::case_execution::service::run as run_case;
use crate::application::case_observation::service::write as record;
use crate::domain::discrepancy::types::Contestant;
use crate::domain::discrepancy::types::OptionalDigest;
use crate::domain::discrepancy::types::ReferenceKind;
use crate::domain::discrepancy::types::ReferenceProof;
use crate::domain::process_failure::types::CaptureRecord;
use crate::outbound::execution::fixtures::observers::Observer;
use crate::outbound::execution::fixtures::observers::Streams;
use crate::outbound::inventory::fixtures::registrations::complete_of;
use crate::outbound::inventory::fixtures::registrations::document_of;
use crate::outbound::inventory::fixtures::registrations::invocation_bytes;
use crate::outbound::inventory::fixtures::registrations::process_failure;
use crate::outbound::inventory::interface::InventoryPort as _;
use crate::outbound::receipts::fixtures::fake::ReceiptsFake;
use crate::outbound::receipts::interface::ReceiptsPort as _;
use crate::outbound::suite::fixtures::authored;
use crate::outbound::suite::fixtures::fake::SuiteFake;
use crate::outbound::suite::interface::SuitePort as _;
use crate::outbound::suite::types::ArtifactPath;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
use ryo_benchmark_contracts::domain::documents::types::text::value::Pin;
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use ryo_benchmark_contracts::domain::documents::types::text::value::YesNo;
use ryo_benchmark_contracts::domain::process::types::ProcessFailure;
use ryo_benchmark_contracts::domain::process::types::ReferenceExpectation;
use ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName;
use ryo_benchmark_contracts::domain::requests::types::ParserSelection;
use ryo_benchmark_contracts::domain::workloads::types::CaseComparison;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceDiscrepancy;
use ryo_benchmark_contracts::domain::workloads::types::ReferenceDiscrepancyIdentity;
use serde_json::Value;
use serde_json::json;
use sha2::Digest as _;
use std::path::PathBuf;

/// The context port the verifier is given.
pub(crate) fn context() -> impl crate::outbound::context::interface::ContextPort {
    crate::outbound::context::factory::local()
}

pub(crate) struct Fixture {
    pub root: tempfile::TempDir,
    pub suite: SuiteFake,
    pub prepared: PreparedCase,
    pub rules: PathBuf,
    pub(super) case: Value,
    pub document: CaseDocument,
    pub proof: ReferenceProof,
    pub(super) receipt: PathBuf,
    pub receipts: ReceiptsFake,
}
pub(super) fn pin(bytes: &[u8]) -> String {
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
}

/// The prepared discrepancy of a case the fixture made differ.
fn differing(prepared: &PreparedCase) -> &PreparedReferenceDiscrepancy {
    match &prepared.reference_discrepancy {
        ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Differs(discrepancy) => discrepancy,
        ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Agrees => panic!("fixture discrepancy"),
    }
}
