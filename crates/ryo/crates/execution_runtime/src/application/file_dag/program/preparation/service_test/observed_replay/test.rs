//! Warm replay closure is observed through the actual prepared facade, not a copied graph algorithm.
use crate::cases::{Case, CaseName, Expect, Word, run};

use super::fixtures::{fixture, selection};
use crate::application::file_dag::program::prepared_execution::types::ReplayClosure;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{MadeAgain, Reuse};
use std::borrow::Cow;
/// The rules made again across files, by place, and the captured file frontiers read for them.
#[derive(Debug, PartialEq)]
struct Closure {
    reuse: Cow<'static, [u32]>,
    buckets: Cow<'static, [Bucket]>,
}

/// Whether a producer runs per file (its frontier is captured), and the rules selected by place.
type Input = (bool, &'static [u32]);
type Value = Closure;
type Failure = OperatingError;

const fn closure(reuse: &'static [u32], buckets: &'static [Bucket]) -> Closure {
    Closure {
        reuse: Cow::Borrowed(reuse),
        buckets: Cow::Borrowed(buckets),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "warm_consumer_selects_transitive_cross_producers_and_changed_producer_selects_consumers",
        ),
        word: Word::Succeeds,
        input: || (false, &[2]),
        expect: Expect::Value(closure(&[0, 1, 2], &[])),
    },
    Case {
        name: CaseName("a_changed_producer_selects_its_transitive_consumers"),
        word: Word::Succeeds,
        input: || (false, &[0]),
        expect: Expect::Value(closure(&[0, 1, 2], &[])),
    },
    Case {
        name: CaseName("an_independent_rule_selects_itself_alone"),
        word: Word::Succeeds,
        input: || (false, &[3]),
        expect: Expect::Value(closure(&[3], &[])),
    },
    Case {
        name: CaseName("no_selected_rule_selects_nothing"),
        word: Word::Succeeds,
        input: || (false, &[]),
        expect: Expect::Value(closure(&[], &[])),
    },
    Case {
        name: CaseName("a_rule_past_the_program_is_refused"),
        word: Word::Refuses,
        input: || (false, &[4]),
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        // The actual captured file frontier is read instead of rerunning its producer.
        name: CaseName("per_file_producer_is_not_reexecuted_in_the_consumer_cross_phase"),
        word: Word::Succeeds,
        input: || (true, &[2]),
        expect: Expect::Value(closure(&[1, 2], &[Bucket(10)])),
    },
];

/// Closes the selection over the prepared program's producers, with four file buckets.
fn subject((per_file, selected): Input) -> Result<Value, Failure> {
    let prepared = fixture(per_file);
    let first = [Bucket(10), Bucket(20), Bucket(30), Bucket(40)];
    let ReplayClosure { reuse, buckets } = prepared.replay_closure(selection(selected), &first)?;
    let Reuse::Cross(MadeAgain::These(rules)) = reuse else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(Closure {
        reuse: Cow::Owned(rules.iter().map(|rule| rule.0).collect()),
        buckets: Cow::Owned(buckets),
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
