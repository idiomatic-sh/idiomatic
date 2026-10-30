// ignore: idiomatic.arch/file-length - one scaffold case matrix covers placement, rendering, filesystem refusals, and complete error context against its shared subject.
//! What the `scaffold` use case promises, against the fakes of the ports it calls.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::errors::ScaffoldError;
use super::fixtures::{
    CYCLING_TEMPLATE, GENERATED_TEMPLATE, HEX_PACKAGE, HEX_WORKSPACE, LANGUAGE_PACKAGE,
    LANGUAGE_WORKSPACE, NAMED_SPEC, NAMED_TEMPLATE, NAMED_WORKSPACE, UNBOUND_WORKSPACE,
    UNREADABLE_WORKSPACE, UNRENDERABLE_TEMPLATE,
};
use super::interface::ScaffoldUseCase as _;
use super::types::ScaffoldRequest;
use super::use_case::ScaffoldUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::entries::types::Setting;
use crate::domain::files::types::MachinePath;
use crate::domain::packages::types::DefinitionName;
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;
use crate::inbound::use_cases::check::fixtures::{PACKAGE, RULE, package, repository};
use crate::outbound::packages::fixtures::fake::PackagesFake;
use crate::outbound::repository::fixtures::fake::RepositoryFake;
use crate::outbound::templates::errors::TemplatesError;

