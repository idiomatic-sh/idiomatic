//! What a template's content sees for a file a scope places.

use std::collections::BTreeMap;

use super::service::context;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::captures::types::Captures;
use crate::domain::contexts::types::{ContextKey, ContextRequest};
use crate::domain::definitions::types::{
    Exactness, LoadedWorkspace, TemplateFile, TemplateText, Why, WorkspaceSection,
};
use crate::domain::entries::types::{
    Alias, Amount, Composition, Count, Described, Description, EntryHeader, Imports,
    ParamDeclaration, ParamDeclarations, ParamName, ParamSource, ParamType, ParamValues,
    TemplateEntry,
};
use crate::domain::findings::types::WorkspaceName;
use crate::domain::packages::types::{DefinitionName, Ownership, PackageName, VersionText};
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::service::scope_file;
use crate::domain::scopes::types::{CompositionRef, Identity, ScopeIndex};
use crate::domain::selectors::types::{PathPattern, Selectors};

/// What a row asks of the context.
#[derive(Clone, Copy)]
enum Question {
    /// The value of a param that comes from here.
    Param(ParamSource),
    /// Whether an adapter's module file sees `errors` present when its port has one and the adapter, nested beneath, has
    /// none.
    Nested,
    /// The instances of `crate` a product-level file sees, beneath an `idiomatic-rust` instance over the whole product.
    Overlapping,
}

