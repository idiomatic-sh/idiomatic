//! The visits of a run's files on the scheduler's workers, each in the worker that took its job, with their findings written to
//! the rows port in the order of the files; the changed files read alone; and the project made on the scheduler's pool.
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::types::{Course, FileVisit, RequestVisitSetting, StreamRun, Streaming, Worker};
use crate::domain::choosing::service::chosen;
use crate::domain::ordering::service::{PENDING_LIMIT, admitted, offer, sort_findings};
use crate::domain::ordering::types::{Admittance, Ordered, Turn};
use crate::domain::render::constants::{
    column_heading, columns_heading, key_heading, line_heading, path_heading, rule_heading,
};
use crate::domain::render::service::{lent_row, row_cells};
use crate::domain::render::types::{Answering, Rendered, RowLimit};
use crate::domain::reporting::service::{room_left, totals};
use crate::domain::reporting::types::RowCounts;
use crate::domain::repository::service::paths_of;
use crate::domain::storing::types::ReadAlone;
use crate::domain::workers::service::{absorb, gathering, size_at, tables_for};
use crate::domain::workers::types::{FileOrder, Visited};
use crate::outbound::row_sink::interface::RowSinkPort;
use crate::outbound::row_sink::types::{AnswerCounts, LentCell};
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::{JobCount, JobNumber, WorkerCount};
use ryo_execution_runtime::outbound::facts::interface::{FactsPort, FactsWorker};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::row_texts::types::{RowTotal, RowsTextView};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};

/// The visits of a run's files: the analysis of one file, the workers it runs in, and where their findings go.
pub(crate) struct FileVisits {
    /// Reads, parses and evaluates one file in the worker that took its job.
    pub(in crate::application::file_analysis) analysis: FileAnalysis,
    /// Holds each worker's rows of a file and keeps the words the cross-file steps read.
    pub(in crate::application::file_analysis) facts: Arc<dyn FactsPort>,
    /// Runs the files' jobs on the workers.
    pub(in crate::application::file_analysis) scheduler: Arc<dyn SchedulerPort>,
    /// How many workers the scheduler runs jobs on: the run makes one worker for each.
    pub(in crate::application::file_analysis) workers: WorkerCount,
    /// Takes the findings the rules write.
    pub(in crate::application::file_analysis) rows: Arc<dyn RowSinkPort>,
}

impl std::fmt::Debug for FileVisits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl FileVisits {
    /// The visits over the analysis of one file and the ports its workers and findings need.
    pub(crate) fn new(
        analysis: FileAnalysis,
        facts: Arc<dyn FactsPort>,
        scheduler: Arc<dyn SchedulerPort>,
        workers: WorkerCount,
        rows: Arc<dyn RowSinkPort>,
    ) -> Self {
        Self {
            analysis,
            facts,
            scheduler,
            workers,
            rows,
        }
    }

    /// The analysis of one file the visits run.
    pub(crate) const fn analysis(&self) -> &FileAnalysis {
        &self.analysis
    }

    /// Reads each file once, whatever the rules read of it, and lets the workers meet. Every worker owns its buffers and its
    /// tables, so no job takes a lock; what the workers kept meets once, after the jobs. The outer refusal is the scheduler's, the
    /// inner one the visits'.
    pub(crate) fn visited(
        &self,
        run: &StreamRun<'_>,
        visit: &FileVisit<'_>,
    ) -> Result<Result<Visited<Box<dyn FactsWorker>>, FileAnalysisError>, SchedulerError> {
        let _reading = tracing::info_span!("read").entered();
        let mut states = Vec::new();
        for number in 0..self.workers.0 {
            let facts = match self.facts.worker() {
                Ok(facts) => facts,
                Err(error) => return Ok(Err(error.into())),
            };
            states.push(Worker::new(tables_for(WorkerNumber(number), run.metadata), facts));
        }
        let (mut headings, limit) = match run.answering {
            Answering::Findings => (
                vec![
                    Heading(rule_heading().into()),
                    Heading(path_heading().into()),
                    Heading(key_heading().into()),
                    Heading(columns_heading().into()),
                ],
                RowTotal(u64::MAX),
            ),
            Answering::Rows(answer) => (answer.headings.clone(), answer.limit),
        };
        headings.push(Heading(line_heading().into()));
        headings.push(Heading(column_heading().into()));
        self.rows.begin(&headings);
        let streaming = Streaming {
            ordered: Mutex::new(Ordered {
                limit,
                ..Ordered::default()
            }),
            turn: Condvar::new(),
        };
        let counted = AtomicU64::new(0);
        let course = Course {
            files: run.files,
            visit,
            streaming: &streaming,
            only: run.only,
            counted: &counted,
        };
        let mut jobs: Vec<Box<dyn FnMut(JobNumber) + Send + '_>> = Vec::new();
        for state in &mut states {
            let course = &course;
            jobs.push(Box::new(move |job| self.process(course, state, job)));
        }
        let count = JobCount(u32::try_from(run.files.len()).unwrap_or(u32::MAX));
        self.scheduler.run(&mut jobs, count)?;
        drop(jobs);
        // The workers meet: their tables, the text they made, what they could not read.
        let mut met = gathering(run.metadata);
        let mut workers = Vec::new();
        for state in states {
            if let Some(error) = state.failure {
                return Ok(Err(error));
            }
            workers.push(state.facts);
            absorb(&mut met, state.tables);
        }
        let counted = RowTotal(counted.load(Ordering::Relaxed));
        let total = match run.answering {
            Answering::Findings => counted,
            Answering::Rows(_) => {
                streaming
                    .ordered
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .total
            }
        };
        Ok(Ok(Visited {
            workers,
            met,
            counted,
            total,
        }))
    }

