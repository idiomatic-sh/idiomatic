//! A prepared case with no expectation yet, and the authored reference claims that attach to it.
use crate::domain::discrepancy::fixtures::case;
use crate::outbound::suite::types::{
    CaseClaim, ClaimedDiscrepancy, ClaimedEvidence, ClaimedFinding, ClaimedIdentity,
    ClaimedMetadata, ClaimedProcess,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{CaseName, PathText, RuleName, Sha256Text};
use ryo_benchmark_contracts::domain::reports::types::SourceByteOffset;
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::{
    ContractSourcePath, LanguageVersion, ParserTarget, Rationale, SourceRevision, WitnessCase,
};
use ryo_benchmark_contracts::domain::workloads::types::{
    DiscrepancyBasis, DiscrepancyReason, PreparedCase,
};

/// Whether the suite serves the case profile.
#[derive(Clone, Copy, Debug)]
pub(super) enum Profile {
    /// The profile is readable.
    Served,
    /// The profile is absent.
    Missing,
}

/// The claims, the case and the rules path one attachment is run over.
pub(super) struct Attachment {
    pub claims: Vec<CaseClaim>,
    pub case: PreparedCase,
    pub rules: &'static str,
    pub profile: Profile,
}

fn repeated(letter: char) -> Sha256Text {
    Sha256Text(letter.to_string().repeat(64))
}

fn identity() -> ClaimedIdentity {
    ClaimedIdentity {
        file_metadata_sha256: ClaimedMetadata::Undeclared,
        tool: "fixture".into(),
        version: "1.0".into(),
        observed_executable_sha256: repeated('a'),
        profile_sha256: repeated('c'),
        settings_sha256: repeated('d'),
        source_sha256: repeated('e'),
        reference_output_sha256: repeated('f'),
        intended_output_sha256: repeated('0'),
    }
}

/// Independent language meaning for the configured parser target.
pub(super) fn language_evidence() -> ClaimedEvidence {
    ClaimedEvidence::LanguageSpec {
        path: "semantic-authority.txt".into(),
        sha256: repeated('f'),
        language_version: LanguageVersion("toy-v1".into()),
        target_configuration: ParserTarget("python3.10".into()),
        version_rationale: Rationale("toy fixture contract".into()),
    }
}

/// A prior public tool contract pinned to a source revision.
pub(super) fn public_contract() -> ClaimedEvidence {
    ClaimedEvidence::PublicToolContract {
        path: "contract.txt".into(),
        sha256: repeated('f'),
        source_revision: SourceRevision("a".repeat(40)),
        source_path: ContractSourcePath("docs/contract.md".into()),
        rationale: Rationale("the published contract selects this rule".into()),
    }
}

/// A prior worker run with a named control.
pub(super) fn prior_worker() -> ClaimedEvidence {
    ClaimedEvidence::PriorWorkerRun {
        path: "prior.txt".into(),
        sha256: repeated('f'),
        worker_sha256: repeated('1'),
        control_source_sha256: repeated('2'),
        control_query_sha256: repeated('3'),
        control_profile_sha256: repeated('4'),
        witness_case: WitnessCase("control".into()),
        rationale: Rationale("the control exercises the same selection boundary".into()),
    }
}

/// A language-defect reference expectation with one diagnostic.
pub(super) fn discrepancy() -> ClaimedDiscrepancy {
    ClaimedDiscrepancy {
        process: ClaimedProcess::Absent,
        expected: vec![ClaimedFinding {
            rule: RuleName("F821".into()),
            file: PathText("case.py".into()),
            start: SourceByteOffset(0),
            end: SourceByteOffset(1),
        }],
        identity: identity(),
        evidence: vec![language_evidence()],
        reason: DiscrepancyReason("independent language contract".into()),
    }
}

/// A reference claim for the fixture case under the given basis.
pub(super) fn wire(basis: DiscrepancyBasis, discrepancy: ClaimedDiscrepancy) -> CaseClaim {
    CaseClaim {
        rule: RuleName("F821".into()),
        name: CaseName("independent-contract".into()),
        basis,
        discrepancy,
    }
}

/// The fixture case with no canonical expectation.
pub(super) fn bare() -> PreparedCase {
    let mut prepared = case();
    prepared.reference_discrepancy = ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Agrees;
    prepared
}

/// One language-defect claim over the bare case.
pub(super) fn language() -> Attachment {
    Attachment {
        claims: vec![wire(DiscrepancyBasis::LanguageDefect, discrepancy())],
        case: bare(),
        rules: "rules/benchmark.toml",
        profile: Profile::Served,
    }
}

/// The attachment with its first claim's discrepancy replaced.
pub(super) fn with(
    basis: DiscrepancyBasis,
    edit: impl FnOnce(&mut ClaimedDiscrepancy),
) -> Attachment {
    let mut discrepancy = discrepancy();
    edit(&mut discrepancy);
    Attachment {
        claims: vec![wire(basis, discrepancy)],
        ..language()
    }
}
