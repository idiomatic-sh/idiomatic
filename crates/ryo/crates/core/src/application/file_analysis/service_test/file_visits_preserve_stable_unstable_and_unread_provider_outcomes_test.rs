//! The scenario `file_visits_preserve_stable_unstable_and_unread_provider_outcomes` of the cases of this file.

use super::*;

pub(super) fn file_visits_preserve_stable_unstable_and_unread_provider_outcomes() -> Result<Value, Failure> {
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
    assert!(stable.failure.is_none());

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
    assert!(unstable.failure.is_none());
    assert!(unstable_language.parses.load(Ordering::Relaxed) > 0);

    // The corpus fake refuses this path before parsing; no owner is prepared.
    let (unread, _) = visited_as(
        &["unstable.rs"],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Discards,
        Arc::new(DigestFake),
    )
    .expect("unread file visit");
    assert!(unread.failure.is_none());
    assert_eq!(unread.tables.unread.len(), 1);
    Ok(1)
}
