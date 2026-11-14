//! The test support of the `execution` use case: the scenario its tests share.
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use ryo_matching::inbound::use_cases::matching::{
    errors::MatchingError, interface::MatchingUseCase, types::RegexSource,
    use_case::MatchingUseCaseImpl,
};
use std::sync::Arc;

#[cfg(test)]
pub(crate) mod facts;
#[cfg(test)]
pub(crate) mod requests;

/// The real operators capability, as the engine composes it.
#[must_use]
pub fn operators() -> impl OperatorsPort {
    ryo_execution_runtime::outbound::operators::factory::use_case(
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
            Arc::new(ryo_operators::outbound::matching::factory::use_case()),
        ),
        ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl::new(),
        ryo_operators::inbound::use_cases::run::use_case::RunUseCaseImpl::new(std::sync::Arc::new(
            ryo_operators::outbound::matching::factory::use_case(),
        )),
    )
}
/// The refusal the engine gives a regular expression the matcher cannot compile, in the
/// matcher's words; `None` when `pattern` compiles.
#[must_use]
pub fn regex_refusal(pattern: &str) -> Option<OperatingError> {
    let error = MatchingUseCaseImpl::new()
        .compile_regex(RegexSource(pattern))
        .err()?;
    let (MatchingError::InvalidRegex { reason } | MatchingError::BadGlob { reason, .. }) = error;
    Some(OperatingError::InvalidRegex { reason })
}
/// The derived capability behind its port, as the edge composes it.
#[must_use]
pub fn derived() -> impl DerivedPort {
    ryo_execution_store::outbound::derived::factory::semantics()
}
/// A facts worker holding the rows of each relation given to it, as a language's reader writes them, that refuses a relation it
/// was not given and counts the reads of its rows: the facts a file runs over through `run_file` in the engine's tests.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
/// use ryo_execution::inbound::use_cases::execution::interface::FactsWorker;
/// use ryo_relations::domain::relations::types::Relation;
/// let facts = TableFacts::new(&[]);
/// assert!(facts.rows(Relation::Function).is_err());
/// assert_eq!(facts.reads(), 1);
/// assert_eq!(facts.history(), [Relation::Function]);
/// ```
pub struct TableFacts {
    /// The columns of each relation given, in the order given.
    held: Vec<(
        ryo_relations::domain::relations::types::Relation,
        Vec<ryo_relations::domain::relations::types::FactColumn>,
    )>,
    /// The relation of each read of rows, in order.
    history: std::sync::Mutex<Vec<ryo_relations::domain::relations::types::Relation>>,
    /// Where the kept words go: the facts fake's worker.
    kept: Box<dyn ryo_execution_runtime::outbound::facts::interface::FactsWorker>,
    /// Whether keeping words is refused, as a store that fails refuses it.
    refuses_keep: bool,
}

impl std::fmt::Debug for TableFacts {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TableFacts")
            .field("held", &self.held)
            .field("history", &self.history)
            .field("refuses_keep", &self.refuses_keep)
            .finish_non_exhaustive()
    }
}

impl TableFacts {
    /// A worker holding each relation's `rows`, whose cells are of the relation's kinds.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
    /// use ryo_execution::inbound::use_cases::execution::interface::FactsWorker;
    /// use ryo_relations::domain::relations::types::Relation;
    /// use ryo_values::domain::rows::types::Table;
    /// let none = Table {
    ///     kinds: ryo_relations::domain::relations::fixtures::kinds(Relation::Comment).to_vec(),
    ///     ..Table::default()
    /// };
    /// let facts = TableFacts::new(&[(Relation::Comment, &none)]);
    /// assert!(facts.rows(Relation::Comment).is_ok());
    /// ```
    ///
    /// # Panics
    ///
    /// When a table's kinds are not its relation's, or the facts fake opens no worker.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "a fixture whose facts fake opens no worker fails the test that built it"
    )]
    pub fn new(
        rows: &[(
            ryo_relations::domain::relations::types::Relation,
            &ryo_values::domain::rows::types::Table,
        )],
    ) -> Self {
        use ryo_relations::domain::relations::fixtures::{kinds, to_bits};
        use ryo_relations::domain::relations::types::FactColumn;
        let held = rows
            .iter()
            .map(|(relation, table)| {
                let kinds = kinds(*relation);
                assert_eq!(
                    table.kinds, kinds,
                    "the rows of {relation:?} are of its kinds"
                );
                let mut columns = vec![FactColumn::default(); kinds.len()];
                for row in table.cells.chunks(kinds.len().max(1)) {
                    for (column, cell) in columns.iter_mut().zip(row) {
                        column.0.push(to_bits(*cell).0);
                    }
                }
                (*relation, columns)
            })
            .collect();
        Self {
            held,
            history: std::sync::Mutex::new(Vec::new()),
            kept: ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake::default()
                .open_worker()
                .expect("the facts fake opens a worker"),
            refuses_keep: false,
        }
    }

    /// The same facts, whose store refuses to keep any word: a captured frontier's storage fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
    /// let facts = TableFacts::new(&[]).refusing_keep();
    /// assert_eq!(facts.reads(), 0);
    /// ```
    #[must_use]
    pub fn refusing_keep(self) -> Self {
        Self {
            refuses_keep: true,
            ..self
        }
    }

    /// How many times rows were read so far.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
    /// assert_eq!(TableFacts::new(&[]).reads(), 0);
    /// ```
    #[must_use]
    pub fn reads(&self) -> usize {
        self.history().len()
    }

    /// The relation of each read of rows so far, in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
    /// assert!(TableFacts::new(&[]).history().is_empty());
    /// ```
    ///
    /// # Panics
    ///
    /// When a read panicked while it recorded itself.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "a poisoned record means a read panicked, which fails the test that reads it"
    )]
    pub fn history(&self) -> Vec<ryo_relations::domain::relations::types::Relation> {
        self.history.lock().expect("the reads are recorded").clone()
    }
}

