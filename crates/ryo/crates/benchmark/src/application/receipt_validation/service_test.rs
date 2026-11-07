//! The offline entry recomputes the metadata witness and never echoes a stale declared digest.
use super::service::validate;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::inventory_receipts::fixtures::with_file_metadata;
use crate::outbound::inventory::types::{
    ProviderName, RawArtifacts, RegisteredRule, Registration, RegistrationAuthority, RuleCommand,
    RuleDefinition, RuleInstance, RuleReason,
};
use ryo_benchmark_contracts::domain::documents::types::case_document::value::{CaseDocument, FileMetadata};
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    FileMetadataDigest, PathText, RuleName,
};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::borrow::Cow;

fn registration(case: CaseDocument) -> Registration {
    Registration {
        authority: RegistrationAuthority::ReceiptOnly,
        provider: ProviderName("fixture".into()),
        defaults: vec![RuleName("A001".into())],
        rules: vec![RegisteredRule {
            definition: RuleDefinition::Unpinned,
            modules: Default::default(),
            artifacts: Default::default(),
            rule_id: RuleName("A001".into()),
            command: RuleCommand::Unregistered,
            reason: RuleReason::Unstated,
            cases: vec![case],
            instance: RuleInstance::Undeclared,
        }],
        identity: Default::default(),
    }
}

/// Which declaration the offline entry is given.
#[derive(Clone, Copy)]
enum Scenario {
    /// The digest the case declared for its file metadata.
    Declared,
    /// A declaration whose mode no longer matches the row's recorded digest.
    StaleDeclaration,
}

/// What the offline entry made of the declaration.
#[derive(Debug, PartialEq, Eq)]
enum Audit {
    /// One recomputed witness, equal to the declared digest, and no errors.
    Recomputed,
    /// Errors reported, and neither a verified pair nor a metadata witness retained.
    Rejected,
    /// Anything else.
    Unexpected,
}

type Input = Scenario;
type Value = Audit;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the offline entry emits a witness recomputed to the declared digest"),
        word: Word::Succeeds,
        input: || Scenario::Declared,
        expect: Expect::Value(Audit::Recomputed),
    },
    Case {
        name: CaseName("a stale declared digest is never echoed and the pair is not verified"),
        word: Word::Succeeds,
        input: || Scenario::StaleDeclaration,
        expect: Expect::Value(Audit::Rejected),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let (cases, records) = with_file_metadata(false);
    let case = cases[0].clone();
    let raw = RawArtifacts {
        bytes: Default::default(),
        invocations: Default::default(),
    };
    let audit_of = |case: CaseDocument| -> Result<_, ReportsError> {
        validate(&registration(case), &records, &raw)
    };
    let verdict = match scenario {
        Scenario::Declared => {
            let audit = audit_of(case.clone()).map_err(|error| error.to_string())?;
            let witness = audit
                .metadata_verified
                .first()
                .map(|entry| entry.digest.0.as_str());
            if audit.errors.is_empty()
                && audit.metadata_verified.len() == 1
                && match &case.file_metadata_sha256 {
                    FileMetadataDigest::Declared(declared) => witness == Some(declared.0.as_str()),
                    FileMetadataDigest::Undeclared => witness.is_none(),
                }
            {
                Audit::Recomputed
            } else {
                Audit::Unexpected
            }
        }
        Scenario::StaleDeclaration => {
            let mut changed = case;
            // Keep the old declaration/row digest to test recomputation.
            changed.file_metadata = FileMetadata::Declared(
                [(PathText("input.txt".into()), FileExecutable::Set)]
                    .into_iter()
                    .collect(),
            );
            let audit = audit_of(changed).map_err(|error| error.to_string())?;
            if !audit.errors.is_empty()
                && audit.verified.is_empty()
                && audit.metadata_verified.is_empty()
            {
                Audit::Rejected
            } else {
                Audit::Unexpected
            }
        }
    };
    Ok(verdict)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
