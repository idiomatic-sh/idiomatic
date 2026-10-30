//! What placement finds over the `check` fixture's workspace: its instances, its placed file, its stray file and its
//! missing count.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::service::place;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::types::{LoadedWorkspace, Role};
use crate::domain::loading::service::{begin, finish, next, rule_read, spec_read, template_read};
use crate::domain::packages::types::{
    PackageName, PackageRoot, Reach, ResolvedPackage, SourcePath,
};
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::errors::PlacementError;
use crate::domain::selectors::types::{PathTest, PatternFailure, Spelling};
use crate::inbound::use_cases::check::fixtures::{PACKAGE, RULE, SPEC, TEMPLATE, WORKSPACE};
use crate::outbound::manifest::interface::ManifestPort as _;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::matching::interface::MatchingPort as _;

/// The fixture's workspace over these files, loaded.
fn workspace(files: &[&str], spec: &str) -> LoadedWorkspace {
    let manifest = crate::outbound::manifest::factory::toml();
    let root = manifest
        .workspace_file(&DefinitionText(WORKSPACE.into()))
        .expect("the workspace reads");
    let Role::Root(section) = root.role else {
        panic!("a root file")
    };
    let mut packages = BTreeMap::new();
    drop(
        packages.insert(
            PackageName("p".into()),
            ResolvedPackage {
                directory: PackageRoot("/p".into()),
                pin: crate::domain::packages::types::PackageSource::Path(SourcePath("p".into())),
                manifest: manifest
                    .package(&DefinitionText(PACKAGE.into()))
                    .expect("the package reads"),
                reach: Reach::Direct,
            },
        ),
    );
    let mut loading = begin(LoadedWorkspace {
        file: RepoPath("tastemaker.toml".into()),
        directory: RepoPath::default(),
        section,
        composition: root.composition,
        locals: Vec::new(),
        packages,
        specs: BTreeMap::new(),
        rules: BTreeMap::new(),
        templates: BTreeMap::new(),
        latest: BTreeMap::new(),
        files: files.iter().map(|file| RepoPath((*file).into())).collect(),
        local_packages: Vec::new(),
    });
    while let Some(wanted) = next(&mut loading) {
        match &*wanted.file.0 {
            file if file.ends_with("rule.toml") => {
                rule_read(
                    &mut loading,
                    &wanted,
                    manifest
                        .rule(&DefinitionText(RULE.into()))
                        .expect("the rule reads"),
                );
            }
            file if file.ends_with("spec.toml") => {
                let problems = spec_read(
                    &mut loading,
                    &wanted,
                    manifest
                        .spec(&DefinitionText(spec.into()))
                        .expect("the spec reads"),
                );
                assert!(
                    problems.is_empty(),
                    "the fixture spec has no variable problems"
                );
            }
            _ => template_read(
                &mut loading,
                &wanted,
                manifest
                    .template(&DefinitionText(TEMPLATE.into()))
                    .expect("the template reads"),
            ),
        }
    }
    finish(loading).workspace
}

fn compile(spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
    crate::outbound::matching::factory::globset().compile(spelling)
}

/// The files of the tree, and the spec placed.
type Input = (&'static [&'static str], &'static str);
/// The scopes, the placed files and the findings, in words.
type Value = String;
type Failure = PlacementError;

/// A spec whose template entry excludes by a regex that does not compile.
const BROKEN: &str = "description = \"d\"\nwhy = \"w\"\ntemplates = [{ template = \"errors\", file = \"errors.txt\", exclude = [{ regex = \"(\" }], description = \"e\" }]\n";

/// No spec at all: the spec entry names a release the workspace did not load.
const UNLOADED: &str = "";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_domain_is_an_instance_that_places_its_errors_file"),
        word: Word::Succeeds,
        input: || {
            (
                &["tastemaker.toml", "src/a/errors.txt", "src/b/errors.txt"],
                SPEC,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_stray_file_and_a_missing_file_are_found"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "tastemaker.toml",
                    "src/a/errors.txt",
                    "src/a/stray.txt",
                    "src/b/notes.txt",
                ],
                SPEC,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_directory_holding_only_directories_is_an_instance_over_its_subtree"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "tastemaker.toml",
                    "src/a/errors.txt",
                    "src/b/inner/errors.txt",
                ],
                SPEC,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_pattern_that_does_not_compile_is_refused"),
        word: Word::Refuses,
        input: || (&["tastemaker.toml", "src/a/errors.txt"], BROKEN),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(PlacementError::Selector {
                    entry: "p/part/errors".into(),
                    source: crate::domain::selectors::errors::SelectorsError::Pattern {
                        pattern: "(".into(),
                        reason: "regex parse error:\n    (\n    ^\nerror: unclosed group".into(),
                    },
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_spec_entry_whose_release_was_not_loaded_is_refused"),
        word: Word::Refuses,
        input: || (&["tastemaker.toml", "src/a/errors.txt"], UNLOADED),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(PlacementError::Unloaded {
                    entry: "p/part".into(),
                    release: "p/part".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn subject((files, spec): Input) -> Result<Value, Failure> {
    let mut loaded = workspace(files, if spec == UNLOADED { SPEC } else { spec });
    if spec == UNLOADED {
        loaded.specs.clear();
    }
    let placement = place(
        &loaded,
        &compile,
        &crate::outbound::manifest::factory::toml(),
    )?;
    let instances = placement.scopes.len() - 1;
    let placed = placement
        .placed
        .keys()
        .map(|path| path.0.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let mut found = placement
        .findings
        .iter()
        .map(|finding| {
            crate::domain::findings::service::printed(&finding.rule)
                .0
                .to_string()
        })
        .collect::<Vec<_>>();
    // Which counts are refused is the count queries'; placement leaves each waiting with the number it found, and the
    // missing one found none.
    found.extend(
        placement
            .counts
            .iter()
            .filter(|check| check.found == crate::domain::entries::types::Amount(0))
            .map(|check| {
                crate::domain::findings::service::printed(&check.rule)
                    .0
                    .to_string()
            }),
    );
    // Which files of a closed directory are strays is the closed query's; placement leaves the files beneath each closed directory
    // that no template places and no `tastemaker.toml` is.
    found.extend(
        placement
            .closings
            .candidates
            .iter()
            .filter(|candidate| {
                !placement.placed.contains_key(&candidate.path)
                    && !placement.closings.own.contains(&candidate.path)
            })
            .map(|candidate| format!("closed:{}", candidate.path.0)),
    );
    found.sort();
    let said = format!("{instances} {placed} {}", found.join(","));
    // `src/b` holds only `inner/`: it is an instance all the same, so its missing errors file is counted and the file of its
    // subtree is beneath its closed directory.
    let expected = match files {
        [_, _, "src/b/errors.txt"] => "2 src/a/errors.txt,src/b/errors.txt ",
        [_, _, "src/b/inner/errors.txt"] => {
            "2 src/a/errors.txt closed:src/b/inner/errors.txt,p/part/errors#count"
        }
        _ => "2 src/a/errors.txt closed:src/a/stray.txt,closed:src/b/notes.txt,p/part/errors#count",
    };
    Ok(if said == expected {
        String::new()
    } else {
        said
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
