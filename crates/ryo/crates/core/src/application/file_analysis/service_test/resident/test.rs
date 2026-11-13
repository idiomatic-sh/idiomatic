//! The cases of resident populations a worker reads more than once.

use crate::application::file_analysis::errors::FileAnalysisError;
use crate::application::file_analysis::fixtures::resident::UnstableResidentFacts;
use crate::application::file_analysis::fixtures::visit::visited_as;
use crate::application::file_analysis::fixtures::visit::visited_as_with_facts;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::language::fixtures::fake::LanguageFake;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;
use std::sync::atomic::Ordering;

fn unstable_same_shape_resident_populations_remain_distinct() -> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
    use ryo_execution_runtime::outbound::facts::interface::FactsWorker as _;
    use ryo_relations::domain::relations::types::FactColumn;
    let facts = UnstableResidentFacts {
        inner: FactsFake::default().worker().expect("inner"),
        alternatives: [
            vec![
                FactColumn(vec![1]),
                FactColumn(vec![0]),
                FactColumn(vec![0]),
                FactColumn(vec![4]),
            ],
            vec![
                FactColumn(vec![2]),
                FactColumn(vec![0]),
                FactColumn(vec![0]),
                FactColumn(vec![4]),
            ],
        ],
        reads: std::sync::atomic::AtomicUsize::new(0),
    };
    let first = facts
        .rows(Relation::Function)
        .expect("first resident population")
        .columns
        .first()
        .expect("first column")
        .0
        .clone();
    seen.push(format!(
        "{:?}",
        (facts.resident_stability())
            == (ryo_execution_runtime::outbound::facts::types::ResidentStability::Unstable)
    ));
    let second = facts
        .rows(Relation::Function)
        .expect("changed population")
        .columns
        .first()
        .expect("first column")
        .0
        .clone();
    seen.push(format!("{:?}", (first) != (second)));
    Ok(seen.join("|"))
}

fn file_visits_preserve_stable_unstable_and_unread_provider_outcomes()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
    use crate::domain::render::types::Answering;
    use crate::domain::visit::types::Keeping;
    use ryo_relations::domain::relations::types::FactColumn;
    let (stable, _) = visited_as(
        &["stable.rs"],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Discards,
        Arc::new(DigestFake),
    )
    .expect("stable file visit");
    seen.push(format!("{:?}", stable.failure.is_none()));

    let alternate = |value| (0..4).map(|_| FactColumn(vec![value])).collect();
    let facts = UnstableResidentFacts {
        inner: FactsFake::default().worker().expect("facts"),
        alternatives: [alternate(0), alternate(1)],
        reads: std::sync::atomic::AtomicUsize::new(0),
    };
    let (unstable, unstable_language) = visited_as_with_facts(
        &["volatile.rs"],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Discards,
        Arc::new(DigestFake),
        Box::new(facts),
    )
    .expect("unstable file visit");
    seen.push(format!("{:?}", unstable.failure.is_none()));
    seen.push(format!(
        "{:?}",
        unstable_language.parses.load(Ordering::Relaxed) > 0
    ));

    // The corpus fake refuses this path before parsing; no owner is prepared.
    let (unread, _) = visited_as(
        &["unstable.rs"],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Discards,
        Arc::new(DigestFake),
    )
    .expect("unread file visit");
    seen.push(format!("{:?}", unread.failure.is_none()));
    seen.push(format!("{:?}", unread.tables.unread.len()));
    Ok(seen.join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `unstable_same_shape_resident_populations_remain_distinct` scenario.
    UnstableSameShapeResidentPopulationsRemainDistinct,
    /// The `file_visits_preserve_stable_unstable_and_unread_provider_outcomes` scenario.
    FileVisitsPreserveStableUnstableAndUnreadProviderOutcomes,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// What a scenario's run refuses.
type Failure = FileAnalysisError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unstable_same_shape_resident_populations_remain_distinct"),
        word: Word::Succeeds,
        input: || Given::UnstableSameShapeResidentPopulationsRemainDistinct,
        expect: Expect::Value("true|true"),
    },
    Case {
        name: CaseName("file_visits_preserve_stable_unstable_and_unread_provider_outcomes"),
        word: Word::Succeeds,
        input: || Given::FileVisitsPreserveStableUnstableAndUnreadProviderOutcomes,
        expect: Expect::Value("true|true|true|true|1"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::UnstableSameShapeResidentPopulationsRemainDistinct => {
            unstable_same_shape_resident_populations_remain_distinct()?
        }
        Given::FileVisitsPreserveStableUnstableAndUnreadProviderOutcomes => {
            file_visits_preserve_stable_unstable_and_unread_provider_outcomes()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
