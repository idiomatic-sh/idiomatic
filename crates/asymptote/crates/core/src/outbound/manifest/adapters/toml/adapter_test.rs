// ignore: idiomatic.arch/file-length - the adapter spec places one adapter_test.rs per adapter and no second one: its case table holds every answer of the manifest port, the members and hooks of a workspace and the tools a capture in inputs expands to, refusals included, over one temporary product each.
//! The TOML adapter of the manifest port: the contract over a product in a temporary directory, found from beneath
//! it, a file of another shape and a file it cannot read.

use super::adapter::ManifestTomlAdapter;
use super::constants::FILE;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifest::types::{Group, Hook, HookName, Manifest, ProductRoot};
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::fixtures::contract::check;
use crate::outbound::manifest::interface::ManifestPort as _;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The contract, and the product found from a directory beneath it.
    Contract,
    /// A start no manifest is above.
    Nowhere,
    /// A manifest of another shape.
    Shape,
    /// A manifest that is a directory.
    Unreadable,
    /// A workspace whose globs and paths match two members.
    Members,
    /// A glob that matches a directory with no manifest.
    MemberWithoutManifest,
    /// A path that does not exist.
    MemberMissing,
    /// A glob that matches nothing.
    MemberGlobEmpty,
    /// A member that declares `members`.
    MemberDeclaresMembers,
    /// A workspace beneath another.
    MembersBelowARoot,
    /// The root's `[hooks]`, asked for from a member beneath it.
    HooksFromMember,
    /// No `[hooks]` table: the default.
    HooksDefault,
    /// A hook that maps a group no tool declares.
    HooksUndeclared,
    /// A member that declares `[hooks]`.
    MemberDeclaresHooks,
    Topmost,
    WriteNew,
    WriteExists,
    /// A tool whose captured input matches no directory on disk.
    CapturedUnbound,
    /// A workspace root that pins the asymptote that runs, found from a member.
    PinnedHere,
    /// A workspace root that pins another asymptote, found from a member.
    PinnedElsewhere,
    /// A workspace root that pins a caret requirement the asymptote that runs satisfies, found from a member.
    PinnedCaret,
    /// A captured input that reaches an entry it cannot read.
    CapturedNotText,
}

