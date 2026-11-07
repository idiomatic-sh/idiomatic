//! The `instance_params` service's cases, on values alone.

use std::collections::BTreeMap;

use super::service::params;
use super::types::Instance;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::Captures;
use crate::domain::definitions::types::{SpecFile, Why};
use crate::domain::entries::types::{
    Alias, Amount, Composition, Count, Described, Description, EntryHeader, EntryIdentity, Imports,
    ParamDeclaration, ParamDeclarations, ParamName, ParamSource, ParamType, ParamValue,
    ParamValues, SpecEntry, Value as EntryValue,
};
use crate::domain::packages::types::{DefinitionName, Ownership, PackageName, VersionText};
use crate::domain::paths::types::RepoPath;
use crate::domain::scalars::types::TextValue;
use crate::domain::scopes::service::scope_file;
use crate::domain::scopes::types::{CompositionRef, Identity};
use crate::domain::selectors::types::Selectors;

/// What the spec entry passes its one param, `lang`, and where the spec takes it from when nothing passes it.
#[derive(Clone, Copy, Debug)]
enum Passed {
    /// The entry passes `"rust"`.
    Literal,
    /// Nothing passes it, and the spec takes the directory's name.
    DirectoryName,
    /// Nothing passes it, and the spec has no other source.
    Nothing,
    /// The entry passes a param its spec does not declare.
    Undeclared,
}

/// What the entry passes.
type Input = Passed;
/// The value `lang` is given, and the messages of the findings, joined.
type Value = (String, String);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_literal_the_entry_passes_is_the_value"),
        word: Word::Succeeds,
        input: || Passed::Literal,
        expect: Expect::Value((String::new(), String::new())),
    },
    Case {
        name: CaseName("a_param_taken_from_the_directory_is_its_name"),
        word: Word::Succeeds,
        input: || Passed::DirectoryName,
        expect: Expect::Value((String::new(), String::new())),
    },
    Case {
        name: CaseName("a_param_nothing_gives_a_value_is_a_finding"),
        word: Word::Succeeds,
        input: || Passed::Nothing,
        expect: Expect::Value((String::new(), String::new())),
    },
    Case {
        name: CaseName("a_param_the_spec_does_not_declare_is_a_finding"),
        word: Word::Succeeds,
        input: || Passed::Undeclared,
        expect: Expect::Value((String::new(), String::new())),
    },
];

fn spec(source: ParamSource) -> SpecFile {
    let mut declared = BTreeMap::new();
    drop(declared.insert(
        ParamName("lang".into()),
        ParamDeclaration {
            kind: ParamType("identifier".into()),
            description: Described::Unsaid,
            source,
        },
    ));
    SpecFile {
        description: Description("d".into()),
        why: Why("w".into()),
        params: ParamDeclarations(declared),
        rules_exclude: Selectors::default(),
        composition: Composition::default(),
        scenarios: Vec::new(),
    }
}

fn entry(passed: BTreeMap<ParamName, ParamValue>) -> SpecEntry {
    SpecEntry {
        header: EntryHeader {
            package: PackageName("p".into()),
            version: VersionText("0.1.0".into()),
            ownership: Ownership::Shared,
            alias: Alias::Unnamed,
            description: Described::Unsaid,
            include: Selectors::default(),
            exclude: Selectors::default(),
            params: ParamValues(passed),
        },
        spec: DefinitionName("language".into()),
        count: Count::Exactly(Amount(1)),
        imports: Imports::Unrestricted,
    }
}

/// The value `lang` is given and the findings' messages, each against the one the row's case expects.
fn subject(passed: Input) -> Value {
    let mut given = BTreeMap::new();
    let (source, value, message) = match passed {
        Passed::Literal => {
            drop(given.insert(
                ParamName("lang".into()),
                ParamValue::Literal(EntryValue::Text(TextValue("rust".into()))),
            ));
            (ParamSource::Given, "rust", "")
        }
        Passed::DirectoryName => (ParamSource::DirectoryName, "toml", ""),
        Passed::Nothing => (
            ParamSource::Given,
            "",
            "`p/language` at `languages/toml` has no value for its param `lang`",
        ),
        Passed::Undeclared => {
            drop(given.insert(
                ParamName("extra".into()),
                ParamValue::Literal(EntryValue::Text(TextValue("x".into()))),
            ));
            (
                ParamSource::DirectoryName,
                "toml",
                "`p/language` passes `extra`, which its spec does not declare",
            )
        }
    };
    let spec = spec(source);
    let entry = entry(given);
    let anchor = RepoPath("tastemaker.toml".into());
    let directory = RepoPath("languages/toml".into());
    let parent = scope_file(
        RepoPath::default(),
        Identity("tastemaker.toml".into()),
        anchor.clone(),
        CompositionRef::Root,
        &Composition::default(),
    );
    let found = params(Instance {
        identity: &EntryIdentity("p/language".into()),
        entry: &entry,
        spec: &spec,
        anchor: &anchor,
        directory: &directory,
        parent: &parent,
        captures: &Captures::default(),
        workspace: &crate::domain::findings::types::WorkspaceName("ws".into()),
        written: &crate::outbound::manifest::factory::toml(),
    });
    let lang = found
        .values
        .0
        .values()
        .next()
        .map(|captured| captured.0.to_string())
        .unwrap_or_default();
    let said = found
        .findings
        .iter()
        .map(|finding| finding.message.0.to_string())
        .collect::<Vec<_>>()
        .join("|");
    (
        if lang == value { String::new() } else { lang },
        if said == message { String::new() } else { said },
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
