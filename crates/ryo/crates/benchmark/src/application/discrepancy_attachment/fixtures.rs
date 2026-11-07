//! A prepared case with no expectation yet, and the authored reference wires that attach to it.
use crate::domain::discrepancy::fixtures::case;
use crate::outbound::suite::types::discrepancy::{
    CaseReferenceWire, DiscrepancyBasisWire, EvidenceWire, ReferenceDiscrepancyWire,
    ReferenceFindingWire, ReferenceIdentityWire,
};
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;

/// Whether the suite serves the case profile.
#[derive(Clone, Copy, Debug)]
pub(super) enum Profile {
    /// The profile is readable.
    Served,
    /// The profile is absent.
    Missing,
}

/// The wires, the case and the rules path one attachment is run over.
pub(super) struct Attachment {
    pub wires: Vec<CaseReferenceWire>,
    pub case: PreparedCase,
    pub rules: &'static str,
    pub profile: Profile,
}

fn repeated(letter: char) -> String {
    letter.to_string().repeat(64)
}

fn identity() -> ReferenceIdentityWire {
    ReferenceIdentityWire {
        file_metadata_sha256: None,
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
pub(super) fn language_evidence() -> EvidenceWire {
    EvidenceWire::LanguageSpec {
        path: "semantic-authority.txt".into(),
        sha256: repeated('f'),
        language_version: "toy-v1".into(),
        target_configuration: "python3.10".into(),
        version_rationale: "toy fixture contract".into(),
    }
}

/// A prior public tool contract pinned to a source revision.
pub(super) fn public_contract() -> EvidenceWire {
    EvidenceWire::PublicToolContract {
        path: "contract.txt".into(),
        sha256: repeated('f'),
        source_revision: "a".repeat(40),
        source_path: "docs/contract.md".into(),
        rationale: "the published contract selects this rule".into(),
    }
}

/// A prior worker run with a named control.
pub(super) fn prior_worker() -> EvidenceWire {
    EvidenceWire::PriorWorkerRun {
        path: "prior.txt".into(),
        sha256: repeated('f'),
        worker_sha256: repeated('1'),
        control_source_sha256: repeated('2'),
        control_query_sha256: repeated('3'),
        control_profile_sha256: repeated('4'),
        witness_case: "control".into(),
        rationale: "the control exercises the same selection boundary".into(),
    }
}

/// A language-defect reference expectation with one diagnostic.
pub(super) fn discrepancy() -> ReferenceDiscrepancyWire {
    ReferenceDiscrepancyWire {
        process: None,
        expected: vec![ReferenceFindingWire {
            rule: "F821".into(),
            file: "case.py".into(),
            start: 0,
            end: 1,
        }],
        identity: identity(),
        evidence: vec![language_evidence()],
        reason: "independent language contract".into(),
    }
}

/// A reference wire for the fixture case under the given basis.
pub(super) fn wire(
    basis: DiscrepancyBasisWire,
    discrepancy: ReferenceDiscrepancyWire,
) -> CaseReferenceWire {
    CaseReferenceWire {
        rule: "F821".into(),
        name: "independent-contract".into(),
        basis,
        discrepancy,
    }
}

/// The fixture case with no canonical expectation.
pub(super) fn bare() -> PreparedCase {
    let mut prepared = case();
    prepared.reference_discrepancy = None;
    prepared
}

/// One language-defect wire over the bare case.
pub(super) fn language() -> Attachment {
    Attachment {
        wires: vec![wire(DiscrepancyBasisWire::LanguageDefect, discrepancy())],
        case: bare(),
        rules: "rules/benchmark.toml",
        profile: Profile::Served,
    }
}

/// The attachment with its first wire's discrepancy replaced.
pub(super) fn with(
    basis: DiscrepancyBasisWire,
    edit: impl FnOnce(&mut ReferenceDiscrepancyWire),
) -> Attachment {
    let mut discrepancy = discrepancy();
    edit(&mut discrepancy);
    Attachment {
        wires: vec![wire(basis, discrepancy)],
        ..language()
    }
}
