//! The `modules` service's cases: which manifest a path is, how a relative directory is resolved, which module a file is in, which
//! modules require which, and which a workspace holds.

use ryo::languages::FilePath;

use super::service::{
    holdings_in, manifestness_of, placed_in, replacement_text, resolve, wirings_in,
};
use super::types::{
    Declared, Directory, Indirectness, Manifest, ManifestPath, Member, ModulePath,
    ModuleReplacementTarget, ModuleVersion, Modules, Relative, Replacement, Requirement,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Which manifest a path is.
    Manifest(&'static str),
    /// The directory a relative one is from the directory of a manifest.
    Resolve(&'static str, &'static str),
    /// The module of each file of a repository of `go.mod` and `tools/go.mod`.
    Placement,
    /// The requirements of `app`, on the modules of a repository found by path or by a `replace`, or outside it, with their origin.
    Wiring,
    /// The modules a workspace uses.
    Workspace,
}

type Input = Question;
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_go_mod_is_known_by_the_last_part_of_its_path"),
        word: Word::Succeeds,
        input: || Question::Manifest("libs/core/go.mod"),
        expect: Expect::Value("Module"),
    },
    Case {
        name: CaseName("a_go_work_is_known_by_the_last_part_of_its_path"),
        word: Word::Succeeds,
        input: || Question::Manifest("go.work"),
        expect: Expect::Value("Workspace"),
    },
    Case {
        name: CaseName("a_file_whose_name_only_ends_like_a_manifest_is_none"),
        word: Word::Succeeds,
        input: || Question::Manifest("tools/mygo.mod"),
        expect: Expect::Value("Other"),
    },
    Case {
        name: CaseName("a_relative_directory_is_resolved_from_the_directory_of_the_manifest"),
        word: Word::Succeeds,
        input: || Question::Resolve("", "./app"),
        expect: Expect::Value("app"),
    },
    Case {
        name: CaseName("a_directory_above_is_resolved_from_a_nested_manifest"),
        word: Word::Succeeds,
        input: || Question::Resolve("libs/core", "../util"),
        expect: Expect::Value("libs/util"),
    },
    Case {
        name: CaseName("the_directory_above_a_top_directory_is_the_root"),
        word: Word::Succeeds,
        input: || Question::Resolve("a", ".."),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_directory_that_leaves_the_repository_is_none"),
        word: Word::Succeeds,
        input: || Question::Resolve("", "../out"),
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("a_file_belongs_to_the_nearest_module_above_it"),
        word: Word::Succeeds,
        input: || Question::Placement,
        expect: Expect::Value("0:0 1:0 2:2 3:2 4:0"),
    },
    Case {
        name: CaseName(
            "a_requirement_is_wired_by_the_path_of_a_module_or_the_directory_it_is_replaced_by_or_has_no_target_and_keeps_the_replace_of_its_version_before_one_of_every_version",
        ),
        word: Word::Succeeds,
        input: || Question::Wiring,
        expect: Expect::Value(
            "0>1 example.com/core Registry [], 0>2 example.com/util Path [../tools], 0>- example.com/far Registry [example.com/fork v1.2.0]",
        ),
    },
    Case {
        name: CaseName("a_workspace_holds_the_modules_in_the_directories_it_uses"),
        word: Word::Succeeds,
        input: || Question::Workspace,
        expect: Expect::Value("0 holds 1"),
    },
];

fn manifest(path: &str, module: Option<&str>) -> Manifest {
    Manifest {
        manifest: ManifestPath(path.into()),
        declared: Declared {
            module: module.map(|each| ModulePath(each.into())),
            ..Declared::default()
        },
    }
}

fn files(paths: &[&str]) -> Vec<FilePath> {
    paths.iter().map(|each| FilePath((*each).into())).collect()
}

