//! The cases of resident populations a worker reads more than once.
use ryo_relations::domain::relations::types::Relation;
use std::sync::atomic::Ordering;
/// A legal unaudited worker whose immutable resident alternatives change between reads.
pub(crate) struct UnstableResidentFacts {
    /// Delegate mutation/retained-word operations without promising resident identity.
    pub(crate) inner: Box<dyn ryo_execution_runtime::outbound::facts::interface::FactsWorker>,
    /// Distinguishable same-shape populations selected by read order.
    pub(crate) alternatives: [Vec<ryo_relations::domain::relations::types::FactColumn>; 2],
    /// Source reads remain observable even when their shapes agree.
    pub(crate) reads: std::sync::atomic::AtomicUsize,
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
