//! The visits of the cases of this file.

use super::*;

/// Visits each file at `paths` of a fake corpus, the way a lint does, with the language `language`: the worker that visited them, and
/// the language to ask what it was given.
pub(super) fn visited(
    paths: &[&str],
    language: LanguageFake,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    let answering =
        crate::domain::render::types::Answering::Rows(crate::domain::render::types::RowsAnswer {
            limit: ryo_values::domain::row_texts::types::RowTotal(0),
            headings: Vec::new(),
            columns: crate::domain::render::types::AnswerColumns::One,
        });
    visited_as(
        paths,
        language,
        &answering,
        crate::domain::visit::types::Keeping::Keeps,
        Arc::new(DigestFake),
    )
}

/// Visits each file at `paths` as `visited` does, answering as `answering` says, over a store that keeps as `keeping` says and with
/// the digests `digest`.
pub(super) fn visited_as(
    paths: &[&str],
    language: LanguageFake,
    answering: &crate::domain::render::types::Answering,
    keeping: crate::domain::visit::types::Keeping,
    digest: Arc<dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort>,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    visited_as_with_facts(
        paths,
        language,
        answering,
        keeping,
        digest,
        FactsFake::default().worker()?,
    )
}

pub(super) fn visited_as_with_facts(
    paths: &[&str],
    language: LanguageFake,
    answering: &crate::domain::render::types::Answering,
    keeping: crate::domain::visit::types::Keeping,
    digest: Arc<dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort>,
    facts: Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker>,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    use crate::outbound::corpus::interface::CorpusPort as _;
    use ryo_values::domain::files::types::{CorpusFile, FileBytes};
    let corpus = Arc::new(CorpusFake {
        files: paths
            .iter()
            .map(|path| CorpusFile {
                executable: Default::default(),
                path: FilePath((*path).into()),
                bytes: FileBytes(b"fn main() {}".to_vec()),
            })
            .collect(),
        ..CorpusFake::default()
    });
    let files = corpus.inventory().expect("fake corpus inventory");
    visited_inputs(
        corpus,
        &files.0,
        &[Relation::Function],
        language,
        answering,
        keeping,
        digest,
        facts,
        None,
        1,
    )
}

/// Runs real file admission against injected input observations, retaining the worker for semantic assertions.
pub(super) fn visited_inputs(
    corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort>,
    files: &[ryo_values::domain::files::types::CorpusInput],
    wanted: &[Relation],
    language: LanguageFake,
    answering: &crate::domain::render::types::Answering,
    keeping: crate::domain::visit::types::Keeping,
    digest: Arc<dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort>,
    facts: Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker>,
    kept: Option<&ryo_values::domain::records::types::Record>,
    repetitions: usize,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    visited_inputs_with_rule_demands(
        corpus,
        files,
        wanted,
        language,
        answering,
        keeping,
        digest,
        facts,
        kept,
        repetitions,
        &[],
        &[],
        Mask(vec![1]),
    )
}

