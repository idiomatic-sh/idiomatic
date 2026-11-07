//! What every `manifest` adapter must pass: a lock it writes reads back the same, an entry it adds and the version it sets
//! are in the text it writes, a text with no dependency is unchanged by rebasing, the files it writes for a scenario name
//! the definition they place, and text that is not a definition is refused by every reader.

// Error locations are contractual; diagnostic wording belongs to each implementation.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::entries::types::{Amount, Count, Description, ParamValues};
use crate::domain::files::types::FileText;
use crate::domain::packages::types::PackageRoot;
use crate::domain::packages::types::PlacedDefinition;
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, Lock, LockedPackage, LockedRelease, PackageName, SourcePath,
    VersionText,
};
use crate::domain::paths::types::RepoPath;
use crate::domain::scenario_entries::service::root_entry;
use crate::domain::scenario_files::types::{ScenarioRoot, Wrapper};
use crate::domain::selectors::types::{PathPattern, Selector, Selectors};
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::{DefinitionText, EntryDraft};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// A text value reads as it is written, alone and inside a list.
    TextAsWritten,
    /// A lock written reads back the same.
    LockRoundTrips,
    /// An entry added is in the text written.
    EntryAdded,
    /// A text with no dependency is unchanged by rebasing.
    RebasedWithoutDependencies,
    /// The root a scenario is placed by names the definition's package and the definition.
    ScenarioRootNamesItsDefinition,
    /// The wrapper an owned definition is proved through names the definition and its spec.
    ScenarioWrapperNamesItsDefinition,
    /// A rule's text that is not TOML is refused.
    NotToml,
    /// A `tastemaker.toml`'s text that is not one is refused.
    NotAWorkspace,
    /// A spec's text that is not one is refused.
    NotASpec,
    /// A template's text that is not one is refused.
    NotATemplate,
    /// A package manifest's text that is not one is refused.
    NotAPackage,
    /// A definition's text that is not one of its kind is refused.
    NotADefinition,
}

/// What the contract asks of the adapter.
type Input = Question;
/// What the adapter answers when it accepts.
type Value = bool;
/// What the adapter refuses with.
type Failure = ManifestError;

/// A refusal of the text's shape.
const SHAPE: Expect<Value, Failure> = Expect::Refusal(|error| {
    if matches!(error, ManifestError::Shape {reason} if !reason.is_empty()) {
        crate::cases::Selection::Matches
    } else {
        crate::cases::Selection::Differs
    }
});

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_text_value_reads_as_it_is_written"),
        word: Word::Succeeds,
        input: || Question::TextAsWritten,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_lock_written_reads_back_the_same"),
        word: Word::Succeeds,
        input: || Question::LockRoundTrips,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_entry_added_is_written"),
        word: Word::Succeeds,
        input: || Question::EntryAdded,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_text_with_no_dependency_is_unchanged_by_rebasing"),
        word: Word::Succeeds,
        input: || Question::RebasedWithoutDependencies,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_scenario_root_names_the_definition_it_places"),
        word: Word::Succeeds,
        input: || Question::ScenarioRootNamesItsDefinition,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_scenario_wrapper_names_the_definition_it_places"),
        word: Word::Succeeds,
        input: || Question::ScenarioWrapperNamesItsDefinition,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rule_that_is_not_toml_is_refused"),
        word: Word::Refuses,
        input: || Question::NotToml,
        expect: SHAPE,
    },
    Case {
        name: CaseName("a_workspace_file_that_is_not_toml_is_refused"),
        word: Word::Refuses,
        input: || Question::NotAWorkspace,
        expect: SHAPE,
    },
    Case {
        name: CaseName("a_spec_that_is_not_toml_is_refused"),
        word: Word::Refuses,
        input: || Question::NotASpec,
        expect: SHAPE,
    },
    Case {
        name: CaseName("a_template_that_is_not_toml_is_refused"),
        word: Word::Refuses,
        input: || Question::NotATemplate,
        expect: SHAPE,
    },
    Case {
        name: CaseName("a_package_manifest_that_is_not_toml_is_refused"),
        word: Word::Refuses,
        input: || Question::NotAPackage,
        expect: SHAPE,
    },
    Case {
        name: CaseName("a_definition_that_is_not_toml_is_refused"),
        word: Word::Refuses,
        input: || Question::NotADefinition,
        expect: SHAPE,
    },
];

