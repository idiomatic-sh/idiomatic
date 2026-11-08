//! The `storing` service's cases: the repository listed in the order of its paths beside what the last run kept, each port's
//! failure answered as its own, and what a store kept used again only under the plan it was written with.
use super::errors::StoringError;
use super::service::Storing;
use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use crate::domain::render::types::Answering;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use crate::outbound::scheduler::types::WorkerCount;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::{CorpusFile, ExecutablePermission, FileBytes};
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::Stored;
use crate::domain::storing::types::Decision;
use std::sync::Arc;

/// Which port of the listing a row makes fail.
#[derive(Clone, Copy, Debug)]
enum Failing {
    /// None: the listing answers.
    Nothing,
    /// The corpus refuses to list.
    Corpus,
    /// The store cannot be read.
    Store,
    /// The memory budget cannot be read.
    Memory,
    /// The machine has one byte to spare.
    Budget,
    /// The scheduler refuses to start.
    Scheduler,
}

/// The order the listing answers its files in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Order {
    /// Each path after the one before it.
    ByPath,
    /// Some other order.
    Other,
}

/// The plan a warm run compiles the same rule text to, beside the plan the store was written under.
#[derive(Clone, Copy, Debug)]
enum PlanAtRead {
    /// The plan is the one the store was written under.
    Same,
    /// A rewrite toggled: the same text lowers to a plan with one more step, so the frontier slots hold other steps.
    Rewritten,
}

/// What a row of the cases does.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Lists the repository, with the port named failing.
    Listing(Failing),
    /// Writes the store under one plan, then decides a warm run over the same rule id and text under the plan named.
    Rerun(PlanAtRead),
}

/// What a warm run does with what the store kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kept {
    /// What the store kept is used again.
    Reused,
    /// Every file is read again and every rule run again.
    Rebuilt,
}

/// What a row answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// How many files were listed, and in which order.
    Listed(usize, Order),
    /// What the warm run did with the store.
    Rerun(Kept),
}

type Input = Scenario;
type Value = Answer;
type Failure = StoringError;

/// A file of the fake corpus at `path`.
fn file(path: &str) -> CorpusFile {
    CorpusFile {
        executable: ExecutablePermission::Unknown,
        path: FilePath(path.into()),
        bytes: FileBytes(b"text".to_vec()),
    }
}

/// A rule over the same text under every plan.
fn rule() -> RuleSpec {
    RuleSpec {
        id: RuleId("same-rule".into()),
        query: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(
            "the same text".into(),
        )),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters("".into()),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

/// The plan the store is written under, and the plan a warm run compiles the same text to.
fn plan(at: PlanAtRead) -> Plan {
    match at {
        PlanAtRead::Same => QueryLanguageFake::selecting().plan,
        PlanAtRead::Rewritten => QueryLanguageFake::selecting_twice().plan,
    }
}

/// The storing service over fakes, with the port named failing.
fn storing(failing: Failing) -> Storing {
    Storing::new(
        Arc::new(CorpusFake {
            files: vec![file("b.txt"), file("a.txt")],
            fails: matches!(failing, Failing::Corpus),
            ..CorpusFake::default()
        }),
        Arc::new(DigestFake),
        Arc::new(LintStoreFake {
            unreadable: matches!(failing, Failing::Store),
            ..LintStoreFake::default()
        }),
        Arc::new(MemoryFake {
            fails: matches!(failing, Failing::Memory),
            available: if matches!(failing, Failing::Budget) { 1 } else { 1 << 30 },
            ..MemoryFake::default()
        }),
        Arc::new(SchedulerFake {
            fails: matches!(failing, Failing::Scheduler),
        }),
        WorkerCount(1),
    )
}

/// Writes the store's rules under the first plan, then decides a warm run under the plan `at`.
fn rerun(at: PlanAtRead) -> Result<Value, Failure> {
    let storing = storing(Failing::Nothing);
    let rules = [rule()];
    let identify = |plan: Plan| storing.stored_rules(&rules, &[plan], &[], Digest([0; 16]), None);
    let mut listing = storing.listing(&Answering::Findings)?;
    listing.stored = Stored {
        rules: identify(plan(PlanAtRead::Same)),
        ..Stored::default()
    };
    let decided = Storing::decision(&Answering::Findings, &listing, &identify(plan(at)));
    Ok(Answer::Rerun(match decided.decision {
        Decision::Whole => Kept::Rebuilt,
        Decision::Again | Decision::Check(_) | Decision::Cross => Kept::Reused,
    }))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let failing = match scenario {
        Scenario::Listing(failing) => failing,
        Scenario::Rerun(at) => return rerun(at),
    };
    let listing = storing(failing).listing(&Answering::Findings)?;
    let paths: Vec<&FilePath> = listing.files.iter().map(input_path).collect();
    let order = if paths.windows(2).all(|pair| pair[0] < pair[1]) {
        Order::ByPath
    } else {
        Order::Other
    };
    Ok(Answer::Listed(paths.len(), order))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_files_are_listed_in_the_order_of_their_paths"),
        word: Word::Succeeds,
        input: || Scenario::Listing(Failing::Nothing),
        expect: Expect::Value(Answer::Listed(2, Order::ByPath)),
    },
    Case {
        name: CaseName("a_corpus_that_cannot_list_fails_the_listing"),
        word: Word::Fails,
        input: || Scenario::Listing(Failing::Corpus),
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, StoringError::Corpus(_))))),
    },
    Case {
        name: CaseName("a_store_that_cannot_be_read_fails_the_listing"),
        word: Word::Fails,
        input: || Scenario::Listing(Failing::Store),
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, StoringError::Store(_))))),
    },
    Case {
        name: CaseName("a_budget_that_cannot_be_read_fails_the_listing"),
        word: Word::Fails,
        input: || Scenario::Listing(Failing::Memory),
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, StoringError::Memory(_))))),
    },
    Case {
        name: CaseName("a_repository_larger_than_the_budget_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Listing(Failing::Budget),
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, StoringError::Budget(_))))),
    },
    Case {
        name: CaseName("a_scheduler_that_cannot_start_fails_the_listing"),
        word: Word::Fails,
        input: || Scenario::Listing(Failing::Scheduler),
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, StoringError::Scheduler(_))))
        }),
    },
    Case {
        name: CaseName("a_store_written_under_another_plan_of_the_same_rule_text_is_rebuilt"),
        word: Word::Succeeds,
        input: || Scenario::Rerun(PlanAtRead::Rewritten),
        expect: Expect::Value(Answer::Rerun(Kept::Rebuilt)),
    },
    Case {
        name: CaseName("a_store_written_under_the_same_plan_is_used_again"),
        word: Word::Succeeds,
        input: || Scenario::Rerun(PlanAtRead::Same),
        expect: Expect::Value(Answer::Rerun(Kept::Reused)),
    },
];

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