fn answer(question: Input) -> String {
    match question {
        Question::Manifest(path) => format!("{:?}", manifestness_of(&FilePath(path.into()))),
        Question::Resolve(from, relative) => resolve(Directory(from), Relative(relative))
            .map_or_else(|| "none".to_owned(), |located| located.0),
        Question::Placement => {
            let modules = Modules {
                manifests: vec![
                    manifest("go.mod", Some("example.com/root")),
                    manifest("tools/go.mod", Some("example.com/tools")),
                ],
                refused: Vec::new(),
            };
            let listed = files(&[
                "go.mod",
                "main.go",
                "tools/go.mod",
                "tools/gen/gen.go",
                "docs/readme.md",
            ]);
            placed_in(&listed, &modules)
                .rows
                .iter()
                .map(|each| format!("{}:{}", each.path.0, each.module.0))
                .collect::<Vec<_>>()
                .join(" ")
        }
        Question::Wiring => {
            let mut app = manifest("app/go.mod", Some("example.com/app"));
            let required = |path: &str, indirect| Requirement {
                path: ModulePath(path.into()),
                version: ModuleVersion("v1.0.0".into()),
                indirect,
            };
            app.declared.requirements = vec![
                required("example.com/core", Indirectness::Direct),
                required("example.com/util", Indirectness::Direct),
                required("example.com/far", Indirectness::Indirect),
            ];
            app.declared.replacements = replacements();
            let modules = Modules {
                manifests: vec![
                    app,
                    manifest("core/go.mod", Some("example.com/core")),
                    manifest("tools/go.mod", Some("example.com/other")),
                ],
                refused: Vec::new(),
            };
            let listed = files(&["app/go.mod", "core/go.mod", "tools/go.mod"]);
            wirings_in(&listed, &modules)
                .rows
                .iter()
                .map(|each| {
                    let target = if each.target == ryo::languages::NO_FILE {
                        "-".to_owned()
                    } else {
                        each.target.0.to_string()
                    };
                    format!(
                        "{}>{target} {} {:?} [{}]",
                        each.source.0,
                        each.path.0,
                        each.origin,
                        replacement_text(each.replacement.as_ref()).0
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
        Question::Workspace => {
            let mut work = manifest("go.work", None);
            work.declared.uses = vec![Member("./app".into()), Member("./missing".into())];
            let modules = Modules {
                manifests: vec![work, manifest("app/go.mod", Some("example.com/app"))],
                refused: Vec::new(),
            };
            let listed = files(&["go.work", "app/go.mod"]);
            holdings_in(&listed, &modules)
                .rows
                .iter()
                .map(|each| format!("{} holds {}", each.workspace.0, each.module.0))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

/// The `replace`s of the app: a directory for `util`, another version of `core` than the one required, and for `far` a directory
/// for every version before a fork, at another version, for the version required.
fn replacements() -> Vec<Replacement> {
    vec![
        Replacement {
            from: ModulePath("example.com/util".into()),
            version: ModuleVersion(Box::default()),
            to: ModuleReplacementTarget("../tools".into()),
            to_version: ModuleVersion(Box::default()),
        },
        // A `replace` of another version than the one required does not apply.
        Replacement {
            from: ModulePath("example.com/core".into()),
            version: ModuleVersion("v0.9.0".into()),
            to: ModuleReplacementTarget("../elsewhere".into()),
            to_version: ModuleVersion(Box::default()),
        },
        // A `replace` of every version, written first, gives way to the one of the version required.
        Replacement {
            from: ModulePath("example.com/far".into()),
            version: ModuleVersion(Box::default()),
            to: ModuleReplacementTarget("../wild".into()),
            to_version: ModuleVersion(Box::default()),
        },
        Replacement {
            from: ModulePath("example.com/far".into()),
            version: ModuleVersion("v1.0.0".into()),
            to: ModuleReplacementTarget("example.com/fork".into()),
            to_version: ModuleVersion("v1.2.0".into()),
        },
    ]
}

fn subject(question: Input) -> Value {
    Box::leak(answer(question).into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
