//! What placement finds over the `check` fixture's workspace: its instances, its placed file, its stray file, its missing
//! count, and the files a `{name:dir}` capture takes only beside the directory it names.

use super::service::place;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::fixtures::{matching, workspace_of};
use crate::domain::findings::service::printed;
use crate::domain::placement::errors::PlacementError;
use crate::inbound::use_cases::check::fixtures::{SPEC, WORKSPACE};

/// The files of the tree, and the spec placed.
type Input = (&'static [&'static str], &'static str);
/// The scopes, the placed files and the findings, in words.
type Value = String;
type Failure = PlacementError;

/// A spec whose template entry excludes by a regex that does not compile.
const BROKEN: &str = "description = \"d\"\nwhy = \"w\"\ntemplates = [{ template = \"errors\", file = \"errors.txt\", exclude = [{ regex = \"(\" }], description = \"e\" }]\n";

/// No spec at all: the spec entry names a release the workspace did not load.
const UNLOADED: &str = "";

/// A spec that places `errors.txt`, a file named for a directory beside it, and the files of a `notes/` directory.
const BESIDE: &str = "description = \"d\"\nwhy = \"w\"\ntemplates = [\n  { template = \"errors\", file = \"errors.txt\", description = \"e\" },\n  { alias = \"module\", template = \"errors\", file = \"{part:dir}.txt\", count = { min = 0 }, description = \"m\" },\n  { alias = \"note\", template = \"errors\", file = \"notes/{note}.txt\", count = { min = 0 }, description = \"n\" },\n]\n";

/// A spec whose entry writes a directory capture before the file's own name.
const NOT_LAST: &str = "description = \"d\"\nwhy = \"w\"\ntemplates = [{ template = \"errors\", file = \"{part:dir}/errors.txt\", description = \"e\" }]\n";

/// A spec whose entry excludes by a directory capture.
const IN_SELECTOR: &str = "description = \"d\"\nwhy = \"w\"\ntemplates = [{ template = \"errors\", file = \"errors.txt\", exclude = [\"{part:dir}.txt\"], description = \"e\" }]\n";

/// A spec whose param `own`, the instance's directory name, names the directory its file is named for.
const PARAMED: &str = "description = \"d\"\nwhy = \"w\"\ntemplates = [{ alias = \"module\", template = \"errors\", file = \"{own:dir}.txt\", count = { min = 0 }, description = \"m\" }]\n\n[params]\nown = { type = \"identifier\", from = \"directory-name\", description = \"o\" }\n";

/// A tree where `src/a/a.txt` has its `a/` beside it and `src/b/b.txt` has no `b/`.
const PARAMED_TREE: &[&str] = &[
    "tastemaker.toml",
    "src/a/a.txt",
    "src/a/a/x.txt",
    "src/b/b.txt",
];

/// A spec whose entry `x` writes more segments out than `y` but takes only a file named for a directory beside it.
const FALLBACK: &str = "description = \"d\"\nwhy = \"w\"\ntemplates = [\n  { template = \"errors\", file = \"errors.txt\", description = \"e\" },\n  { alias = \"x\", template = \"errors\", file = \"n/{a:dir}.txt\", count = { min = 0 }, description = \"x\" },\n  { alias = \"y\", template = \"errors\", file = \"{*o}/{b}.txt\", count = { min = 0 }, description = \"y\" },\n]\n";

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
            refused(
                actual,
                "(",
                "regex parse error:\n    (\n    ^\nerror: unclosed group",
            )
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
    Case {
        name: CaseName(
            "a_file_named_for_a_directory_beside_it_is_placed_and_one_without_is_closed",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "tastemaker.toml",
                    "src/a/errors.txt",
                    "src/a/helper.txt",
                    "src/a/lowering.txt",
                    "src/a/lowering/inner.txt",
                    "src/a/notes/n.txt",
                    "src/b/errors.txt",
                ],
                BESIDE,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_directory_of_that_name_elsewhere_does_not_place_the_file"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "tastemaker.toml",
                    "src/a/deep/helper/x.txt",
                    "src/a/errors.txt",
                    "src/a/helper.txt",
                    "src/b/errors.txt",
                    "src/b/helper/y.txt",
                ],
                BESIDE,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_directory_sharing_the_names_prefix_does_not_place_the_file"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "tastemaker.toml",
                    "src/a/errors.txt",
                    "src/a/low.txt",
                    "src/a/lowering/x.txt",
                    "src/b/errors.txt",
                ],
                BESIDE,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_directory_holding_only_directories_places_its_file"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "tastemaker.toml",
                    "src/a/errors.txt",
                    "src/a/lowering.txt",
                    "src/a/lowering/deeper/x.txt",
                    "src/b/errors.txt",
                ],
                BESIDE,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_role_file_beside_a_directory_of_its_name_stays_the_roles"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "tastemaker.toml",
                    "src/a/errors.txt",
                    "src/a/errors/x.txt",
                    "src/b/errors.txt",
                ],
                BESIDE,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_whose_name_captures_a_dot_or_two_names_no_directory"),
        word: Word::Succeeds,
        input: || (&["tastemaker.toml", "src/a/..txt", "src/a/...txt"], BESIDE),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_directory_capture_before_the_files_name_is_refused"),
        word: Word::Refuses,
        input: || (&["tastemaker.toml", "src/a/errors.txt"], NOT_LAST),
        expect: Expect::Refusal(|actual| {
            refused(
                actual,
                "{part:dir}/errors.txt",
                "a `{name:dir}` capture is written only in the last segment of a template entry's `file`, the segment that names the file",
            )
        }),
    },
    Case {
        name: CaseName("a_directory_capture_in_a_selector_list_is_refused"),
        word: Word::Refuses,
        input: || (&["tastemaker.toml", "src/a/errors.txt"], IN_SELECTOR),
        expect: Expect::Refusal(|actual| {
            refused(
                actual,
                "{part:dir}.txt",
                "a `{name:dir}` capture is written only in a template entry's `file`, never in a selector list, which reads no workspace",
            )
        }),
    },
    Case {
        name: CaseName("a_stray_beside_no_directory_falls_to_a_less_specific_entry"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "tastemaker.toml",
                    "src/a/errors.txt",
                    "src/a/n/h.txt",
                    "src/b/errors.txt",
                ],
                FALLBACK,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_directory_capture_of_a_param_needs_the_directory_its_value_names"),
        word: Word::Succeeds,
        input: || (PARAMED_TREE, PARAMED),
        expect: Expect::Value(String::new()),
    },
];

