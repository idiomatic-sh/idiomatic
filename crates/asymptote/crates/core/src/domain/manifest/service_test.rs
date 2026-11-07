//! The groups a check selects, and the hooks that map to them.

use super::service::{groups, hook, hooked, mapped, pinned, reach, repeated, selects};
use super::types::{
    Argv, Fix, Group, Groups, Hook, HookName, HookWord, Output, ProductRoot, Running, Seconds,
    Tool, Version,
};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::violations::types::ToolId;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// No word selects every tool.
    NoWords,
    /// Two words select exactly their groups.
    Two,
    /// A repeated word selects its group once.
    Repeated,
    /// A selection runs a tool of a group it names.
    Runs,
    /// A selection leaves a tool of a group it does not name.
    Leaves,
    /// A hook git runs is known.
    KnownHook,
    /// A hook git does not run is not.
    UnknownHook,
    /// A mapped hook runs its groups.
    Hooked,
    /// The hooks mapped are listed.
    Mapped,
    /// A hook that is not mapped.
    Unmapped,
    /// Two tools of one id.
    Twice,
    /// Exact, caret, partial and zero-major pins against 1.4.2.
    Pins,
    /// Pins with and without a pre-release against releases and pre-releases, each pin with an installed version.
    PreReleases,
    /// A root that names no members reaches itself alone.
    Alone,
    /// A workspace root with tools of its own reaches itself, then each member by its path.
    Members,
    /// A workspace root with no tools of its own reaches each member alone.
    MembersOnly,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_root_with_no_members_reaches_itself_alone"),
        word: Word::Succeeds,
        input: || Question::Alone,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Product .=/w")),
    },
    Case {
        name: CaseName("a_workspace_root_with_tools_reaches_itself_then_each_member"),
        word: Word::Succeeds,
        input: || Question::Members,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Workspace .=/w crates/a=/w/crates/a crates/b=/w/crates/b",
        )),
    },
    Case {
        name: CaseName("a_workspace_root_with_no_tools_reaches_each_member_alone"),
        word: Word::Succeeds,
        input: || Question::MembersOnly,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Workspace crates/a=/w/crates/a crates/b=/w/crates/b",
        )),
    },
    Case {
        name: CaseName("no_words_select_every_tool"),
        word: Word::Succeeds,
        input: || Question::NoWords,
        expect: Expect::Value(std::borrow::Cow::Borrowed("All")),
    },
    Case {
        name: CaseName("two_words_select_their_groups"),
        word: Word::Succeeds,
        input: || Question::Two,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Only([Group(\"lint\"), Group(\"test\")])",
        )),
    },
    Case {
        name: CaseName("a_repeated_word_selects_its_group_once"),
        word: Word::Succeeds,
        input: || Question::Repeated,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Only([Group(\"perf\")])")),
    },
    Case {
        name: CaseName("a_selection_runs_the_tools_of_its_groups"),
        word: Word::Succeeds,
        input: || Question::Runs,
        expect: Expect::Value(std::borrow::Cow::Borrowed("true")),
    },
    Case {
        name: CaseName("a_selection_leaves_the_tools_of_other_groups"),
        word: Word::Succeeds,
        input: || Question::Leaves,
        expect: Expect::Value(std::borrow::Cow::Borrowed("false")),
    },
    Case {
        name: CaseName("a_hook_git_runs_is_known"),
        word: Word::Succeeds,
        input: || Question::KnownHook,
        expect: Expect::Value(std::borrow::Cow::Borrowed("true")),
    },
    Case {
        name: CaseName("a_hook_git_does_not_run_is_not_known"),
        word: Word::Succeeds,
        input: || Question::UnknownHook,
        expect: Expect::Value(std::borrow::Cow::Borrowed("false")),
    },
    Case {
        name: CaseName("a_mapped_hook_runs_the_groups_it_maps"),
        word: Word::Succeeds,
        input: || Question::Hooked,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Only([Group(\"lint\"), Group(\"test\")])",
        )),
    },
    Case {
        name: CaseName("the_hooks_mapped_are_listed"),
        word: Word::Succeeds,
        input: || Question::Mapped,
        expect: Expect::Value(std::borrow::Cow::Borrowed("pre-commit, pre-push")),
    },
    Case {
        name: CaseName("a_hook_that_is_not_mapped_is_refused_with_the_hooks_mapped"),
        word: Word::Refuses,
        input: || Question::Unmapped,
        expect: Expect::Refusal(|actual| {
            if actual == "pre-commit, pre-push" {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_first_id_two_tools_share_is_found"),
        word: Word::Succeeds,
        input: || Question::Twice,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Some(ToolId(\"t/a\")) None")),
    },
    Case {
        name: CaseName(
            "a_pre_release_pin_is_not_met_by_its_release_nor_a_release_pin_by_a_pre_release",
        ),
        word: Word::Succeeds,
        input: || Question::PreReleases,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "0.1.0-rc1@0.1.0:Unmet 0.1.0@0.1.0-rc1:Unmet 0.1.0-rc1@0.1.0-rc1:Met \
             0.1.0-rc1@0.1.0-rc2:Unmet ^0.1.0-rc1@0.1.0:Met ^0.1.0-rc1@0.1.0-rc2:Met \
             ^0.1.0@0.1.0-rc1:Unmet 0.1.0-rc1@0.1.0-rc.2:Unmet 0.1.0@0.1.0:Met \
             0.1.0@0.1.0-rc.2:Unmet 0.1.0-rc.2@0.1.0:Unmet 0.1.0-rc.2@0.1.0-rc.2:Met \
             ^0.1.0-rc.1@0.1.0:Met ^0.1.0-rc.1@0.1.0-rc.2:Met ^0.1.0-rc.10@0.1.0-rc.2:Unmet \
             ^0.1.0@0.1.0-rc.2:Unmet ^0.1@0.1.0-rc.2:Unmet ^0.1-rc.1@0.1.0:Unmet \
             1.0.0-@1.0.0:Unmet 1.0.0@1.0.0-:Unmet",
        )),
    },
    Case {
        name: CaseName("a_pin_is_met_exactly_or_by_its_caret_range"),
        word: Word::Succeeds,
        input: || Question::Pins,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Met Unmet Met Met Unmet Unmet Unmet",
        )),
    },
];

/// A tool of id `id`, in the `check` group.
fn tool(id: &str) -> Tool {
    Tool {
        id: ToolId(id.into()),
        run: Argv(Vec::new()),
        fix: Fix::Absent,
        output: Output::ExitStatus,
        group: group("check"),
        timeout: Seconds(60),
        inputs: Vec::new(),
    }
}

/// A manifest mapping `pre-commit` to lint and test, and `pre-push` to perf.
fn mapping() -> Vec<Hook> {
    vec![
        Hook {
            name: HookName("pre-commit".into()),
            groups: vec![group("lint"), group("test")],
        },
        Hook {
            name: HookName("pre-push".into()),
            groups: vec![group("perf")],
        },
    ]
}

fn group(word: &str) -> Group {
    Group(word.into())
}

fn subject(question: Input) -> Result<Value, Failure> {
    let said = |selected: Groups| format!("{selected:?}");
    let reached = |members: &[&str], tools: &[Tool]| {
        let root = ProductRoot("/w".into());
        let members = members
            .iter()
            .map(|member| ProductRoot(format!("/w/{member}").into()))
            .collect();
        let reach = reach(&root, members, tools, |root| root.0.to_string());
        std::iter::once(format!("{:?}", reach.layout))
            .chain(
                reach
                    .products
                    .iter()
                    .map(|each| format!("{}={}", each.path.0, each.service)),
            )
            .collect::<Vec<_>>()
            .join(" ")
    };
    let named =
        |selected: &[&str]| groups(&selected.iter().map(|word| group(word)).collect::<Vec<_>>());
    Ok(std::borrow::Cow::Owned(match question {
        Question::NoWords => said(named(&[])),
        Question::Alone => reached(&[], &[tool("t/a")]),
        Question::Members => reached(&["crates/a", "crates/b"], &[tool("t/a")]),
        Question::MembersOnly => reached(&["crates/a", "crates/b"], &[]),
        Question::Two => said(named(&["lint", "test"])),
        Question::Repeated => said(named(&["perf", "perf"])),
        Question::Runs => format!(
            "{:?}",
            selects(&Groups::Only(vec![group("perf")]), &group("perf")) == Running::Runs
        ),
        Question::Leaves => format!(
            "{:?}",
            selects(&Groups::Only(vec![group("perf")]), &group("lint")) == Running::Runs
        ),
        Question::KnownHook => hook(&HookWord("pre-commit".into())).is_some().to_string(),
        Question::UnknownHook => hook(&HookWord("pre-lunch".into())).is_some().to_string(),
        Question::Hooked => format!(
            "{:?}",
            hooked(&mapping(), &HookWord("pre-commit".into())).ok_or_else(String::new)?
        ),
        Question::Mapped => mapped(&mapping()).0.to_string(),
        Question::Twice => format!(
            "{:?} {:?}",
            repeated(&[tool("t/a"), tool("t/b"), tool("t/a")]),
            repeated(&[tool("t/a"), tool("t/b")])
        ),
        Question::PreReleases => [
            ("0.1.0-rc1", "0.1.0"),
            ("0.1.0", "0.1.0-rc1"),
            ("0.1.0-rc1", "0.1.0-rc1"),
            ("0.1.0-rc1", "0.1.0-rc2"),
            ("^0.1.0-rc1", "0.1.0"),
            ("^0.1.0-rc1", "0.1.0-rc2"),
            ("^0.1.0", "0.1.0-rc1"),
            ("0.1.0-rc1", "0.1.0-rc.2"),
            ("0.1.0", "0.1.0"),
            ("0.1.0", "0.1.0-rc.2"),
            ("0.1.0-rc.2", "0.1.0"),
            ("0.1.0-rc.2", "0.1.0-rc.2"),
            ("^0.1.0-rc.1", "0.1.0"),
            ("^0.1.0-rc.1", "0.1.0-rc.2"),
            ("^0.1.0-rc.10", "0.1.0-rc.2"),
            ("^0.1.0", "0.1.0-rc.2"),
            ("^0.1", "0.1.0-rc.2"),
            ("^0.1-rc.1", "0.1.0"),
            ("1.0.0-", "1.0.0"),
            ("1.0.0", "1.0.0-"),
        ]
        .iter()
        .map(|(pin, installed)| {
            format!(
                "{pin}@{installed}:{:?}",
                pinned(&Version((*pin).into()), &Version((*installed).into()))
            )
        })
        .collect::<Vec<_>>()
        .join(" "),
        Question::Pins => [
            "1.4.2", "1.4.3", "^1.3.0", "1", "^1.5.0", "^0.4.2", "nonsense",
        ]
        .iter()
        .map(|pin| {
            format!(
                "{:?}",
                pinned(&Version((*pin).into()), &Version("1.4.2".into()))
            )
        })
        .collect::<Vec<_>>()
        .join(" "),
        Question::Unmapped => {
            // Only the refusal is the answer: a hook that maps is a failure of the row.
            if hooked(&mapping(), &HookWord("post-merge".into())).is_none() {
                return Err(mapped(&mapping()).0.to_string());
            }
            String::new()
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
