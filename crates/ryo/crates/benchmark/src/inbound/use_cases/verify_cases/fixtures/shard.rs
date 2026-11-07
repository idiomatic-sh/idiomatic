//! Independent authored evidence, constructed apart from either contestant response.
use crate::inbound::use_cases::verify_cases::service::verify_prepared_shard;
use crate::outbound::execution::fixtures::fake::ExecutionFake;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::fixtures::fake::LinterFake;
use crate::outbound::linter::interface::LinterPort;
use crate::outbound::receipts::fixtures::fake::ReceiptsFake;
use crate::outbound::receipts::types::CaseReceipts;
use crate::outbound::suite::fixtures::authored;
use crate::outbound::suite::fixtures::fake::SuiteFake;
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