type Input = Question;
/// The value the content reads for `part`.
type Value = String;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_param_defaults_to_the_name_of_its_directory"),
        word: Word::Succeeds,
        input: || Question::Param(ParamSource::DirectoryName),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_param_nothing_gives_is_refused"),
        word: Word::Refuses,
        input: || Question::Param(ParamSource::Given),
        expect: Expect::Refusal(|message| {
            if message.contains("has no value for its param") {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_module_file_reads_what_the_part_nested_beside_it_places"),
        word: Word::Succeeds,
        input: || Question::Nested,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_instance_of_another_spec_over_the_path_hides_no_instance"),
        word: Word::Succeeds,
        input: || Question::Overlapping,
        expect: Expect::Value(String::new()),
    },
];

fn workspace() -> LoadedWorkspace {
    LoadedWorkspace {
        file: RepoPath("tastemaker.toml".into()),
        directory: RepoPath::default(),
        section: WorkspaceSection {
            name: WorkspaceName("w".into()),
            rules_exclude: Selectors::default(),
            members: Selectors::default(),
            dependencies: Vec::new(),
        },
        composition: Composition::default(),
        locals: Vec::new(),
        packages: BTreeMap::new(),
        specs: BTreeMap::new(),
        rules: BTreeMap::new(),
        templates: BTreeMap::new(),
        latest: BTreeMap::new(),
        files: Vec::new(),
        local_packages: Vec::new(),
    }
}

fn entry(template: &str, file: &str) -> TemplateEntry {
    TemplateEntry {
        header: EntryHeader {
            package: PackageName("p".into()),
            version: VersionText("0.1.0".into()),
            ownership: Ownership::Shared,
            alias: Alias::Unnamed,
            description: Described::Unsaid,
            include: Selectors::default(),
            exclude: Selectors::default(),
            params: ParamValues::default(),
        },
        template: DefinitionName(template.into()),
        file: PathPattern(file.into()),
        count: Count::Exactly(Amount(1)),
        imports: Imports::Unrestricted,
    }
}

fn template(text: &str, params: ParamDeclarations) -> TemplateFile {
    TemplateFile {
        description: Description("d".into()),
        why: Why("w".into()),
        params,
        exact: Exactness::Contains,
        content: TemplateText(text.into()),
        scenarios: Vec::new(),
    }
}

/// A port at `src/port` with its own `errors.txt`, and its adapter `ignore` nested at `src/port/adapters/ignore` with
/// none: what `present.errors` says for the adapter's module file, `src/port/adapters/ignore.txt`.
fn nested() -> Result<Value, Failure> {
    use crate::domain::captures::types::{CaptureName, CaptureValue};
    use crate::domain::definitions::types::LocalFile;
    use crate::domain::scopes::types::{EntryIndex, Matched, Origin};
    let implementation = Captures(BTreeMap::from([(
        CaptureName("implementation".into()),
        CaptureValue("ignore".into()),
    )]));
    let mut loaded = workspace();
    loaded.composition.templates = vec![
        entry("errors", "errors.txt"),
        entry("adapter-module", "adapters/{implementation}.txt"),
    ];
    let adapter = Composition {
        templates: vec![entry("errors", "errors.txt")],
        ..Composition::default()
    };
    loaded.locals.push(LocalFile {
        file: RepoPath("src/port/adapters/ignore/tastemaker.toml".into()),
        directory: RepoPath("src/port/adapters/ignore".into()),
        composition: adapter,
    });
    let port = scope_file(
        RepoPath("src/port".into()),
        Identity("tastemaker.toml".into()),
        RepoPath("tastemaker.toml".into()),
        CompositionRef::Root,
        &Composition::default(),
    );
    let mut beneath = port.clone();
    beneath.dir = RepoPath("src/port/adapters/ignore".into());
    beneath.captures = implementation.clone();
    beneath.composition = CompositionRef::Local(EntryIndex(0));
    beneath.origin = Origin::Instance {
        parent: ScopeIndex(0),
        entry: EntryIndex(0),
    };
    let matched = [
        Matched {
            scope: ScopeIndex(0),
            entry: EntryIndex(0),
            names: std::collections::BTreeSet::new(),
            tuples: std::collections::BTreeSet::from([Captures::default()]),
            counted: Amount(1),
            leading: std::collections::BTreeSet::new(),
        },
        Matched {
            scope: ScopeIndex(1),
            entry: EntryIndex(0),
            names: std::collections::BTreeSet::new(),
            tuples: std::collections::BTreeSet::new(),
            counted: Amount(0),
            leading: std::collections::BTreeSet::new(),
        },
    ];
    let scopes = [port, beneath];
    let placing = entry("adapter-module", "adapters/{implementation}.txt");
    let empty = template("", ParamDeclarations(BTreeMap::new()));
    let file = RepoPath("src/port/adapters/ignore.txt".into());
    let found = context(ContextRequest {
        workspace: &loaded,
        scopes: &scopes,
        matched: &matched,
        index: ScopeIndex(0),
        entry: &placing,
        template: &empty,
        file: &file,
        captures: &implementation,
        written: &crate::outbound::manifest::factory::toml(),
    })
    .map_err(|unvalued| unvalued.0)?;
    let errors = found
        .present
        .iter()
        .find(|(key, _)| key.0.ends_with("errors"))
        .map(|(_, present)| *present);
    Ok(match errors {
        Some(crate::domain::contexts::types::Present::No) => String::new(),
        other => format!("{other:?}"),
    })
}

/// The `crate` instances the product's own file sees: the crates beneath `idiomatic-rust`'s instance at the product, not
/// the one nested in a crate, and no instance a crate placed.
fn overlapping() -> Value {
    use crate::domain::packages::types::{DefinitionKind, Release};
    use crate::domain::scopes::types::{EntryIndex, Origin};
    let root = scope_file(
        RepoPath(String::new().into()),
        Identity("tastemaker.toml".into()),
        RepoPath("crates/p/tastemaker.toml".into()),
        CompositionRef::Root,
        &Composition::default(),
    );
    let instance = |dir: &str, spec: &str| {
        let mut scope = root.clone();
        scope.dir = RepoPath(dir.into());
        scope.composition = CompositionRef::Spec(Release {
            package: PackageName("idiomatic.arch".into()),
            kind: DefinitionKind::Spec,
            name: DefinitionName(spec.into()),
            version: VersionText("0.1.0".into()),
            ownership: Ownership::Shared,
        });
        scope.origin = Origin::Instance {
            parent: ScopeIndex(0),
            entry: EntryIndex(0),
        };
        scope
    };
    let scopes = [
        root.clone(),
        instance("crates/p", "idiomatic-rust"),
        instance("crates/p/cli", "crate"),
        instance("crates/p/crates/core", "crate"),
        instance("crates/p/crates/core/macros", "crate"),
        instance("crates/p/crates/core/src/domain/x", "concept"),
    ];
    let mut scopes = scopes;
    // The concept is the core crate's: placed by it, it is no instance the product's file sees.
    if let Some(concept) = scopes.last_mut() {
        concept.origin = Origin::Instance {
            parent: ScopeIndex(3),
            entry: EntryIndex(0),
        };
    }
    let found = super::service::instances(&scopes, ScopeIndex(0));
    if found.contains_key(&ContextKey("concept".into())) {
        return "the core's concept is seen".to_owned();
    }
    let seen = found
        .get(&ContextKey("crate".into()))
        .map(|each| {
            each.iter()
                .map(|values| {
                    values
                        .0
                        .get(&ContextKey("path".into()))
                        .map(|path| path.0.to_string())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    if seen == "cli crates/core" {
        String::new()
    } else {
        format!("the crates seen: [{seen}]")
    }
}

fn subject(question: Input) -> Result<Value, Failure> {
    let source = match question {
        Question::Param(source) => source,
        Question::Nested => return nested(),
        Question::Overlapping => return Ok(overlapping()),
    };
    let scopes = [scope_file(
        RepoPath("src/greeting".into()),
        Identity("tastemaker.toml".into()),
        RepoPath("tastemaker.toml".into()),
        CompositionRef::Root,
        &Composition::default(),
    )];
    let entry = TemplateEntry {
        header: EntryHeader {
            package: PackageName("p".into()),
            version: VersionText("0.1.0".into()),
            ownership: Ownership::Shared,
            alias: Alias::Unnamed,
            description: Described::Unsaid,
            include: Selectors::default(),
            exclude: Selectors::default(),
            params: ParamValues::default(),
        },
        template: DefinitionName("errors".into()),
        file: PathPattern("errors.txt".into()),
        count: Count::Exactly(Amount(1)),
        imports: Imports::Unrestricted,
    };
    let mut params = BTreeMap::new();
    drop(params.insert(
        ParamName("part".into()),
        ParamDeclaration {
            kind: ParamType("identifier".into()),
            description: Described::Unsaid,
            source,
        },
    ));
    let template = TemplateFile {
        description: Description("d".into()),
        why: Why("w".into()),
        params: ParamDeclarations(params),
        exact: Exactness::Contains,
        content: TemplateText("{{ part }}".into()),
        scenarios: Vec::new(),
    };
    let loaded = workspace();
    let file = RepoPath("src/greeting/errors.txt".into());
    let captures = Captures::default();
    let found = context(ContextRequest {
        workspace: &loaded,
        scopes: &scopes,
        matched: &[],
        index: ScopeIndex(0),
        entry: &entry,
        template: &template,
        file: &file,
        captures: &captures,
        written: &crate::outbound::manifest::factory::toml(),
    })
    .map_err(|unvalued| unvalued.0)?;
    let part = found
        .values
        .get(&ContextKey("part".into()))
        .map(|value| value.0.to_string())
        .unwrap_or_default();
    Ok(if part == "greeting" {
        String::new()
    } else {
        part
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
