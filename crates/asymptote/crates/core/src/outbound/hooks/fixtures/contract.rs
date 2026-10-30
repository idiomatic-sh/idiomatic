//! What every `hooks` adapter must pass: a hook not there is absent, a script installed is read back as written,
//! and one removed is absent again.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::hooks::types::ScriptText;
use crate::domain::manifest::types::{HookName, ProductRoot};
use crate::outbound::hooks::errors::HooksError;
use crate::outbound::hooks::interface::HooksPort;
use crate::outbound::hooks::types::Installed;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// A hook nothing wrote.
    Untouched,
    /// A script installed.
    Installed,
    /// A script installed, then removed.
    Removed,
    /// A script removed that was never there.
    RemovedTwice,
    /// A script installed over another.
    Replaced,
    /// The top level of the repository the product is at the top of.
    Top,
}

/// The steps a row takes before it inspects the hook.
type Input = Question;
/// What `inspect` answers after the question's steps.
type Value = std::borrow::Cow<'static, str>;
/// The adapter's failure, which no row of the contract expects.
type Failure = HooksError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_hook_nothing_wrote_is_absent"),
        word: Word::Succeeds,
        input: || Question::Untouched,
        expect: Expect::Value(std::borrow::Cow::Borrowed("absent")),
    },
    Case {
        name: CaseName("a_script_installed_is_read_back_as_written"),
        word: Word::Succeeds,
        input: || Question::Installed,
        expect: Expect::Value(std::borrow::Cow::Borrowed("#!/bin/sh\nfirst\n")),
    },
    Case {
        name: CaseName("a_script_installed_over_another_replaces_it"),
        word: Word::Succeeds,
        input: || Question::Replaced,
        expect: Expect::Value(std::borrow::Cow::Borrowed("#!/bin/sh\nsecond\n")),
    },
    Case {
        name: CaseName("the_top_level_of_a_repository_is_where_its_product_is_at_the_top"),
        word: Word::Succeeds,
        input: || Question::Top,
        expect: Expect::Value(std::borrow::Cow::Borrowed("here")),
    },
    Case {
        name: CaseName("a_script_removed_is_absent"),
        word: Word::Succeeds,
        input: || Question::Removed,
        expect: Expect::Value(std::borrow::Cow::Borrowed("absent")),
    },
    Case {
        name: CaseName("removing_a_script_that_was_never_there_is_no_failure"),
        word: Word::Succeeds,
        input: || Question::RemovedTwice,
        expect: Expect::Value(std::borrow::Cow::Borrowed("absent")),
    },
];

/// Runs the contract's rows against one implementation, over the product at `root`, which is in a repository.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl HooksPort, root: &ProductRoot) {
    let name = HookName("pre-commit".into());
    let script = |body: &str| ScriptText(format!("#!/bin/sh\n{body}\n").into());
    let subject = |question: Input| -> Result<Value, Failure> {
        // Every row starts from no hook, whatever the row before it left.
        adapter.remove(root, &name)?;
        if matches!(question, Question::Top) {
            let top = adapter.top(root)?;
            let canonical = |place: &ProductRoot| std::fs::canonicalize(&*place.0).ok();
            let here =
                top == *root || (canonical(&top).is_some() && canonical(&top) == canonical(root));
            return Ok(std::borrow::Cow::Borrowed(if here {
                "here"
            } else {
                "elsewhere"
            }));
        }
        match question {
            Question::Top | Question::Untouched => {}
            Question::Installed => adapter.install(root, &name, &script("first"))?,
            Question::Replaced => {
                adapter.install(root, &name, &script("first"))?;
                adapter.install(root, &name, &script("second"))?;
            }
            Question::Removed => {
                adapter.install(root, &name, &script("first"))?;
                adapter.remove(root, &name)?;
            }
            Question::RemovedTwice => {
                adapter.remove(root, &name)?;
                adapter.remove(root, &name)?;
            }
        }
        Ok(std::borrow::Cow::Owned(
            match adapter.inspect(root, &name)? {
                Installed::Absent => "absent".to_owned(),
                Installed::Script { text, .. } => text.0.to_string(),
            },
        ))
    };
    run(CASES, Word::Succeeds, subject);
}