pub(super) fn visited_inputs_with_rule_demands(
    corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort>,
    files: &[ryo_values::domain::files::types::CorpusInput],
    wanted: &[Relation],
    language: LanguageFake,
    answering: &crate::domain::render::types::Answering,
    keeping: crate::domain::visit::types::Keeping,
    digest: Arc<dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort>,
    facts: Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker>,
    kept: Option<&ryo_values::domain::records::types::Record>,
    repetitions: usize,
    ids: &[ryo_values::domain::runs::types::RuleId],
    needs: &[ryo_execution::inbound::use_cases::metadata::interface::Needs],
    always: Mask,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    use super::types::{FileVisit, Worker};
    use crate::domain::visit::types::{Noting, RoleRead};
    use ryo_execution::outbound::matching::fixtures::fake::MatchingFake;
    use crate::outbound::scheduler::types::JobNumber;
    use ryo_values::domain::runs::types::MemoryBytes;
    let language = Arc::new(language);
    let service = FileAnalysis::new(
        corpus,
        Arc::<LanguageFake>::clone(&language),
        digest,
        Arc::new(MemoryFake::default()),
    );
    let placed = PlacedScopes {
        placement: ScopePlacement {
            always,
            ..ScopePlacement::default()
        },
        matcher: MatchingFake.compile(&[]).expect("empty glob set"),
    };
    let records: Vec<_> = files.iter().map(|_| kept).collect();
    let plans = ids
        .iter()
        .map(|_| Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: Vec::new(),
            steps: vec![ryo_values::domain::plans::types::PlanStep {
                scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
                operation: ryo_values::domain::plans::types::Operation::Select {
                    from: ryo_values::domain::plan_parts::types::Input::Relation(
                        Relation::Function,
                    ),
                    comparisons: Vec::new(),
                    kept: vec![ColumnIndex(0)],
                },
            }],
            comparison_origins: Vec::new(),
            emit: Emit {
                step: StepId(0),
                subject: ColumnIndex(0),
                key: Vec::new(),
                headings: Vec::new(),
            },
            frontier: Vec::new(),
        })
        .collect::<Vec<_>>();
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let (program, request) = prepared_test_program(&plans, ids, files);
    let visit = FileVisit {
        store: None,
        setting: RequestVisitSetting {
            source_language: None,
            program,
            services: &request,
            domain: crate::domain::workers::types::VisitSetting {
                unknown: &[],
                metadata: &metadata,
                wanted,
                paths: &[],
                first_bucket: &[],
                ids,
                applicability: &[],
                applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
                placed: &placed,
                records: &records,
                reuse: &ryo_values::domain::stores::types::Reuse::Whole,
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: &NOTHING_WANTED,
                budget: MemoryBytes(u64::MAX),
                noted: Noting::Unwanted,
                roled: RoleRead::Skips,
                needs,
                answering,
                keeping,
            },
        },
    };
    let mut worker = Worker {
        tables: tables_for(WorkerNumber(0), &[]),
        facts,
        old_rows: None,
        failure: None,
    };
    for _ in 0..repetitions {
        for index in 0..u32::try_from(files.len()).unwrap_or(u32::MAX) {
            service.visit_file(&mut worker, &visit, files, JobNumber(index));
        }
    }
    Ok((worker, language))
}

/// A legal unaudited worker whose immutable resident alternatives change between reads.
pub(super) struct UnstableResidentFacts {
    /// Delegate mutation/retained-word operations without promising resident identity.
    pub(super) inner: Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker>,
    /// Distinguishable same-shape populations selected by read order.
    pub(super) alternatives: [Vec<ryo_relations::domain::relations::types::FactColumn>; 2],
    /// Source reads remain observable even when their shapes agree.
    pub(super) reads: std::sync::atomic::AtomicUsize,
}

impl ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker
    for UnstableResidentFacts
{
    fn resident_stability(
        &self,
    ) -> ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability {
        ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability::Unstable
    }
    fn columns_mut(
        &mut self,
    ) -> Result<
        ryo_execution::inbound::use_cases::execution::types::facts::ColumnsMut<'_>,
        ryo_execution::inbound::use_cases::execution::types::facts::FactsError,
    > {
        self.inner.columns_mut()
    }
    fn rows(
        &self,
        relation: Relation,
    ) -> Result<
        ryo_relations::domain::relations::types::ColumnsView<'_>,
        ryo_execution::inbound::use_cases::execution::types::facts::FactsError,
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
        words: ryo_execution::inbound::use_cases::execution::types::facts::WordsToKeep<'_>,
    ) -> Result<(), ryo_execution::inbound::use_cases::execution::types::facts::FactsError> {
        self.inner.keep(bucket, words)
    }
    fn release(
        &mut self,
    ) -> Result<(), ryo_execution::inbound::use_cases::execution::types::facts::FactsError> {
        self.inner.release()
    }
    fn finished(
        self: Box<Self>,
    ) -> Result<
        Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsFinish>,
        ryo_execution::inbound::use_cases::execution::types::facts::FactsError,
    > {
        self.inner.finished()
    }
}
