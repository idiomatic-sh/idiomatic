//! What the manifest's schemas accept and refuse.

use super::manifest;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The file's text.
type Input = &'static str;
/// The tools read, `id:output:inputs`, joined.
type Value = std::borrow::Cow<'static, str>;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_sarif_tool_and_an_exit_status_tool_are_read"),
        word: Word::Succeeds,
        input: || "[[tools]]\nid = \"lint\"\nrun = [\"lint\"]\ngroup = \"lint\"\noutput = \"sarif\"\ninputs = [\".\", \"../shared\"]\n\n[[tools]]\nid = \"fmt\"\nrun = [\"fmt\"]\ngroup = \"lint\"\nfix = [\"fmt\", \"--write\"]\n",
        expect: Expect::Value(std::borrow::Cow::Borrowed("lint:Sarif:2 fmt:ExitStatus:1")),
    },
    Case {
        name: CaseName("a_perf_tool_reading_bencher_metric_format_is_read"),
        word: Word::Succeeds,
        input: || "[[tools]]\nid = \"bench\"\nrun = [\"bench\"]\ngroup = \"perf\"\noutput = \"bmf\"\n",
        expect: Expect::Value(std::borrow::Cow::Borrowed("bench:Bmf:perf:1")),
    },
    Case {
        name: CaseName("a_test_tool_reading_its_exit_status_is_read"),
        word: Word::Succeeds,
        input: || "[[tools]]\nid = \"t\"\nrun = [\"t\"]\ngroup = \"test\"\n",
        expect: Expect::Value(std::borrow::Cow::Borrowed("t:ExitStatus:test:1")),
    },
    Case {
        name: CaseName("a_workspace_root_with_members_and_no_tool_is_read"),
        word: Word::Succeeds,
        input: || "members = [\"crates/*\", \"tools/one\"]\n",
        expect: Expect::Value(std::borrow::Cow::Borrowed("members=crates/*,tools/one")),
    },
    Case {
        name: CaseName("a_tool_without_a_group_is_refused"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"a\"\nrun = [\"a\"]\n",
        expect: Expect::Refusal(|actual| {
            if actual == "tool `a`: `group` is required: a label of your choosing, such as `lint`" {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_timeout_of_zero_seconds_is_refused"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"a\"\nrun = [\"a\"]\ngroup = \"lint\"\ntimeout-seconds = 0\n",
        expect: Expect::Refusal(|actual| {
            if actual == "tool `a`: `timeout-seconds` is a positive number of seconds, not 0" {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_member_that_climbs_out_is_refused"),
        word: Word::Refuses,
        input: || "members = [\"../other\"]\n",
        expect: Expect::Refusal(|actual| {
            if actual.starts_with("member `../other` is not a relative glob") {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("any_group_label_and_a_hooks_table_are_read"),
        word: Word::Succeeds,
        input: || "[[tools]]\nid = \"r\"\nrun = [\"r\"]\ngroup = \"release\"\n\n[hooks]\npre-commit = [\"lint\", \"test\"]\npre-push = [\"perf\"]\n",
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "r:ExitStatus:release:1 hooks=pre-commit:[Group(\"lint\"), Group(\"test\")],pre-push:[Group(\"perf\")]",
        )),
    },
    Case {
        name: CaseName("reuse_is_no_key_a_tool_that_reads_a_git_range_is_stamped_with_its_commits"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"c\"\nrun = [\"c\"]\ngroup = \"lint\"\nreuse = false\n",
        expect: Expect::Refusal(|actual| {
            if actual.starts_with("unknown field `reuse`") {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_hooks_table_alone_is_a_manifest"),
        word: Word::Succeeds,
        input: || "[hooks]\npre-commit = [\"lint\"]\n",
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "hooks=pre-commit:[Group(\"lint\")]",
        )),
    },
    Case {
        name: CaseName("a_hook_git_does_not_run_is_refused_with_the_hooks"),
        word: Word::Refuses,
        input: || "[hooks]\npre-lunch = [\"lint\"]\n",
        expect: Expect::Refusal(|actual| {
            if actual.starts_with("hook `pre-lunch` is not a git hook: use one of applypatch-msg, ")
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_hook_that_lists_no_group_is_refused"),
        word: Word::Refuses,
        input: || "[hooks]\npre-commit = []\n",
        expect: Expect::Refusal(|actual| {
            if actual
                == "hook `pre-commit` lists no group: list a group, or remove the hook from `[hooks]` to install none"
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("no_tool_is_refused"),
        word: Word::Refuses,
        input: || "tools = []\n",
        expect: Expect::Refusal(|actual| {
            if actual == "it names no tool" {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("two_tools_of_one_id_are_refused"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"a\"\nrun = [\"a\"]\ngroup = \"lint\"\n\n[[tools]]\nid = \"a\"\nrun = [\"a\"]\ngroup = \"lint\"\n",
        expect: Expect::Refusal(|actual| {
            if actual == "two tools are named `a`" {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_output_asymptote_does_not_read_is_refused"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"a\"\nrun = [\"a\"]\ngroup = \"lint\"\noutput = \"ruff\"\n",
        expect: Expect::Refusal(|actual| {
            if actual == "tool `a`: `output` is `sarif`, `bmf` or absent, not `ruff`" {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("v1_keys_are_refused"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"a\"\nrun = [\"a\"]\ngroup = \"lint\"\nparser = \"sarif\"\n",
        expect: Expect::Refusal(|actual| {
            if actual
                == "unknown field `parser`, expected one of `id`, `run`, `fix`, `group`, `output`, `timeout-seconds`, `inputs`"
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_empty_command_is_refused"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"a\"\nrun = []\ngroup = \"lint\"\n",
        expect: Expect::Refusal(|actual| {
            if actual == "tool `a` runs an empty command" {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_capture_in_inputs_is_read_as_written_and_expanded_on_disk"),
        word: Word::Succeeds,
        input: || "[[tools]]\nid = \"t\"\nrun = [\"t\", \"{lang}\"]\ngroup = \"check\"\ninputs = [\"languages/{lang}\", \"crates\"]\n",
        expect: Expect::Value(std::borrow::Cow::Borrowed("t:ExitStatus:check:2")),
    },
    Case {
        name: CaseName("a_capture_in_run_that_no_input_writes_is_refused"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"t\"\nrun = [\"t\", \"-p\", \"x-{lang}\"]\ngroup = \"check\"\ninputs = [\"languages\"]\n",
        expect: Expect::Refusal(|actual| {
            if actual
                == "tool `t`: `run` uses `{lang}`, which no input captures: write it in one of its `inputs`, as in `<directory>/{lang}`"
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_capture_in_fix_that_no_input_writes_is_refused"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"t\"\nrun = [\"t\"]\nfix = [\"t\", \"{dir}\"]\ngroup = \"lint\"\n",
        expect: Expect::Refusal(|actual| {
            if actual.starts_with("tool `t`: `fix` uses `{dir}`, which no input captures") {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("braces_around_no_name_are_text_in_run"),
        word: Word::Succeeds,
        input: || "[[tools]]\nid = \"g\"\nrun = [\"grep\", \"-E\", \"^={7}$\"]\ngroup = \"lint\"\n",
        expect: Expect::Value(std::borrow::Cow::Borrowed("g:ExitStatus:1")),
    },
    Case {
        name: CaseName("an_empty_id_is_refused"),
        word: Word::Refuses,
        input: || "[[tools]]\nid = \"\"\nrun = [\"a\"]\ngroup = \"lint\"\n",
        expect: Expect::Refusal(|actual| {
            if actual == "a tool has an empty id" {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn told(text: Input) -> Result<String, Failure> {
    let parsed = manifest(text)?;
    let mut parts: Vec<String> = parsed
        .tools
        .iter()
        .map(|tool| {
            let group = if &*tool.group.0 == "lint" {
                String::new()
            } else {
                format!(":{}", tool.group.0)
            };
            format!(
                "{}:{:?}{group}:{}",
                tool.id.0,
                tool.output,
                tool.inputs.len()
            )
        })
        .collect();
    if !parsed.hooks.is_empty() {
        let hooks: Vec<String> = parsed
            .hooks
            .iter()
            .map(|hook| format!("{}:{:?}", hook.name.0, hook.groups))
            .collect();
        parts.push(format!("hooks={}", hooks.join(",")));
    }
    if !parsed.members.is_empty() {
        let members: Vec<&str> = parsed.members.iter().map(|member| &*member.0).collect();
        parts.push(format!("members={}", members.join(",")));
    }
    Ok(parts.join(" "))
}

fn subject(input: Input) -> Result<Value, Failure> {
    told(input).map(std::borrow::Cow::Owned)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