    /// The job of one file: wait to be admitted, visit the file, and write its findings when it is the file's turn, or when the
    /// files before it have left.
    fn process(&self, course: &Course<'_>, worker: &mut Worker, job: JobNumber) {
        let place = FileOrder(usize::try_from(job.0).unwrap_or(usize::MAX));
        let size = size_at(course.files, place);
        let stream = course.streaming;
        {
            let mut ordered = stream
                .ordered
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            while admitted(&ordered, size) == Admittance::Waits {
                ordered = stream
                    .turn
                    .wait(ordered)
                    .unwrap_or_else(PoisonError::into_inner);
            }
            ordered.inflight.0 += size.0;
        }
        self.analysis
            .visit_file(worker, course.visit, course.files, job);
        stream
            .ordered
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .inflight
            .0 -= size.0;
        stream.turn.notify_all();
        // The file's turn: its findings leave now, or wait for the files before it.
        let rendered = std::mem::take(&mut worker.tables.rendered);
        let mut ordered = stream
            .ordered
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        // Too much waits for the files before this one: this worker waits with it, and the worker that has the file whose turn it
        // is never waits, so the turn always comes.
        while ordered.held.0 >= PENDING_LIMIT && ordered.next != Turn(place.0) {
            ordered = stream
                .turn
                .wait(ordered)
                .unwrap_or_else(PoisonError::into_inner);
        }
        let mut ready = offer(&mut ordered, Turn(place.0), rendered);
        let mut spare = None;
        while let Some(mut each) = ready {
            let written = match course.visit.setting.answering {
                Answering::Findings => {
                    crate::domain::ordering::service::account(&mut ordered, &each);
                    self.write_findings(&each, course.only).0
                }
                Answering::Rows(_) => {
                    let allowed = crate::domain::ordering::service::allowance(&mut ordered, &each);
                    self.write_rows(&each, allowed).0
                }
            };
            let _counted_before = course.counted.fetch_add(written, Ordering::Relaxed);
            if spare.is_none() {
                crate::domain::ordering::service::clear(&mut each);
                spare = Some(each);
            } else {
                crate::domain::ordering::service::recycle(&mut ordered, each);
            }
            ready = crate::domain::ordering::service::next_ready(&mut ordered);
        }
        worker.tables.rendered = spare.or_else(|| ordered.spares.pop()).unwrap_or_default();
        drop(ordered);
        stream.turn.notify_all();
    }

    /// Writes the findings of one file to the rows port, those of the files asked for, in the order they were put in.
    fn write_findings(&self, rendered: &Rendered, only: &[FilePath]) -> RowTotal {
        let mut written = 0;
        for cells in chosen(rendered, only) {
            self.rows
                .row(&mut lent_row(RowsTextView(&rendered.text), cells).map(LentCell::from));
            written += 1;
        }
        RowTotal(written)
    }

    /// Writes the first `allowed` rows of one file's rows of the rules' own columns to the rows port, in the order they were put
    /// in, and answers how many it wrote.
    fn write_rows(&self, rendered: &Rendered, allowed: RowLimit) -> RowTotal {
        let mut written = 0;
        for each in rendered.order.iter().take(allowed.0) {
            let cells = row_cells(&rendered.cells, &rendered.ends, each.row);
            self.rows
                .row(&mut lent_row(RowsTextView(&rendered.text), cells).map(LentCell::from));
            written += 1;
        }
        RowTotal(written)
    }

    /// Writes the rows of the cross-file steps after those the files streamed, `files`, and answers what the answer has and shows:
    /// every finding, in the order findings leave in; or every row, of which those the limit leaves room for are written.
    pub(crate) fn write_later(
        &self,
        run: &StreamRun<'_>,
        later: &mut Rendered,
        files: RowCounts,
    ) -> AnswerCounts {
        match run.answering {
            Answering::Findings => {
                sort_findings(later);
                let findings = RowTotal(files.shown.0 + self.write_findings(later, run.only).0);
                AnswerCounts {
                    total: findings,
                    shown: findings,
                }
            }
            Answering::Rows(answer) => {
                let room = room_left(answer.limit, files.shown);
                let _written = self.write_rows(
                    later,
                    RowLimit(usize::try_from(room.0).unwrap_or(usize::MAX)),
                );
                let RowCounts { total, shown } = totals(files, later.total, room);
                AnswerCounts { total, shown }
            }
        }
    }

    /// What the changed files show other files now, each read alone: the records the visit makes of them, which the findings of the
    /// rules that read across files stand on or fall with.
    pub(crate) fn read_alone(
        &self,
        setting: &RequestVisitSetting<'_>,
        files: &[CorpusInput],
        store: Option<&dyn crate::outbound::lint_store::interface::ReadStore>,
    ) -> Result<ReadAlone, FileAnalysisError> {
        let paths = paths_of(files);
        let setting = RequestVisitSetting {
            domain: crate::domain::workers::types::VisitSetting {
                paths: &paths,
                records: &[],
                ..(setting.clone()).domain
            },
            ..setting.clone()
        };
        let visit = FileVisit {
            store,
            setting: setting.clone(),
        };
        let mut worker = Worker::new(
            tables_for(WorkerNumber(0), setting.metadata),
            self.facts.worker()?,
        );
        for place in 0..files.len() {
            self.analysis.visit_file(
                &mut worker,
                &visit,
                files,
                JobNumber(u32::try_from(place).unwrap_or(u32::MAX)),
            );
            worker.tables.rendered = Rendered::default();
        }
        if let Some(error) = worker.failure {
            return Err(error);
        }
        Ok(ReadAlone {
            records: worker
                .tables
                .fresh
                .into_iter()
                .map(|each| each.record)
                .collect(),
            unknowns: worker.tables.unknowns,
            skipped: worker.tables.skipped,
        })
    }
}
