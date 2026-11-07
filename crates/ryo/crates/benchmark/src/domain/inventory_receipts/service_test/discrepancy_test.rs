//! A documented reference difference and a process failure are admitted only with bound evidence.
use ryo_benchmark_contracts::domain::{
    documents::types::{
        case_document::value::CaseReference,
        finding::value::ExpectedFindings,
        observation_document::value::{ReferenceProcess, ToolContractRecord, ValidationRecord},
        reference_discrepancy::value::DiscrepancyRecord,
        text::value::{FileMetadataDigest, Sha256Text, YesNo},
    },
    reports::types::ComparisonContract,
    workloads::types::DiscrepancyBasis,
};

use super::super::{
    errors::InventoryReceiptsError,
    fixtures::{self, Documents},
    types::ReceiptCounts,
};
use super::{counts, process_discrepancy, process_observation};
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::domain::process_receipts::errors::ProcessReceiptsError;
use ryo_benchmark_contracts::domain::documents::types::case_document::value::ExpectationContract;

type Input = Documents;
type Value = ReceiptCounts;
type Failure = InventoryReceiptsError;

fn subject(input: Input) -> Result<Value, Failure> {
    super::subject(input)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an independently bound reference difference is a documented discrepancy"),
        word: Word::Succeeds,
        input: fixtures::independently_bound_discrepancy,
        expect: Expect::Value(counts(0, 1, 0)),
    },
    Case {
        name: CaseName("a discrepancy with an unbound settings identity is refused"),
        word: Word::Refuses,
        input: fixtures::unbound_discrepancy,
        expect: Expect::Error(InventoryReceiptsError::StaleDiscrepancyEvidence),
    },
    Case {
        name: CaseName("a registered process failure without an observed process is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0].reference_discrepancy = CaseReference::Differs(DiscrepancyRecord {
                    ..process_discrepancy()
                });
                for row in &mut d.1 {
                    row.documented_reference_discrepancy = YesNo::Yes;
                    row.reference_expected = ExpectedFindings::Underivable;
                }
            })
        },
        expect: Expect::Error(InventoryReceiptsError::Process(
            ProcessReceiptsError::ReferenceVariantDiffers,
        )),
    },
    Case {
        name: CaseName("a process expectation under a complete-report contract is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0].expectation_contract =
                    ExpectationContract::Declared(ComparisonContract::CompleteReport);
                d.0[0].reference_discrepancy = CaseReference::Differs(process_discrepancy());
            })
        },
        expect: Expect::Error(InventoryReceiptsError::ProcessExpectationNotComplete),
    },
    Case {
        name: CaseName("a documented discrepancy the registration never declared is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0].documented_reference_discrepancy = YesNo::Yes
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyAuthorityDiffers),
    },
    Case {
        name: CaseName("a process observation inside a diagnostic report is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0].process_observation = ReferenceProcess::Observed(process_observation())
            })
        },
        expect: Expect::Error(InventoryReceiptsError::UnexpectedProcessObservation),
    },
    Case {
        name: CaseName("a discrepancy bound to other file metadata is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                if let CaseReference::Differs(certificate) = &mut d.0[0].reference_discrepancy {
                    certificate.identity.file_metadata =
                        FileMetadataDigest::Declared(Sha256Text("other".into()));
                }
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyMetadataDiffers),
    },
    Case {
        name: CaseName("a discrepancy validation that the sides do not share is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                d.1[1].discrepancy_validation = ValidationRecord::Unattested
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyValidationMissing),
    },
    Case {
        name: CaseName("a discrepancy basis the sides did not verify is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                if let CaseReference::Differs(certificate) = &mut d.0[0].reference_discrepancy {
                    certificate.basis = DiscrepancyBasis::ToolContract;
                }
                for row in &mut d.1 {
                    row.documented_reference_discrepancy = YesNo::No;
                    row.documented_tool_contract_difference =
                        ToolContractRecord::Recorded(YesNo::Yes);
                }
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyAuthorityUnverified),
    },
    Case {
        name: CaseName("a discrepancy without its tool identity is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                for row in &mut d.1 {
                    if let ValidationRecord::Attested(proof) = &mut row.discrepancy_validation {
                        proof.actual_executable_sha256 = Sha256Text(String::new());
                    }
                }
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyToolMissing),
    },
    Case {
        name: CaseName("a discrepancy tool that is not independently pinned is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                if let CaseReference::Differs(certificate) = &mut d.0[0].reference_discrepancy {
                    certificate.approved_executable_sha256 = vec![Sha256Text("another-pin".into())];
                }
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyToolUnpinned),
    },
];

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