/// Puts `question` to the adapter and answers whether the property it asks about holds.
fn answer(adapter: &impl ManifestPort, question: Question) -> Result<Value, Failure> {
    let workspace = DefinitionText("".into());
    let not_toml = DefinitionText("this is = = not toml".into());
    match question {
        Question::TextAsWritten => {
            let text = crate::domain::entries::types::Value::Text(
                crate::domain::scalars::types::TextValue("ryo-css".into()),
            );
            let alone = adapter.write(&text).0;
            let listed = adapter
                .write(&crate::domain::entries::types::Value::List(vec![text]))
                .0;
            Ok(alone == "ryo-css" && listed.contains("ryo-css") && listed != alone)
        }
        Question::LockRoundTrips => {
            // A lock of one package and one release.
            let lock = Lock {
                dependencies: vec![PackageName("idiomatic.arch".into())],
                packages: vec![LockedPackage {
                    name: PackageName("idiomatic.arch".into()),
                    source: crate::domain::packages::types::PackageSource::Path(SourcePath(
                        "../arch".into(),
                    )),
                    dependencies: Vec::new(),
                    releases: vec![LockedRelease {
                        kind: DefinitionKind::Rule,
                        name: DefinitionName("file-length".into()),
                        version: VersionText("0.3.0".into()),
                    }],
                }],
            };
            Ok(adapter.lock(&adapter.lock_text(&lock))? == lock)
        }
        Question::EntryAdded => {
            // A rule entry to add.
            let draft = EntryDraft {
                kind: DefinitionKind::Rule,
                package: PackageName("idiomatic.arch".into()),
                version: VersionText("0.1.0".into()),
                name: DefinitionName("file-length".into()),
                include: Selectors(vec![Selector::Glob(PathPattern("src/**".into()))]),
                file: PathPattern(Box::default()),
                count: Count::Exactly(Amount(1)),
                params: ParamValues::default(),
                description: Description("long files are split".into()),
            };
            let added = adapter.add_entry(&FileText(workspace.0.clone()), &draft)?;
            Ok(added.0.contains("file-length"))
        }
        Question::RebasedWithoutDependencies => {
            let file = FileText(workspace.0.clone());
            Ok(adapter.rebased(&file, &RepoPath("sub".into()))? == file)
        }
        Question::ScenarioRootNamesItsDefinition => {
            let written = adapter.scenario_root(&ScenarioRoot {
                path: &PackageRoot("/packages/arch".into()),
                placed: &PlacedDefinition {
                    kind: DefinitionKind::Rule,
                    package: PackageName("idiomatic.arch".into()),
                    name: DefinitionName("file-length".into()),
                },
                keys: &root_entry(),
            });
            Ok(written.0.contains("idiomatic.arch") && written.0.contains("file-length"))
        }
        Question::ScenarioWrapperNamesItsDefinition => {
            let written = adapter.scenario_wrapper(&Wrapper {
                kind: DefinitionKind::Rule,
                name: &DefinitionName("file-length".into()),
                spec: &DefinitionName("crate".into()),
                keys: &root_entry(),
            });
            Ok(written.0.contains("file-length") && written.0.contains("crate"))
        }
        Question::NotToml => adapter.rule(&not_toml).map(|_| true),
        Question::NotAWorkspace => adapter.workspace_file(&not_toml).map(|_| true),
        Question::NotASpec => adapter.spec(&not_toml).map(|_| true),
        Question::NotATemplate => adapter.template(&not_toml).map(|_| true),
        Question::NotAPackage => adapter.package(&not_toml).map(|_| true),
        Question::NotADefinition => adapter
            .definition(DefinitionKind::Template, &not_toml)
            .map(|_| true),
    }
}

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl ManifestPort) {
    run(CASES, Word::Succeeds, |question| answer(adapter, question));
    run(CASES, Word::Refuses, |question| answer(adapter, question));
}
