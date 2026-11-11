//! A project submission answers exactly the roots it names, and refuses by name the ones it did not declare. These are the runtime's
//! own contracts on the crate-private root evaluation: a caller of `evaluate_occurrences` cannot assemble such roots.
use super::super::super::fixtures::Entry;
use super::super::super::fixtures::submission::{Root, foreign_phase_outcome, submit};
use super::fixtures::{Prepared, Scopes, root, scopes};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::borrow::Cow;

/// A root the program never declared: its scope and frontiers come from rule 0, the task names rule 3.
const EXTRA: Root = Root {
    declared: 0,
    named: 3,
};
/// A root the session did not declare though it is in range: rule 0's scope and frontiers, named rule 1.
const UNDECLARED: Root = Root {
    declared: 0,
    named: 1,
};

/// What a row submits or opens.
#[derive(Clone, Copy)]
enum Submission {
    /// These roots through the real project phase or stage of a request over three rules; the guard
    /// rejects this rule, if any.
    Roots {
        entry: Entry,
        roots: &'static [Root],
        guard: Option<RuleIndex>,
    },
    /// A root evaluated in a phase over a scope another phase opened.
    ForeignScope,
    /// Scopes of declared rule 2 and undeclared rule 3, and, in a phase, frontiers of declared
    /// rule 0 and undeclared rule 3.
    Scopes(Entry),
}

/// What a row answered.
#[derive(Debug, PartialEq)]
enum Answered {
    /// Each submitted root's refusal (none when it was evaluated), the rules the guard was asked
    /// about, and the batch's refusal.
    Submitted {
        outcomes: Cow<'static, [Option<OperatingError>]>,
        guarded: Cow<'static, [RuleIndex]>,
        refused: Option<OperatingError>,
    },
    /// The foreign scope's refusal.
    Foreign(Option<OperatingError>),
    Scopes(Scopes),
}

type Input = Submission;
type Value = Answered;
type Failure = OperatingError;