type Input = Question;
/// Whether the answer is the expected one.
type Value = bool;
type Failure = ManifestError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_root_that_pins_the_asymptote_that_runs_is_found"),
        word: Word::Succeeds,
        input: || Question::PinnedHere,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "a_root_that_pins_a_caret_requirement_the_asymptote_that_runs_meets_is_found",
        ),
        word: Word::Succeeds,
        input: || Question::PinnedCaret,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_root_that_pins_another_asymptote_stops_every_command"),
        word: Word::Refuses,
        input: || Question::PinnedElsewhere,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Pinned { needed, installed } if needed == "9.9.9" && installed == env!("CARGO_PKG_VERSION"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_adapter_keeps_the_contract_and_finds_the_product_from_beneath"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_start_with_no_manifest_above_it_is_refused"),
        word: Word::Refuses,
        input: || Question::Nowhere,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Missing {start} if std::path::Path::new(start).is_absolute() && std::path::Path::new(start).file_name().is_some_and(|name| name.to_string_lossy().starts_with(".tmp")))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_manifest_of_another_shape_is_refused"),
        word: Word::Refuses,
        input: || Question::Shape,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Shape {path,reason} if path.ends_with("/asymptote.toml") && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_manifest_that_cannot_be_read_fails"),
        word: Word::Fails,
        input: || Question::Unreadable,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Unreadable {path,reason} if path.ends_with("/asymptote.toml") && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_workspace_names_its_members_by_glob_and_by_path_sorted"),
        word: Word::Succeeds,
        input: || Question::Members,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_member_glob_that_matches_a_directory_without_a_manifest_is_refused"),
        word: Word::Refuses,
        input: || Question::MemberWithoutManifest,
        expect: Expect::Refusal(|error| refused_member(error, "crates/b", "has no asymptote.toml")),
    },
    Case {
        name: CaseName("a_member_path_that_does_not_exist_is_refused"),
        word: Word::Refuses,
        input: || Question::MemberMissing,
        expect: Expect::Refusal(|error| refused_member(error, "tools/ghost", "no directory")),
    },
    Case {
        name: CaseName("a_member_glob_that_matches_nothing_is_refused"),
        word: Word::Refuses,
        input: || Question::MemberGlobEmpty,
        expect: Expect::Refusal(|error| refused_member(error, "none/*", "no directory")),
    },
    Case {
        name: CaseName("a_member_that_declares_members_is_refused_by_name"),
        word: Word::Refuses,
        input: || Question::MemberDeclaresMembers,
        expect: Expect::Refusal(|error| refused_member(error, "crates/a", "declares `members`")),
    },
    Case {
        name: CaseName("members_below_another_root_are_refused"),
        word: Word::Refuses,
        input: || Question::MembersBelowARoot,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::NotRoot { path } if path.ends_with("inner/asymptote.toml"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_roots_hooks_are_found_from_a_member_beneath_it"),
        word: Word::Succeeds,
        input: || Question::HooksFromMember,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("no_hooks_table_maps_no_hook"),
        word: Word::Succeeds,
        input: || Question::HooksDefault,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_topmost_manifest_is_the_workspace_root"),
        word: Word::Succeeds,
        input: || Question::Topmost,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_manifest_is_written_where_there_is_none"),
        word: Word::Succeeds,
        input: || Question::WriteNew,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_hook_mapping_a_group_no_tool_declares_is_refused_naming_it"),
        word: Word::Refuses,
        input: || Question::HooksUndeclared,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Hook { hook, reason } if hook == "pre-commit" && reason.ends_with("release"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_member_that_declares_hooks_is_refused_by_name"),
        word: Word::Refuses,
        input: || Question::MemberDeclaresHooks,
        expect: Expect::Refusal(|error| refused_member(error, "crates/a", "declares `hooks`")),
    },
    Case {
        name: CaseName("a_captured_tool_whose_inputs_match_nothing_is_refused_naming_them"),
        word: Word::Refuses,
        input: || Question::CapturedUnbound,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Shape { reason, .. } if reason.contains("`test-{lang}`") && reason.contains("langauges/{lang}"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_captured_input_that_reaches_an_entry_it_cannot_read_is_refused"),
        word: Word::Refuses,
        input: || Question::CapturedNotText,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Unreadable { .. }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_manifest_is_never_written_over_another"),
        word: Word::Refuses,
        input: || Question::WriteExists,
        expect: Expect::Refusal(|error| {
            if matches!(error, ManifestError::Unwritable { path, .. } if path.ends_with("/asymptote.toml"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Whether `error` refuses the member `named` for a reason that holds `said`.
fn refused_member(error: &ManifestError, named: &str, said: &str) -> crate::cases::Selection {
    if matches!(error, ManifestError::Member { member, reason } if member == named && reason.contains(said))
    {
        crate::cases::Selection::Matches
    } else {
        crate::cases::Selection::Differs
    }
}

const TOOL: &str = "[[tools]]\nid = \"t\"\nrun = [\"t\"]\ngroup = \"lint\"\n";

/// A directory `relative` to `base` holding `text` as its manifest.
fn product(base: &std::path::Path, relative: &str, text: &str) {
    let directory = base.join(relative);
    std::fs::create_dir_all(&directory).expect("a directory");
    std::fs::write(directory.join(FILE), text).expect("a manifest");
}

/// The manifest `asymptote init` writes: the default hook, and nothing else.
fn minimal() -> Manifest {
    Manifest {
        hooks: vec![Hook {
            name: HookName("pre-commit".into()),
            groups: vec![Group("lint".into())],
        }],
        ..Manifest::default()
    }
}

fn workspace(question: Question, base: &std::path::Path) -> Result<Value, Failure> {
    let manifest = |members: &str| format!("members = {members}\n{TOOL}");
    let root = |text: &str| product(base, ".", text);
    match question {
        Question::Members => {
            root(&manifest("[\"crates/*\", \"tools/one\"]"));
            for member in ["crates/b", "crates/a", "tools/one"] {
                product(base, member, TOOL);
            }
            product(base, "crates/not-a-file", TOOL);
            let found = ManifestTomlAdapter.members(&self::root(base))?.0;
            // Each member's directory beneath the root, read component by component: the platform's separator is its own.
            let shown: Vec<Vec<String>> = found
                .iter()
                .map(|member| {
                    std::path::Path::new(&*member.0)
                        .strip_prefix(base)
                        .expect("a member beneath the root")
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy().into_owned())
                        .collect()
                })
                .collect();
            Ok(shown
                == [
                    ["crates", "a"],
                    ["crates", "b"],
                    ["crates", "not-a-file"],
                    ["tools", "one"],
                ])
        }
        Question::MemberWithoutManifest => {
            root(&manifest("[\"crates/*\"]"));
            product(base, "crates/a", TOOL);
            std::fs::create_dir_all(base.join("crates/b")).expect("a directory");
            ManifestTomlAdapter.members(&self::root(base)).map(|_| true)
        }
        Question::MemberMissing => {
            root(&manifest("[\"tools/ghost\"]"));
            ManifestTomlAdapter.members(&self::root(base)).map(|_| true)
        }
        Question::MemberGlobEmpty => {
            root(&manifest("[\"none/*\"]"));
            ManifestTomlAdapter.members(&self::root(base)).map(|_| true)
        }
        Question::MemberDeclaresMembers => {
            root(&manifest("[\"crates/*\"]"));
            product(base, "crates/a", &manifest("[\"x\"]"));
            ManifestTomlAdapter.members(&self::root(base)).map(|_| true)
        }
        Question::MembersBelowARoot => {
            root(TOOL);
            product(base, "inner", &manifest("[\"x\"]"));
            ManifestTomlAdapter
                .members(&self::root(&base.join("inner")))
                .map(|_| true)
        }
        Question::HooksFromMember => {
            root(&format!(
                "members = [\"crates/*\"]\n{TOOL}\n[hooks]\npre-push = [\"lint\"]\n"
            ));
            product(base, "crates/a", TOOL);
            let found = ManifestTomlAdapter
                .hooks(&self::root(&base.join("crates/a")))?
                .0;
            Ok(found.len() == 1 && &*found[0].name.0 == "pre-push")
        }
        Question::HooksDefault => {
            root(TOOL);
            let found = ManifestTomlAdapter.hooks(&self::root(base))?.0;
            Ok(found.is_empty())
        }
        Question::HooksUndeclared => {
            root(&format!("[hooks]\npre-commit = [\"release\"]\n{TOOL}"));
            ManifestTomlAdapter.hooks(&self::root(base)).map(|_| true)
        }
        Question::MemberDeclaresHooks => {
            root(&manifest("[\"crates/*\"]"));
            let hooked = format!("[hooks]\npre-commit = [\"lint\"]\n{TOOL}");
            product(base, "crates/a", &hooked);
            ManifestTomlAdapter.members(&self::root(base)).map(|_| true)
        }
        Question::Topmost => {
            root(&manifest("[\"crates/*\"]"));
            product(base, "crates/a", TOOL);
            Ok(
                ManifestTomlAdapter.topmost(&self::root(&base.join("crates/a")))?
                    == self::root(base),
            )
        }
        Question::WriteNew => {
            ManifestTomlAdapter.create(&self::root(base), &minimal())?;
            let written = std::fs::read_to_string(base.join(FILE)).expect("the manifest written");
            // Read back and written again elsewhere, the manifest is the same bytes.
            let read = ManifestTomlAdapter.read(&self::root(base))?;
            let elsewhere = base.join("again");
            std::fs::create_dir_all(&elsewhere).expect("another product");
            ManifestTomlAdapter.create(&self::root(&elsewhere), &read)?;
            let again =
                std::fs::read_to_string(elsewhere.join(FILE)).expect("the manifest written again");
            Ok(written
                == "# The tools asymptote runs, each in a group, a label of your choosing.\n# See `asymptote check --help`.\n\n[hooks]\npre-commit = [\"lint\"]\n"
                && again == written)
        }
        Question::WriteExists => {
            root(TOOL);
            ManifestTomlAdapter
                .create(&self::root(base), &minimal())
                .map(|()| true)
        }
        Question::CapturedUnbound => {
            std::fs::create_dir_all(base.join("languages/go")).expect("a directory");
            root(
                "[[tools]]\nid = \"test-{lang}\"\nrun = [\"t\"]\ngroup = \"check\"\ninputs = [\"langauges/{lang}\"]\n",
            );
            ManifestTomlAdapter.read(&self::root(base)).map(|_| true)
        }
        Question::CapturedNotText => {
            std::fs::create_dir_all(base.join("languages/go/src")).expect("a directory");
            root(
                "[[tools]]\nid = \"test-{lang}\"\nrun = [\"t\"]\ngroup = \"check\"\ninputs = [\"languages/{lang}/src\"]\n",
            );
            let undo = unreadable_entry(base);
            let answer = ManifestTomlAdapter.read(&self::root(base)).map(|_| true);
            undo();
            answer
        }
        Question::Contract
        | Question::Nowhere
        | Question::Shape
        | Question::Unreadable
        | Question::PinnedHere
        | Question::PinnedElsewhere
        | Question::PinnedCaret => Ok(false),
    }
}

/// Makes `languages/go` of `base` an entry a captured input reaches but cannot read: on Unix a directory that cannot be
/// searched, so the metadata of `languages/go/src` is refused; on Windows a sibling whose name is no UTF-8 text (an unpaired
/// surrogate). Answers what undoes it, so the temporary directory can be removed.
#[cfg(unix)]
fn unreadable_entry(base: &std::path::Path) -> impl FnOnce() {
    use std::os::unix::fs::PermissionsExt as _;
    let go = base.join("languages/go");
    std::fs::set_permissions(&go, std::fs::Permissions::from_mode(0o000)).expect("the mode");
    move || {
        drop(std::fs::set_permissions(
            &go,
            std::fs::Permissions::from_mode(0o755),
        ));
    }
}

/// Makes `languages/go` of `base` an entry a captured input reaches but cannot read: on Windows a sibling whose name is no
/// UTF-8 text (an unpaired surrogate). Answers what undoes it.
#[cfg(windows)]
fn unreadable_entry(base: &std::path::Path) -> impl FnOnce() {
    use std::os::windows::ffi::OsStringExt as _;
    std::fs::create_dir_all(
        base.join("languages")
            .join(std::ffi::OsString::from_wide(&[0xD800])),
    )
    .expect("a directory");
    || ()
}

fn root(directory: &std::path::Path) -> ProductRoot {
    ProductRoot(directory.display().to_string().into())
}

fn subject(question: Input) -> Result<Value, Failure> {
    let product = tempfile::tempdir().expect("a temporary directory");
    let path = product.path();
    let answer = match question {
        Question::Members
        | Question::MemberWithoutManifest
        | Question::MemberMissing
        | Question::MemberGlobEmpty
        | Question::MemberDeclaresMembers
        | Question::MembersBelowARoot
        | Question::HooksFromMember
        | Question::HooksDefault
        | Question::HooksUndeclared
        | Question::MemberDeclaresHooks
        | Question::Topmost
        | Question::WriteNew
        | Question::WriteExists
        | Question::CapturedUnbound
        | Question::CapturedNotText => workspace(question, path),
        Question::Contract => {
            std::fs::write(
                path.join(FILE),
                "[[tools]]\nid = \"fmt\"\nrun = [\"fmt\"]\ngroup = \"lint\"\n\n[[tools]]\nid = \"lint\"\nrun = [\"lint\"]\ngroup = \"lint\"\noutput = \"sarif\"\n",
            )
            .expect("the manifest");
            std::fs::create_dir_all(path.join("src")).expect("a directory beneath it");
            let elsewhere = tempfile::tempdir().expect("another temporary directory");
            let blank = tempfile::tempdir().expect("a blank place");
            check(
                &ManifestTomlAdapter,
                &root(path),
                &root(elsewhere.path()),
                &root(blank.path()),
            );
            ManifestTomlAdapter
                .find(&root(&path.join("src")))
                .map(|found| found == root(path))
        }
        Question::Nowhere => ManifestTomlAdapter.find(&root(path)).map(|_| true),
        Question::PinnedHere | Question::PinnedElsewhere | Question::PinnedCaret => {
            let installed = env!("CARGO_PKG_VERSION");
            let mut parts = installed.split('.');
            let pinned = if matches!(question, Question::PinnedCaret) {
                format!(
                    "^{}.{}",
                    parts.next().unwrap_or_default(),
                    parts.next().unwrap_or_default()
                )
            } else if matches!(question, Question::PinnedHere) {
                installed.to_owned()
            } else {
                "9.9.9".to_owned()
            };
            std::fs::write(
                path.join(FILE),
                format!("asymptote = \"{pinned}\"\nmembers = [\"p\"]\n"),
            )
            .expect("the root's manifest");
            std::fs::create_dir_all(path.join("p")).expect("a member");
            std::fs::write(
                path.join("p").join(FILE),
                "[[tools]]\nid = \"lint\"\nrun = [\"lint\"]\ngroup = \"lint\"\n",
            )
            .expect("the member's manifest");
            ManifestTomlAdapter
                .find(&root(&path.join("p")))
                .map(|found| found == root(&path.join("p")))
        }
        Question::Shape => {
            std::fs::write(path.join(FILE), "tools = 1\n").expect("the manifest");
            ManifestTomlAdapter.read(&root(path)).map(|_| true)
        }
        Question::Unreadable => {
            std::fs::create_dir_all(path.join(FILE)).expect("a directory where the manifest is");
            ManifestTomlAdapter.read(&root(path)).map(|_| true)
        }
    };
    if let Err(error) = &answer {
        assert!(matches!(
            error,
            ManifestError::Missing { .. }
                | ManifestError::Shape { .. }
                | ManifestError::Unreadable { .. }
                | ManifestError::Member { .. }
                | ManifestError::NotRoot { .. }
                | ManifestError::Hook { .. }
                | ManifestError::Unwritable { .. }
                | ManifestError::Pinned { .. }
        ));
    }
    answer
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
