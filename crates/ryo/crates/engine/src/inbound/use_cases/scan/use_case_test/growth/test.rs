//! The growth harness: for every kind of operation, its growth plan is loaded and scanned over a file of 1,024 words and one
//! of 4,096, and its counted work (rows visited and rows its fills read and gave, never a time) over the rows it read and found
//! may not rise by more than a quarter. A case holds only when it finds its rows at both sizes. A kind whose lowering or
//! kernels have not landed is on the pending list: the harness fails for a listed kind whose case holds, so the list cannot
//! hide one, and for an unlisted kind whose case does not.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::digests_of;
use crate::domain::lowering::fixtures::growth::growth_plan;
use crate::domain::lowering::fixtures::pending::{GROWTH_PENDING, growth_bindings};
use crate::domain::lowering::types::{LoadGiven, SelectedRule, SelectedRules};
use crate::domain::statistics::types::StatisticsBytes;
use crate::inbound::use_cases::load::fixtures::kinds;
use crate::inbound::use_cases::load::interface::LoadUseCase as _;
use crate::inbound::use_cases::load::types::LoadSource;
use crate::inbound::use_cases::load::use_case::LoadUseCaseImpl;
use crate::inbound::use_cases::scan::fixtures::{each_its_own, scanned};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// The two sizes a growth plan is scanned at, in words.
const SIZES: [usize; 2] = [1_024, 4_096];

type Input = ();
/// How many kinds were run, and each kind the harness fails for with why.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_operations_counted_work_grows_with_its_rows_or_the_operation_is_pending"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Cow::Borrowed("31 kinds run, 14 pending; faults: []")),
}];

/// What a scan of a growth plan over a file of some words counted.
#[derive(Clone, Copy, Debug)]
struct Counted {
    /// The words of the file.
    words: usize,
    /// The rows found.
    found: usize,
    /// The work counted: the rows the programs' iterations gave and the rows the fills' stages read and gave.
    work: u64,
}

/// Loads a kind's growth plan and scans a file of `words` words with it; why not, when it does not load or scan.
fn counted(kind: OperationKind, words: usize) -> Result<Counted, String> {
    let plan = growth_plan(kind);
    let bindings = growth_bindings(&plan);
    let digests = digests_of(&plan, RuleBindings(&bindings));
    let rule = SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan: &plan,
        bindings: RuleBindings(&bindings),
        digests: &digests,
    };
    let loaded = LoadUseCaseImpl
        .load(
            LoadSource::Plans(SelectedRules(&[rule]), StatisticsBytes(&[]), LoadGiven::default()),
            &kinds(),
        )
        .map_err(|error| error.to_string())?;
    if let Some(refused) = loaded.refused.first() {
        return Err(format!("not loaded: {}", refused.refusal));
    }
    let scan = scanned(&loaded.programs, &each_its_own(words))
        .map_err(|error| format!("not walked: {error:?}"))?;
    let scan = scan.ok_or_else(|| "not scanned".to_owned())?;
    let counters = scan.answer.map_err(|error| error.to_string())?;
    if !scan.notices.is_empty() {
        return Err(format!("noticed: {:?}", scan.notices));
    }
    Ok(Counted {
        words,
        found: scan.found.0,
        work: counters.visited.0.saturating_add(counters.fill_rows.0),
    })
}

/// Whether a kind's case holds: it finds its rows at both sizes, and its work over the rows it read and found does not rise by
/// more than a quarter from the smaller file to the larger. Why not, when it does not.
fn held(kind: OperationKind) -> Result<(), String> {
    let [small, large] = SIZES.map(|words| counted(kind, words));
    let (small, large) = (small?, large?);
    if small.found == 0 || large.found == 0 {
        return Err(format!("found {} and {} rows", small.found, large.found));
    }
    // work(large) / rows(large) <= 5/4 * work(small) / rows(small), in whole numbers.
    let rows = |counted: Counted| {
        u128::try_from(counted.words.saturating_add(counted.found)).unwrap_or(u128::MAX)
    };
    let (left, right) = (
        u128::from(large.work)
            .saturating_mul(rows(small))
            .saturating_mul(4),
        u128::from(small.work)
            .saturating_mul(rows(large))
            .saturating_mul(5),
    );
    if left > right {
        return Err(format!(
            "work {} over {} rows, then {} over {} rows: it grows faster than its rows",
            small.work,
            rows(small),
            large.work,
            rows(large)
        ));
    }
    Ok(())
}

fn subject((): Input) -> Result<Value, Failure> {
    let (mut faults, mut waiting) = (Vec::new(), Vec::new());
    for kind in OperationKind::ALL {
        let listed = GROWTH_PENDING.contains(kind);
        match held(*kind) {
            Ok(()) if listed => faults.push(format!(
                "{kind:?} is pending and its case holds: take it off the list"
            )),
            Ok(()) => {}
            Err(why) if listed => waiting.push(why),
            Err(why) => faults.push(format!(
                "{kind:?} is not pending and its case does not hold: {why}"
            )),
        }
    }
    Ok(Cow::Owned(format!(
        "{} kinds run, {} pending; faults: {faults:?}",
        OperationKind::ALL.len(),
        waiting.len()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