impl ryo_execution_runtime::outbound::facts::interface::FactsWorker for TableFacts {
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
        self.history
            .lock()
            .map_err(
                |_| ryo_execution_runtime::outbound::facts::errors::FactsError::Store {
                    reason: "the reads were poisoned".into(),
                },
            )?
            .push(relation);
        self.held
            .iter()
            .find(|(each, _)| *each == relation)
            .map(
                |(_, columns)| ryo_relations::domain::relations::types::ColumnsView {
                    kinds: ryo_relations::domain::relations::fixtures::kinds(relation),
                    columns,
                },
            )
            .ok_or_else(
                || ryo_execution_runtime::outbound::facts::errors::FactsError::Store {
                    reason: format!("{relation:?} was not given"),
                },
            )
    }

    fn keep(
        &self,
        bucket: ryo_values::domain::stored_facts::types::Bucket,
        words: ryo_execution_runtime::outbound::facts::types::WordsToKeep<'_>,
    ) -> Result<(), ryo_execution_runtime::outbound::facts::errors::FactsError> {
        if self.refuses_keep {
            return Err(
                ryo_execution_runtime::outbound::facts::errors::FactsError::Store {
                    reason: "the store refused the words".into(),
                },
            );
        }
        self.kept.keep(bucket, words)
    }

    fn release(
        &mut self,
    ) -> Result<(), ryo_execution_runtime::outbound::facts::errors::FactsError> {
        for column in self
            .held
            .iter_mut()
            .flat_map(|(_, columns)| columns.iter_mut())
        {
            column.0.clear();
        }
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

/// What a file's run is given so that its partiality is `publication`, and its evaluation sees `evaluation`: the parts core says
/// are read in part, and how evaluation answers. Evaluation sees the physical partiality or none of it.
///
/// # Panics
///
/// When `evaluation` is neither empty nor `publication`.
#[must_use]
pub fn partiality(
    evaluation: &[crate::inbound::use_cases::execution::interface::Partial],
    publication: &[crate::inbound::use_cases::execution::interface::Partial],
) -> (
    Vec<ryo_values::domain::stored_facts::types::ReadInPart>,
    crate::inbound::use_cases::execution::interface::Partiality,
) {
    use crate::inbound::use_cases::execution::interface::Partiality;
    use crate::inbound::use_cases::metadata::interface::{Lacking, PartialPlace};
    use ryo_relations::domain::relations::types::{Missing, RowKind};
    use ryo_values::domain::stored_facts::types::{PartialAt, PartialScope, ReadInPart};
    assert!(
        evaluation.is_empty() || evaluation == publication,
        "evaluation sees the physical partiality or none of it"
    );
    let said = publication
        .iter()
        .map(|partial| ReadInPart {
            relation: partial.relation,
            cause: partial.cause.clone(),
            lacking: match &partial.lacking {
                Lacking::Any => Missing::Any,
                // A kind named once per case: the leak lasts as long as the test.
                Lacking::Kind(kind) => Missing::Kind(RowKind(Box::leak(kind.0.clone()))),
            },
            at: match partial.at {
                PartialPlace::File(file) => PartialScope::OneFile(PartialAt(file.0)),
                PartialPlace::Repository => PartialScope::Repository,
            },
        })
        .collect();
    let answering = if evaluation.is_empty() {
        Partiality::Relaxed
    } else {
        Partiality::Physical
    };
    (said, answering)
}
