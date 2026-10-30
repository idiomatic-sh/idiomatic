//! The `language` port's fake runs the port's contract, and fails as it is told to.

use super::super::contract::check;
use super::{FakeCatalog, LanguageFake};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::language::errors::LanguageError;
use crate::outbound::language::interface::LanguagePort as _;
use crate::outbound::language::types::{ProjectInput, ProjectNotes};
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::parse::fixtures::contract::Discard;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_language::inbound::use_cases::project::types::{ManifestFile, ManifestText};

/// What a row of the cases asks of the fake.
#[derive(Clone, Copy)]
enum Scenario {
    /// Lists the catalog of a fake that works.
    Catalog,
    /// Lists the catalog of a fake told to fail.
    BrokenCatalog,
    /// Reads a manifest whose path the fake fails on.
    BrokenManifest,
}

/// What a row of the cases asks of the fake.
type Input = Scenario;
/// How many relations the catalog lists, or how many manifests were refused.
type Value = usize;
type Failure = LanguageError;

/// The rows of the fake's own cases: a name, one case word, an input and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_catalog_of_a_fake_that_works_lists_relations"),
        word: Word::Succeeds,
        input: || Scenario::Catalog,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("fails_when_the_catalog_cannot_be_read"),
        word: Word::Fails,
        input: || Scenario::BrokenCatalog,
        expect: Expect::Refusal(|error| selects(matches!(error, LanguageError::Catalog { .. }))),
    },
    Case {
        name: CaseName("fails_when_a_manifest_does_not_parse"),
        word: Word::Fails,
        input: || Scenario::BrokenManifest,
        expect: Expect::Refusal(|error| selects(matches!(error, LanguageError::Manifest { .. }))),
    },
];

/// Calls the fake on one input.
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Catalog => LanguageFake::default()
            .relations()
            .map(|relations| usize::from(!relations.0.is_empty())),
        Scenario::BrokenCatalog => LanguageFake {
            catalog: FakeCatalog::Fails,
            ..LanguageFake::default()
        }
        .relations()
        .map(|relations| relations.0.len()),
        Scenario::BrokenManifest => {
            let root = RootPath("fake".into());
            let notes = ProjectNotes::default();
            let manifests = [ManifestFile {
                path: ryo_language::domain::files::types::FilePath("broken/manifest.fake".into()),
                text: ManifestText("".into()),
            }];
            let input = ProjectInput {
                repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
                root: &root,
                manifests: &manifests,
                files: &[],
                notes: &notes,
                wanted: &[],
            };
            LanguageFake::default()
                .project(&input, &mut Symbols::default(), &mut Discard)
                .map(|answer| answer.refused.len())
        }
    }
}

#[test]
fn succeeds() {
    check(&LanguageFake::default());
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