const fn submitted(
    outcomes: &'static [Option<OperatingError>],
    guarded: &'static [RuleIndex],
    refused: Option<OperatingError>,
) -> Answered {
    Answered::Submitted {
        outcomes: Cow::Borrowed(outcomes),
        guarded: Cow::Borrowed(guarded),
        refused,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_submitted_root_gets_no_outcome"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[] },
            guard: None,
        },
        expect: Expect::Value(submitted(&[], &[], None)),
    },
    Case {
        name: CaseName("one_submitted_root_gets_one_outcome"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0)] },
            guard: None,
        },
        expect: Expect::Value(submitted(&[None], &[RuleIndex(0)], None)),
    },
    Case {
        name: CaseName("two_submitted_roots_get_two_outcomes"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0), root(1)] },
            guard: None,
        },
        expect: Expect::Value(submitted(&[None, None], &[RuleIndex(0)], None)),
    },
    Case {
        name: CaseName("every_submitted_root_gets_one_outcome"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0), root(1), root(2)] },
            guard: None,
        },
        expect: Expect::Value(submitted(&[None, None, None], &[RuleIndex(0)], None)),
    },
    Case {
        name: CaseName(
            "an_omitted_declared_root_gets_no_outcome_and_no_guard_and_every_submitted_root_gets_one",
        ),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0), root(2)] },
            guard: None,
        },
        expect: Expect::Value(submitted(&[None, None], &[RuleIndex(0)], None)),
    },
    Case {
        name: CaseName("an_extra_root_is_refused_by_name_without_displacing_the_declared_roots"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0), EXTRA, root(2)] },
            guard: None,
        },
        expect: Expect::Value(submitted(
            &[None, Some(OperatingError::NotRun), None],
            &[RuleIndex(0), RuleIndex(3)],
            None,
        )),
    },
    Case {
        name: CaseName("a_guard_refusal_at_an_extra_root_completes_only_the_roots_before_it"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0), EXTRA, root(2)] },
            guard: Some(RuleIndex(3)),
        },
        expect: Expect::Value(submitted(
            &[None],
            &[RuleIndex(0), RuleIndex(3)],
            Some(OperatingError::WrongBinding),
        )),
    },
    Case {
        name: CaseName("an_in_range_root_the_session_did_not_declare_is_refused_by_name"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0), UNDECLARED, root(2)] },
            guard: None,
        },
        expect: Expect::Value(submitted(
            &[None, Some(OperatingError::NotRead), None],
            &[RuleIndex(0), RuleIndex(1)],
            None,
        )),
    },
    Case {
        name: CaseName("an_undeclared_in_range_root_of_a_stage_is_refused_by_name"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Stage,
            roots: const { &[root(0), UNDECLARED, root(2)] },
            guard: None,
        },
        expect: Expect::Value(submitted(
            &[None, Some(OperatingError::NotRead), None],
            &[RuleIndex(0), RuleIndex(1)],
            None,
        )),
    },
    Case {
        name: CaseName(
            "a_guard_refusal_at_an_undeclared_in_range_root_completes_only_the_roots_before_it",
        ),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0), UNDECLARED, root(2)] },
            guard: Some(RuleIndex(1)),
        },
        expect: Expect::Value(submitted(
            &[None],
            &[RuleIndex(0), RuleIndex(1)],
            Some(OperatingError::WrongBinding),
        )),
    },
    Case {
        name: CaseName("a_duplicated_root_keeps_one_outcome_per_submission"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0), root(0)] },
            guard: None,
        },
        expect: Expect::Value(submitted(&[None, None], &[RuleIndex(0)], None)),
    },
    Case {
        name: CaseName("a_duplicated_undeclared_root_is_refused_every_time"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Phase,
            roots: const { &[root(0), UNDECLARED, UNDECLARED] },
            guard: None,
        },
        expect: Expect::Value(submitted(
            &[
                None,
                Some(OperatingError::NotRead),
                Some(OperatingError::NotRead),
            ],
            &[RuleIndex(0), RuleIndex(1), RuleIndex(1)],
            None,
        )),
    },
    Case {
        name: CaseName("a_stage_answers_every_submitted_root"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Stage,
            roots: const { &[root(0), root(1), root(2)] },
            guard: None,
        },
        expect: Expect::Value(submitted(&[None, None, None], &[RuleIndex(0)], None)),
    },
    Case {
        name: CaseName("a_stage_gives_an_omitted_root_no_outcome_and_no_guard"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Stage,
            roots: const { &[root(0), root(2)] },
            guard: None,
        },
        expect: Expect::Value(submitted(&[None, None], &[RuleIndex(0)], None)),
    },
    Case {
        name: CaseName("every_holder_of_roots_refuses_an_incomplete_extra_or_undeclared_root"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Stage,
            roots: const { &[root(0), EXTRA, root(2)] },
            guard: None,
        },
        expect: Expect::Value(submitted(
            &[None, Some(OperatingError::NotRun), None],
            &[RuleIndex(0), RuleIndex(3)],
            None,
        )),
    },
    Case {
        name: CaseName("a_stage_guard_refusal_at_an_extra_root_completes_only_the_roots_before_it"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Stage,
            roots: const { &[root(0), EXTRA, root(2)] },
            guard: Some(RuleIndex(3)),
        },
        expect: Expect::Value(submitted(
            &[None],
            &[RuleIndex(0), RuleIndex(3)],
            Some(OperatingError::WrongBinding),
        )),
    },
    Case {
        name: CaseName("a_stage_keeps_one_outcome_per_submission_of_a_duplicated_root"),
        word: Word::Succeeds,
        input: || Submission::Roots {
            entry: Entry::Stage,
            roots: const { &[root(0), root(0)] },
            guard: None,
        },
        expect: Expect::Value(submitted(&[None, None], &[RuleIndex(0)], None)),
    },
    Case {
        name: CaseName("a_scope_belongs_to_its_own_phase"),
        word: Word::Succeeds,
        input: || Submission::ForeignScope,
        expect: Expect::Value(Answered::Foreign(Some(OperatingError::NotRead))),
    },
    Case {
        name: CaseName("frontiers_and_scopes_of_an_undeclared_rule_are_refused"),
        word: Word::Succeeds,
        input: || Submission::Scopes(Entry::Phase),
        expect: Expect::Value(Answered::Scopes(Scopes {
            declared: None,
            undeclared: Some(OperatingError::NotRun),
            frontiers: Some((Prepared::Ready, Prepared::Metadata(OperatingError::NotRead))),
        })),
    },
    Case {
        name: CaseName("a_stage_opens_scopes_only_for_declared_rules"),
        word: Word::Succeeds,
        input: || Submission::Scopes(Entry::Stage),
        expect: Expect::Value(Answered::Scopes(Scopes {
            declared: None,
            undeclared: Some(OperatingError::NotRun),
            frontiers: None,
        })),
    },
];

/// Submits the roots, evaluates the foreign scope, or opens the scopes.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Submission::Roots {
            entry,
            roots,
            guard,
        } => {
            let submitted = submit(entry, 3, roots, guard);
            Ok(Answered::Submitted {
                outcomes: submitted.outcomes.into_iter().map(Result::err).collect(),
                guarded: Cow::Owned(submitted.guarded),
                refused: submitted.refused,
            })
        }
        Submission::ForeignScope => Ok(Answered::Foreign(foreign_phase_outcome().err())),
        Submission::Scopes(entry) => scopes(entry).map(Answered::Scopes),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