/// Whether `actual` is the refusal of the entry `p/part/errors`'s `pattern`, for `reason`.
fn refused(actual: &PlacementError, pattern: &str, reason: &str) -> crate::cases::Selection {
    let source = crate::domain::selectors::errors::SelectorsError::Pattern {
        pattern: pattern.into(),
        reason: reason.into(),
    };
    if actual
        == &(PlacementError::Selector {
            entry: "p/part/errors".into(),
            source,
        })
    {
        crate::cases::Selection::Matches
    } else {
        crate::cases::Selection::Differs
    }
}

/// The scopes, placed files and findings a row's tree and spec are right to give.
fn expected(files: &[&str], spec: &str) -> &'static str {
    match (spec, files) {
        (BESIDE, [.., "src/a/notes/n.txt", _]) => {
            "2 src/a/errors.txt,src/a/lowering.txt,src/a/notes/n.txt,src/b/errors.txt closed:src/a/helper.txt,closed:src/a/lowering/inner.txt,p/part/errors(module)#count,p/part/errors(note)#count"
        }
        (BESIDE, [_, "src/a/deep/helper/x.txt", ..]) => {
            "2 src/a/errors.txt,src/b/errors.txt closed:src/a/deep/helper/x.txt,closed:src/a/helper.txt,closed:src/b/helper/y.txt,p/part/errors(module)#count,p/part/errors(module)#count,p/part/errors(note)#count,p/part/errors(note)#count"
        }
        (BESIDE, [_, _, "src/a/low.txt", ..]) => {
            "2 src/a/errors.txt,src/b/errors.txt closed:src/a/low.txt,closed:src/a/lowering/x.txt,p/part/errors(module)#count,p/part/errors(module)#count,p/part/errors(note)#count,p/part/errors(note)#count"
        }
        (BESIDE, [_, _, _, "src/a/lowering/deeper/x.txt", _]) => {
            "2 src/a/errors.txt,src/a/lowering.txt,src/b/errors.txt closed:src/a/lowering/deeper/x.txt,p/part/errors(module)#count,p/part/errors(note)#count,p/part/errors(note)#count"
        }
        (BESIDE, [_, "src/a/..txt", _]) => {
            "1  closed:src/a/...txt,closed:src/a/..txt,p/part/errors#count,p/part/errors(module)#count,p/part/errors(note)#count"
        }
        (BESIDE, _) => {
            "2 src/a/errors.txt,src/b/errors.txt closed:src/a/errors/x.txt,p/part/errors(module)#count,p/part/errors(module)#count,p/part/errors(note)#count,p/part/errors(note)#count"
        }
        (PARAMED, _) => {
            "2 src/a/a.txt closed:src/a/a/x.txt,closed:src/b/b.txt,p/part/errors(module)#count"
        }
        (FALLBACK, _) => {
            "2 src/a/errors.txt@0,src/a/n/h.txt@2,src/b/errors.txt@0 p/part/errors(x)#count,p/part/errors(x)#count"
        }
        (_, [_, _, "src/b/errors.txt"]) => "2 src/a/errors.txt,src/b/errors.txt ",
        (_, [_, _, "src/b/inner/errors.txt"]) => {
            "2 src/a/errors.txt closed:src/b/inner/errors.txt,p/part/errors#count"
        }
        _ => "2 src/a/errors.txt closed:src/a/stray.txt,closed:src/b/notes.txt,p/part/errors#count",
    }
}

fn subject((files, spec): Input) -> Result<Value, Failure> {
    let mut loaded = workspace_of(WORKSPACE, if spec == UNLOADED { SPEC } else { spec }, files);
    if spec == UNLOADED {
        loaded.specs.clear();
    }
    let placement = place(
        &loaded,
        matching().compile,
        &crate::outbound::manifest::factory::toml(),
    )?;
    let instances = placement.scopes.len() - 1;
    // Where two entries could place a file, the entry that placed it is written beside it.
    let placed = placement
        .placed
        .iter()
        .map(|(path, file)| {
            if spec == FALLBACK {
                format!("{}@{}", path.0, file.entry.0)
            } else {
                path.0.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut found = placement
        .findings
        .iter()
        .map(|finding| printed(&finding.rule).0.to_string())
        .collect::<Vec<_>>();
    // Which counts are refused is the count queries'; placement leaves each waiting with the number it found, and the
    // missing one found none.
    found.extend(
        placement
            .counts
            .iter()
            .filter(|check| check.found == crate::domain::entries::types::Amount(0))
            .map(|check| printed(&check.rule).0.to_string()),
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
    let expected = expected(files, spec);
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
