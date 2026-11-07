//! What every `validate_store` adapter must pass: the versions kept are loaded back whole, and keeping again replaces
//! them.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::validate_store::interface::ValidateStorePort;

/// How many versions a row keeps last: both, or only the first.
type Input = usize;
/// Whether the versions kept last are loaded back whole.
type Value = bool;
/// The text of the port's failure.
type Failure = String;

/// The contract's rows: every implementation runs them, in order.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_versions_kept_are_loaded_whole"),
        word: Word::Succeeds,
        input: || 2,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("keeping_again_replaces_the_versions"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value(true),
    },
];

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl ValidateStorePort) {
    use crate::domain::digests::types::Digest;
    use crate::domain::files::types::Stamp;
    use crate::domain::packages::types::{DefinitionKind, DefinitionName, Ownership};
    use crate::domain::packages::types::{PackageName, Release, VersionText};
    use crate::domain::proofs::types::ProvenVersion;
    let release = |package: &str, kind, name: &str| Release {
        package: PackageName(package.into()),
        kind,
        name: DefinitionName(name.into()),
        version: VersionText("0.1.0".into()),
        ownership: Ownership::Shared,
    };
    // A rule that uses nothing, and a spec that uses the rule and, through a package only a scenario names, a template.
    let rows = [
        ProvenVersion {
            release: release("p", DefinitionKind::Rule, "a"),
            stamp: Stamp {
                size: crate::domain::files::types::FileSize(10),
                modified: crate::domain::files::types::ModifiedTime(1),
            },
            digest: Digest("va".into()),
            uses: Vec::new(),
        },
        ProvenVersion {
            release: release("p", DefinitionKind::Spec, "s"),
            stamp: Stamp {
                size: crate::domain::files::types::FileSize(20),
                modified: crate::domain::files::types::ModifiedTime(2),
            },
            digest: Digest("vs".into()),
            uses: vec![
                release("p", DefinitionKind::Rule, "a"),
                release("q@/elsewhere/q", DefinitionKind::Template, "t"),
            ],
        },
    ];
    run(CASES, Word::Succeeds, |count| -> Result<Value, Failure> {
        let kept = rows.get(..count).unwrap_or_default();
        adapter
            .keep_definitions(kept)
            .map_err(|error| error.to_string())?;
        Ok(adapter.definitions().0 == kept)
    });
}
