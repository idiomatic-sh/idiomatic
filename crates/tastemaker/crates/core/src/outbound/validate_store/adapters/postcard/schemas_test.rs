//! What the postcard schemas read back: the versions they wrote, every text kept once.

use super::StoreSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row writes.
#[derive(Clone, Copy, Debug)]
enum Written {
    /// Two versions of one package, the second using the first.
    TwoVersions,
}

type Input = Written;
/// The texts the store keeps, and whether its rows read back whole.
type Value = (usize, bool);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("versions_written_are_read_back_each_text_once"),
    word: Word::Succeeds,
    input: || Written::TwoVersions,
    expect: Expect::Value((6, true)),
}];

fn subject(written: Input) -> Value {
    use crate::domain::digests::types::Digest;
    use crate::domain::files::types::Stamp;
    use crate::domain::packages::types::{DefinitionKind, DefinitionName, Ownership};
    use crate::domain::packages::types::{PackageName, Release, VersionText};
    use crate::domain::proofs::types::ProvenVersion;
    let Written::TwoVersions = written;
    let release = |kind, name: &str| Release {
        package: PackageName("p".into()),
        kind,
        name: DefinitionName(name.into()),
        version: VersionText("0.1.0".into()),
        ownership: Ownership::Shared,
    };
    let rows = vec![
        ProvenVersion {
            release: release(DefinitionKind::Rule, "a"),
            stamp: Stamp::default(),
            digest: Digest("da".into()),
            uses: Vec::new(),
        },
        ProvenVersion {
            release: release(DefinitionKind::Spec, "s"),
            stamp: Stamp::default(),
            digest: Digest("ds".into()),
            uses: vec![release(DefinitionKind::Rule, "a")],
        },
    ];
    let kept = StoreSchema::with(&crate::domain::settings::types::Build("b".into()), &rows);
    (kept.strings.len(), kept.definitions() == Some(rows))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
