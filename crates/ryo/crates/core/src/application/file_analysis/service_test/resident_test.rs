//! The cases of resident populations a worker reads more than once.
use super::super::errors::FileAnalysisError;
use super::visit_test::visited_as;
use super::visit_test::visited_as_with_facts;
use crate::domain::render::types::Answering;
use crate::domain::visit::types::Keeping;
use crate::outbound::language::fixtures::fake::LanguageFake;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;
use std::sync::atomic::Ordering;
/// A legal unaudited worker whose immutable resident alternatives change between reads.
pub(super) struct UnstableResidentFacts {
    /// Delegate mutation/retained-word operations without promising resident identity.
    inner: Box<dyn ryo_execution_runtime::outbound::facts::interface::FactsWorker>,
    /// Distinguishable same-shape populations selected by read order.
    alternatives: [Vec<ryo_relations::domain::relations::types::FactColumn>; 2],
    /// Source reads remain observable even when their shapes agree.
    reads: std::sync::atomic::AtomicUsize,
}

pub(super) fn unstable_same_shape_resident_populations_remain_distinct()
-> Result<usize, FileAnalysisError> {
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
    assert_eq!(
        facts.resident_stability(),
        ryo_execution_runtime::outbound::facts::types::ResidentStability::Unstable
    );
    let second = facts
        .rows(Relation::Function)
        .expect("changed population")
        .columns
        .first()
        .expect("first column")
        .0
        .clone();
    assert_ne!(
        first, second,
        "same relation/schema/count is insufficient without an audited stability promise"
    );
    Ok(1)
}

pub(super) fn file_visits_preserve_stable_unstable_and_unread_provider_outcomes()
-> Result<usize, FileAnalysisError> {
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

impl ryo_execution_runtime::outbound::facts::interface::FactsWorker for UnstableResidentFacts {
    fn resident_stability(
        &self,
    ) -> ryo_execution_runtime::outbound::facts::types::ResidentStability {
        ryo_execution_runtime::outbound::facts::types::ResidentStability::Unstable
    }
    fn columns_mut(
        &mut self,
    ) -> Result<
        ryo_execution_runtime::outbound::facts::types::ColumnsMut<'_>,
        ryo_execution_runtime::outbound::facts::errors::FactsError,
    > {
        self.inner.columns_mut()
    }
    fn rows(
        &self,
        relation: Relation,
    ) -> Result<
        ryo_relations::domain::relations::types::ColumnsView<'_>,
        ryo_execution_runtime::outbound::facts::errors::FactsError,
    > {
        use ryo_relations::domain::relations::{service::kinds, types::ColumnsView};
        if relation != Relation::Function {
            return self.inner.rows(relation);
        }
        let place = self.reads.fetch_add(1, Ordering::Relaxed) % 2;
        let columns = self
            .alternatives
            .get(place)
            .expect("one of two immutable alternatives");
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns,
        })
    }
    fn keep(
        &self,
        bucket: ryo_values::domain::stored_facts::types::Bucket,
        words: ryo_execution_runtime::outbound::facts::types::WordsToKeep<'_>,
    ) -> Result<(), ryo_execution_runtime::outbound::facts::errors::FactsError> {
        self.inner.keep(bucket, words)
    }
    fn release(
        &mut self,
    ) -> Result<(), ryo_execution_runtime::outbound::facts::errors::FactsError> {
        self.inner.release()
    }
    fn finished(
        self: Box<Self>,
    ) -> Result<
        Box<dyn ryo_execution_runtime::outbound::facts::interface::FactsFinish>,
        ryo_execution_runtime::outbound::facts::errors::FactsError,
    > {
        self.inner.finished()
    }
}
