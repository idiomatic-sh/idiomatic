//! How the time of the control-flow rules grows with a constructor: each rule linted alone over one derived constructor of
//! 1k, 2k and 4k statements of branches, loops and try blocks, the time at each size no more than 2.2 times the time at the
//! half (the fastest of three runs each): a rule whose work is quadratic in the constructor's blocks takes four times as long.
use super::fixtures::{constructor, fastest, rule};
use ryo::linter::{Cache, Linter, RuleId};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// The sizes of the constructor, in statements, each double the one before.
const SIZES: [usize; 3] = [1000, 2000, 4000];
/// The most the time may grow when the constructor doubles: linear growth and noise, not quadratic.
const MOST_GROWTH: f64 = 2.2;

type Input = &'static str;
type Value = Cow<'static, str>;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("constructor_super_grows_linearly_with_the_constructor"),
        word: Word::Succeeds,
        input: || "eslint.constructor-super",
        expect: Expect::Value(Cow::Borrowed("linear")),
    },
    Case {
        name: CaseName("no_this_before_super_grows_linearly_with_the_constructor"),
        word: Word::Succeeds,
        input: || "eslint.no-this-before-super",
        expect: Expect::Value(Cow::Borrowed("linear")),
    },
    Case {
        name: CaseName("no_unreachable_grows_linearly_with_the_constructor"),
        word: Word::Succeeds,
        input: || "eslint.no-unreachable",
        expect: Expect::Value(Cow::Borrowed("linear")),
    },
];

/// Lints the long constructors with the rule `id` and answers `linear`, or the times when one doubling grew them more.
fn subject(id: Input) -> Result<Value, Failure> {
    let rule = rule(id)?;
    let rule_id = RuleId::try_from(id).map_err(|error| Cow::Owned(error.to_string()))?;
    let pool = ryo::linter::WorkerPool::from(ryo::linter::Workers(std::num::NonZeroUsize::MIN));
    let linter = Linter::new(
        ryo::linter::Languages(vec![ryo_typescript::linked()]),
        pool,
        ryo::linter::MemoryBudget::Machine,
        Cache::None,
    );
    let mut times = Vec::with_capacity(SIZES.len());
    for size in SIZES {
        times.push(fastest(&linter, &rule_id, &rule, &constructor(size))?);
    }
    let growths: Vec<f64> = times
        .windows(2)
        .map(|pair| match pair {
            [half, whole] => whole.as_secs_f64() / half.as_secs_f64().max(f64::MIN_POSITIVE),
            _ => 0.0,
        })
        .collect();
    if growths.iter().all(|growth| *growth <= MOST_GROWTH) {
        Ok(Cow::Borrowed("linear"))
    } else {
        Err(Cow::Owned(format!(
            "{id} at {SIZES:?} statements took {times:?}: grew {growths:?}"
        )))
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
