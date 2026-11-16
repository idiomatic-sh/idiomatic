//! The script of a hook, and whose a script is.

use super::service::{ownership, script};
use super::types::ScriptText;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifest::types::HookName;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The script of `pre-commit`.
    Script,
    /// Whose that script is.
    Ours,
    /// Whose a script someone else wrote is.
    Foreign,
    /// Whose a script that only mentions the marker is.
    Mentioned,
    /// Whose an empty file is.
    Empty,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_hook_script_is_the_shebang_the_marker_and_the_check_it_runs"),
        word: Word::Succeeds,
        input: || Question::Script,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "#!/bin/sh\n# written by asymptote init; edit [hooks] in asymptote.toml instead\nexec asymptote check --hook pre-commit\n",
        )),
    },
    Case {
        name: CaseName("a_script_asymptote_wrote_is_its_own"),
        word: Word::Succeeds,
        input: || Question::Ours,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Ours")),
    },
    Case {
        name: CaseName("a_script_someone_else_wrote_is_foreign"),
        word: Word::Succeeds,
        input: || Question::Foreign,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Foreign")),
    },
    Case {
        name: CaseName("a_script_that_only_mentions_the_marker_is_foreign"),
        word: Word::Succeeds,
        input: || Question::Mentioned,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Foreign")),
    },
    Case {
        name: CaseName("an_empty_file_is_foreign"),
        word: Word::Succeeds,
        input: || Question::Empty,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Foreign")),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "every part's runner takes a subject that answers a Result, as `run` states it; these rows only succeed"
)]
fn subject(question: Input) -> Result<Value, Failure> {
    let ours = script(&HookName("pre-commit".into()));
    let said = |text: &ScriptText| format!("{:?}", ownership(text));
    Ok(std::borrow::Cow::Owned(match question {
        Question::Script => ours.0.to_string(),
        Question::Ours => said(&ours),
        Question::Foreign => said(&ScriptText("#!/bin/sh\nlint-staged\n".into())),
        Question::Mentioned => said(&ScriptText(
            "#!/bin/sh\necho \"# written by asymptote init; edit [hooks] in asymptote.toml instead\"\n".into(),
        )),
        Question::Empty => said(&ScriptText(Box::default())),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
