//! Facts a use case test runs a file over when its rows change from one read to the next: a worker over a read, and the columns
//! of a table.
use crate::inbound::use_cases::execution::interface::FactsWorker;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::Bucket;

/// A facts worker whose rows of a relation are what `read` answers on each read, and whose kept words go to the facts fake: for a
/// test whose facts change from one read to the next, or are refused part way.
pub(crate) struct ReadFacts<'f, F> {
    read: F,
    kept: Box<dyn FactsWorker>,
    /// The rows `read` answers live as long as `'f`.
    rows: std::marker::PhantomData<&'f ()>,
}
impl<F> ReadFacts<'_, F> {
    /// The worker reading through `read`.
    pub(crate) fn new(read: F) -> Self {
        Self {
            read,
            kept: ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake::default()
                .open_worker()
                .expect("the facts fake opens a worker"),
            rows: std::marker::PhantomData,
        }
    }
}
impl<'f, F> FactsWorker for ReadFacts<'f, F>
where
    F: Fn(
            ryo_relations::domain::relations::types::Relation,
        ) -> Result<
            ryo_relations::domain::relations::types::ColumnsView<'f>,
            ryo_execution_runtime::outbound::facts::errors::FactsError,
        > + Send
        + Sync,
{
    fn resident_stability(
        &self,
    ) -> ryo_execution_runtime::outbound::facts::types::ResidentStability {
        ryo_execution_runtime::outbound::facts::types::ResidentStability::Stable
    }
    fn columns_mut(
        &mut self,
    ) -> Result<
        ryo_execution_runtime::outbound::facts::types::ColumnsMut<'_>,
        ryo_execution_runtime::outbound::facts::errors::FactsError,
    > {
        self.kept.columns_mut()
    }
    fn rows(
        &self,
        relation: ryo_relations::domain::relations::types::Relation,
    ) -> Result<
        ryo_relations::domain::relations::types::ColumnsView<'_>,
        ryo_execution_runtime::outbound::facts::errors::FactsError,
    > {
        (self.read)(relation)
    }
    fn keep(
        &self,
        bucket: Bucket,
        words: ryo_execution_runtime::outbound::facts::types::WordsToKeep<'_>,
    ) -> Result<(), ryo_execution_runtime::outbound::facts::errors::FactsError> {
        self.kept.keep(bucket, words)
    }
    fn release(
        &mut self,
    ) -> Result<(), ryo_execution_runtime::outbound::facts::errors::FactsError> {
        self.kept.release()
    }
    fn finished(
        self: Box<Self>,
    ) -> Result<
        Box<dyn ryo_execution_runtime::outbound::facts::interface::FactsFinish>,
        ryo_execution_runtime::outbound::facts::errors::FactsError,
    > {
        self.kept.finished()
    }
}
/// The columns of `table`, as a reader writes a relation of its kinds.
pub(crate) fn columns_of(
    table: &Table,
) -> Vec<ryo_relations::domain::relations::types::FactColumn> {
    let width = table.kinds.len();
    (0..width)
        .map(|column| {
            ryo_relations::domain::relations::types::FactColumn(
                table
                    .cells
                    .iter()
                    .skip(column)
                    .step_by(width.max(1))
                    .map(|cell| ryo_relations::domain::relations::fixtures::to_bits(*cell).0)
                    .collect(),
            )
        })
        .collect()
}
/// The rows of `relation` held in `columns`, as a worker lends them.
pub(crate) fn view(
    relation: ryo_relations::domain::relations::types::Relation,
    columns: &[ryo_relations::domain::relations::types::FactColumn],
) -> ryo_relations::domain::relations::types::ColumnsView<'_> {
    ryo_relations::domain::relations::types::ColumnsView {
        kinds: ryo_relations::domain::relations::fixtures::kinds(relation),
        columns,
    }
}
/// The refusal a test's facts give a relation they do not hold.
pub(crate) fn not_held() -> ryo_execution_runtime::outbound::facts::errors::FactsError {
    ryo_execution_runtime::outbound::facts::errors::FactsError::Store {
        reason: "not held".into(),
    }
}
/// The engine's error a file stop answers: its own, or `NotRead` for the facts' refusal, as the engine meets a refused read.
pub(crate) fn operating(
    stop: crate::inbound::use_cases::execution::interface::FileStop,
) -> crate::inbound::use_cases::execution::interface::OperatingError {
    match stop {
        crate::inbound::use_cases::execution::interface::FileStop::Operating(error) => error,
        crate::inbound::use_cases::execution::interface::FileStop::Facts(_)
        | crate::inbound::use_cases::execution::interface::FileStop::Reader => {
            crate::inbound::use_cases::execution::interface::OperatingError::NotRead
        }
    }
}