/// The entry, and the directory of the instance.
type Input = (&'static str, &'static str);
/// The files written and kept, joined.
type Value = String;
type Failure = ScaffoldError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_target_outside_the_repository_is_refused"),
        word: Word::Refuses,
        input: || ("part", "../outside"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::InvalidPath {
                    path: "../outside".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_target_that_still_captures_is_refused"),
        word: Word::Refuses,
        input: || ("part", "src/{name}"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::UnresolvedAt {
                    at: "src/{name}".into(),
                    capture: "name".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_absolute_target_is_refused"),
        word: Word::Refuses,
        input: || ("part", "/outside"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::InvalidPath {
                    path: "/outside".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_template_that_negates_its_presence_refuses_without_writing"),
        word: Word::Refuses,
        input: || ("cycling", "src/b.cycle"),
        expect: Expect::error(ScaffoldError::Unsettled),
    },
    Case {
        name: CaseName("an_instance_outside_its_entry_is_refused_by_its_directory"),
        word: Word::Refuses,
        input: || ("named", "outside"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::OutsideTarget {
                    at: "outside".into(),
                    entry: "named".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_directory_its_include_does_not_select_is_refused"),
        word: Word::Refuses,
        input: || ("hexish", "lib/p"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::OutsideTarget {
                    at: "lib/p".into(),
                    entry: "hexish".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_set_name_no_file_of_the_spec_reads_is_refused_by_name"),
        word: Word::Refuses,
        input: || ("lang-part", "langs/typo"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::Unread {
                    name: "kidn".into(),
                    entry: "lang-part".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_set_name_the_template_entry_does_not_read_is_refused_by_name"),
        word: Word::Refuses,
        input: || ("manifest", "typo"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::Unread {
                    name: "domains".into(),
                    entry: "manifest".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_condition_whose_rule_param_nothing_gives_is_undecided"),
        word: Word::Refuses,
        input: || ("unbound", "."),
        expect: Expect::Refusal(|actual| {
            if matches!(actual, ScaffoldError::Undecided { .. }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_spec_entry_writes_the_files_its_instance_lacks"),
        word: Word::Succeeds,
        input: || ("part", "src/b"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_that_exists_is_kept"),
        word: Word::Succeeds,
        input: || ("part", "src/a"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_spec_param_naming_a_capture_is_written_with_its_value_and_a_conditional_file_waits",
        ),
        word: Word::Succeeds,
        input: || ("named", "src/b"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_template_param_naming_a_capture_is_written_with_its_value"),
        word: Word::Succeeds,
        input: || ("manifest", "src/b.toml"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_its_template_renders_nothing_into_is_refused"),
        word: Word::Refuses,
        input: || ("generated", "src/b.lock"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::Unwritten {
                    file: "src/b.lock".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_language_is_written_whole_from_its_directory_and_what_is_set"),
        word: Word::Succeeds,
        input: || ("lang-part", "langs/go"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_required_file_whose_capture_nothing_gave_is_refused"),
        word: Word::Refuses,
        input: || ("lang-part", "langs/bare"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::Capture {
                    file: "items/{lib}.txt".into(),
                    capture: "lib".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_param_its_template_declares_and_nothing_gave_is_refused_by_name"),
        word: Word::Refuses,
        input: || ("lang-part", "langs/nokind"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::Param {
                    file: "langs/nokind/needs.txt".into(),
                    param: "kind".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_workspace_file_that_is_no_toml_fails_with_its_name"),
        word: Word::Fails,
        input: || ("unreadable", "src/b"),
        expect: Expect::Refusal(|actual| {
            if actual == &(ScaffoldError::Manifest {
            file: "tastemaker.toml".into(),
            source: crate::outbound::manifest::errors::ManifestError::Shape {
                reason: "TOML parse error at line 1, column 13\n  |\n1 | version = [2\n  |             ^\nunclosed array, expected `]`\n".into(),
            },
        }) { crate::cases::Selection::Matches } else { crate::cases::Selection::Differs }
        }),
    },
    Case {
        name: CaseName("a_template_that_does_not_render_fails_with_its_file"),
        word: Word::Fails,
        input: || ("unrenderable", "src/b.bad"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::Render {
                    file: "src/b.bad".into(),
                    source: TemplatesError::Render {
                        reason: "unknown filter: filter no_such_filter is unknown (in <string>:1)"
                            .into(),
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
        name: CaseName("a_hex_is_written_with_the_files_of_its_kind_and_its_module_file"),
        word: Word::Succeeds,
        input: || ("hexish", "src/p"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_hex_scaffolded_without_its_kind_is_refused"),
        word: Word::Refuses,
        input: || ("hexish", "src/n"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::Render {
                    file: "src/n/errors.txt".into(),
                    source: TemplatesError::Render {
                        reason: "invalid operation: say the kind (in <string>:1)".into(),
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
        name: CaseName("a_spec_named_by_its_package_writes_the_files_its_instance_lacks"),
        word: Word::Succeeds,
        input: || ("p/part", "src/b"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_spec_named_at_its_version_writes_the_files_its_instance_lacks"),
        word: Word::Succeeds,
        input: || ("p/part@0.1.0", "src/b"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_template_named_as_explain_names_it_writes_its_file_in_the_instance_at"),
        word: Word::Succeeds,
        input: || ("p/part/errors", "src/b"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_template_named_as_explain_names_it_keeps_the_file_that_exists"),
        word: Word::Succeeds,
        input: || ("p/part/errors", "src/a"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_qualified_name_at_a_directory_no_entry_takes_is_refused"),
        word: Word::Refuses,
        input: || ("p/part/errors", "elsewhere/deep"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::NoEntry {
                    entry: "p/part/errors".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_entry_the_workspace_has_not_is_refused"),
        word: Word::Refuses,
        input: || ("nothing", "src/c"),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ScaffoldError::NoEntry {
                    entry: "nothing".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// The manifest `entry` writes for `src/b` (the spec at `src/b`, the template entry at `src/b.toml`), its capture
/// substituted into the param.
fn named(entry: &str, written: &str) -> Result<Value, Failure> {
    let (scaffold, repository) = over(
        NAMED_WORKSPACE,
        "p",
        &[
            ("package.toml", PACKAGE),
            ("specs/named/0.1.0/spec.toml", NAMED_SPEC),
            ("specs/named/0.1.0/rules/present/rule.toml", RULE),
            (
                "specs/named/0.1.0/templates/manifest/template.toml",
                NAMED_TEMPLATE,
            ),
            ("templates/manifest/0.1.0/template.toml", NAMED_TEMPLATE),
            ("templates/cycling/0.1.0/template.toml", CYCLING_TEMPLATE),
            (
                "templates/generated/0.1.0/template.toml",
                GENERATED_TEMPLATE,
            ),
            (
                "templates/unrenderable/0.1.0/template.toml",
                UNRENDERABLE_TEMPLATE,
            ),
        ],
    );
    let files = repository.files;
    // Only the files the scaffold writes are read back, not its report.
    let _report = scaffold.scaffold(ScaffoldRequest {
        start: MachinePath("/fake".into()),
        entry: DefinitionName(entry.into()),
        at: RepoPath(if written == "outside" {
            "outside".into()
        } else {
            "src/b".into()
        }),
        // The spec's capture comes from where it is written; the template entry's is given.
        set: if entry == "named" {
            BTreeMap::new()
        } else if written == "typo" {
            BTreeMap::from([
                (Setting("domain".into()), Setting("b".into())),
                (Setting("domains".into()), Setting("b".into())),
            ])
        } else {
            BTreeMap::from([(Setting("domain".into()), Setting("b".into()))])
        },
        writing: Writing::Write,
    })?;
    let written = files
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&RepoPath(if written == "src/b" {
            "src/b/manifest.toml".into()
        } else {
            written.into()
        }))
        .cloned()
        .unwrap_or_default();
    let conditional = files
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .contains_key(&RepoPath("src/b/notes.toml".into()));
    Ok(if &*written.0 == "name = \"x-b\"\n" && !conditional {
        String::new()
    } else {
        written.0.to_string()
    })
}

/// The use case over a workspace file and its one package, held by the fakes of its ports, and the files it writes into.
fn over(
    workspace: &str,
    id: &str,
    package: &[(&str, &str)],
) -> (ScaffoldUseCaseImpl, RepositoryFake) {
    let repository = RepositoryFake::of(&[("tastemaker.toml", workspace)]);
    let held = repository.clone();
    let scaffold = ScaffoldUseCaseImpl::new(
        Arc::new(repository),
        Arc::new(crate::outbound::manifest::factory::toml()),
        Arc::new(PackagesFake::of(id, package)),
        Arc::new(crate::outbound::matching::factory::globset()),
        Arc::new(crate::outbound::templates::factory::minijinja()),
        Arc::new(crate::outbound::analysis::fixtures::fake::AnalysisFake::default()),
    );
    (scaffold, held)
}

/// The language scene: one spec placed by a capture of its directory, a required file for each library given with `--set`, a
/// nested part whose param has the capture in it, a file that waits on a condition, and the files read back.
fn language(at: &str) -> Result<Value, Failure> {
    let (scaffold, repository) = over(LANGUAGE_WORKSPACE, "q", LANGUAGE_PACKAGE);
    let files = repository.files;
    let report = scaffold.scaffold(ScaffoldRequest {
        start: MachinePath("/fake".into()),
        entry: DefinitionName("lang-part".into()),
        at: RepoPath(at.into()),
        set: match at {
            "langs/go" => BTreeMap::from([
                (Setting("lib".into()), Setting("x".into())),
                (Setting("kind".into()), Setting("core".into())),
            ]),
            "langs/nokind" => BTreeMap::from([(Setting("lib".into()), Setting("x".into()))]),
            "langs/typo" => BTreeMap::from([
                (Setting("lib".into()), Setting("x".into())),
                (Setting("kidn".into()), Setting("core".into())),
            ]),
            _ => BTreeMap::new(),
        },
        writing: Writing::Write,
    })?;
    let held = files
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let text = |path: &str| {
        held.get(&RepoPath(path.into()))
            .map(|found| found.0.to_string())
            .unwrap_or_default()
    };
    let said = format!(
        "{} ; {} ; {} ; {}",
        listed(&report.written),
        text("langs/go/inner/body.txt"),
        text("langs/go/items/x.txt"),
        held.contains_key(&RepoPath("langs/go/extra.txt".into())),
    );
    let expected = "langs/go/root.txt,langs/go/items/x.txt,langs/go/marker.txt,langs/go/needs.txt,langs/go/inner/body.txt ; trait=pkg::go ; item x ; false";
    Ok(against(said, expected))
}

/// The hex scene: the kind is what is set, and an optional file is written only by a template that says it starts a first writing.
fn hex(at: &str) -> Result<Value, Failure> {
    let (scaffold, _) = over(HEX_WORKSPACE, "h", HEX_PACKAGE);
    let report = scaffold.scaffold(ScaffoldRequest {
        start: MachinePath("/fake".into()),
        entry: DefinitionName("hexish".into()),
        at: RepoPath(at.into()),
        set: (at == "src/p")
            .then(|| (Setting("kind".into()), Setting("port".into())))
            .into_iter()
            .collect(),
        writing: Writing::Write,
    })?;
    let said = listed(&report.written);
    Ok(against(said, "src/p/errors.txt,src/p/ports.txt,src/p.txt"))
}

/// Nothing when `said` is what was expected, else what was said.
fn against(said: String, expected: &str) -> Value {
    if said == expected {
        String::new()
    } else {
        said
    }
}

/// The paths a report names, in order, joined by commas.
fn listed(paths: &[RepoPath]) -> String {
    paths
        .iter()
        .map(|path| &*path.0)
        .collect::<Vec<_>>()
        .join(",")
}

fn subject((entry, at): Input) -> Result<Value, Failure> {
    if ["named", "manifest", "generated", "unrenderable", "cycling"].contains(&entry) {
        return named(entry, at);
    }
    if entry == "lang-part" {
        return language(at);
    }
    if entry == "hexish" {
        return hex(at);
    }
    if entry == "unbound" {
        let (scaffold, _) = over(
            UNBOUND_WORKSPACE,
            "p",
            &[
                ("package.toml", PACKAGE),
                ("rules/marker/0.1.0/rule.toml", RULE),
                ("templates/manifest/0.1.0/template.toml", NAMED_TEMPLATE),
            ],
        );
        return scaffold
            .scaffold(ScaffoldRequest {
                start: MachinePath("/fake".into()),
                entry: DefinitionName(entry.into()),
                at: RepoPath(at.into()),
                set: BTreeMap::new(),
                writing: Writing::Write,
            })
            .map(|report| listed(&report.written));
    }
    let workspace = if entry == "unreadable" {
        RepositoryFake::of(&[("tastemaker.toml", UNREADABLE_WORKSPACE)])
    } else {
        repository()
    };
    let scaffold = ScaffoldUseCaseImpl::new(
        Arc::new(workspace),
        Arc::new(crate::outbound::manifest::factory::toml()),
        Arc::new(package()),
        Arc::new(crate::outbound::matching::factory::globset()),
        Arc::new(crate::outbound::templates::factory::minijinja()),
        Arc::new(crate::outbound::analysis::fixtures::fake::AnalysisFake::default()),
    );
    let report = scaffold.scaffold(ScaffoldRequest {
        start: MachinePath("/fake".into()),
        entry: DefinitionName(entry.into()),
        at: RepoPath(at.into()),
        set: BTreeMap::new(),
        writing: Writing::Write,
    })?;
    let said = format!(
        "wrote {} kept {}",
        listed(&report.written),
        listed(&report.kept)
    );
    let expected = if at == "src/b" {
        "wrote src/b/errors.txt kept "
    } else {
        "wrote  kept src/a/errors.txt"
    };
    Ok(against(said, expected))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
