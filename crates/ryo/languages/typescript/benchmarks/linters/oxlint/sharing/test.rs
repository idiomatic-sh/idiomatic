//! How the work of the control-flow rules grows with a constructor: each rule linted alone over one derived constructor of
//! 1k, 2k and 4k statements of branches, loops and try blocks, less the lint of the same file with `eslint.no-debugger` (one
//! pass over the syntax: the reading, the directives and the run every rule shares), the work at each size no more than 2.2
//! times the work at the half (the fastest of five runs each). A rule whose work is quadratic in the constructor's blocks
//! grows four times; what the debug build and the load of the machine add to every lint cancels in the difference.
use super::fixtures::{constructor, fastest, rule};
use ryo::linter::{Cache, Linter, RuleId};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// The sizes of the constructor, in statements, each double the one before.
const SIZES: [usize; 3] = [1000, 2000, 4000];
/// The most the work may grow when the constructor doubles: linear growth and noise, not quadratic.
const MOST_GROWTH: f64 = 2.2;
/// The rule whose lint of the same file is subtracted: one pass over the syntax, linear by construction.
const BASELINE: &str = "eslint.no-debugger";

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
    let checked = rule(id)?;
    let rule_id = RuleId::try_from(id).map_err(|error| Cow::Owned(error.to_string()))?;
    let pool = ryo::linter::WorkerPool::from(ryo::linter::Workers(std::num::NonZeroUsize::MIN));
    let linter = Linter::new(
        ryo::linter::Languages(vec![ryo_typescript::linked()]),
        pool,
        ryo::linter::MemoryBudget::Machine,
        Cache::None,
    );
    let baseline = rule(BASELINE)?;
    let baseline_id = RuleId::try_from(BASELINE).map_err(|error| Cow::Owned(error.to_string()))?;
    let mut times = Vec::with_capacity(SIZES.len());
    for size in SIZES {
        let text = constructor(size);
        let whole = fastest(&linter, &rule_id, &checked, &text)?;
        let shared = fastest(&linter, &baseline_id, &baseline, &text)?;
        times.push(whole.saturating_sub(shared));
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
            "{id} at {SIZES:?} statements took {times:?} beyond {BASELINE}: grew {growths:?}"
        )))
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
