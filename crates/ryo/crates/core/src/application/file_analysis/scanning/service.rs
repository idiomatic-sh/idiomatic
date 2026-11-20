//! The files of a run scanned by the new engine. Before the files, the request's rule sets are loaded once. Each file is then
//! read and scanned in the worker that took its job, and its rows are rendered as the old engine's rows are: alone, they are
//! streamed to the rows port in the order of the files; beside the old engine, they are compared with the old engine's rows of
//! the file, and the first difference stops the run. Nothing is done per rule for a file: which rules govern a file is decided
//! once, for a file that has rows.
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::streaming::service::FileVisits;
use super::super::types::{
    RowsDiffer, ScanRequest, ScanState, ScannedRun, Scanning, StreamRun, Streaming,
};
use crate::domain::found_rows::service::{first_difference, render_found};
use crate::domain::found_rows::types::{Compared, FoundInput};
use crate::domain::ordering::types::Ordered;
use crate::domain::render::service::{rendered_limit, shown_by};
use crate::domain::render::types::{Answering, FindingRule, Rendered};
use crate::domain::repository::service::paths_of;
use crate::domain::value_cells::types::Scene;
use crate::domain::workers::service::{file_at, size_at};
use crate::domain::workers::types::FileOrder;
use crate::outbound::row_sink::types::AnswerCounts;
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::types::{JobCount, JobNumber};
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::file_scan::interface::FileScanUseCase as _;
use ryo_execution::inbound::use_cases::file_scan::types::{
    FileToScan, RuleSetNumber, ScanLoadRequest, Scanned,
};
use ryo_execution::inbound::use_cases::metadata::interface::{
    Governing, Message, MetadataUseCase as _, Unknown,
};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::files::types::{FilePath, SourcePath};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::spike::{EngineMode, engine_compared, engine_difference};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::files::types::{CorpusInput, SourceView};
use ryo_values::domain::records::types::{Unread, Unreadable};
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::runs::types::{RuleNumber, UnknownCause};
use ryo_values::domain::scan_rows::types::FaultOf;
use ryo_values::domain::stored_facts::types::ReadFailure;
use ryo_values::domain::stores::types::{KeptRows, Stored, StoredPack};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, PoisonError};

impl Scanning<'_> {
    /// The packs of the request's rule sets: what the store keeps for the next run. None under the old engine alone.
    pub(crate) fn packs(&self) -> Vec<StoredPack> {
        self.scan.packs(&self.session)
    }

    /// Which engines run the files: under the new engine alone, the old engine prepares and runs nothing.
    pub(crate) fn mode(&self) -> EngineMode {
        self.scan.mode()
    }

    /// How the rows of `rule` are compared in a file the rule set `by` scanned: none for a rule the rule set does not run.
    fn compared(&self, by: RuleSetNumber, rule: RuleNumber) -> Option<Compared<'_>> {
        let accepted = self.scan.accepted(&self.session, by);
        let ordinal = SelectedRuleOrdinal(rule.0);
        let at = usize::try_from(rule.0).unwrap_or(usize::MAX);
        let (emit, id) = (self.emits.get(at)?, self.ids.get(at)?);
        match accepted.binary_search(&ordinal) {
            Ok(_) => Some(Compared {
                shown: shown_by(self.answering, rule, FindingRule(&id.0)).shown,
                emit,
                kinds: self.scan.kinds(&self.session, by, ordinal),
            }),
            Err(_) => None,
        }
    }

    /// A worker's state before its first file.
    fn state(&self) -> ScanState {
        ScanState {
            worker: self.scan.worker(&self.session),
            ..ScanState::default()
        }
    }
}

impl FileVisits {
    /// The new engine's part of a run, its rule sets loaded: under the old engine alone nothing is loaded and the store's packs
    /// are not read; otherwise the packs the store kept are read, and each rule set is read back from its pack or lowered.
    pub(crate) fn scanning<'a>(
        &self,
        request: ScanRequest<'a, '_>,
    ) -> Result<Scanning<'a>, FileAnalysisError> {
        let scan = Box::new(self.analysis.scanner(request.named));
        // The packs are asked of the store only by a run of the new engine.
        let stored = match (request.read, scan.mode()) {
            (_, EngineMode::Old) | (None, _) => Vec::new(),
            (Some(read), EngineMode::New | EngineMode::Both) => read.packs()?,
        };
        let session = scan.load(ScanLoadRequest {
            plans: request.plans,
            digests: request.digests,
            stored: &stored,
        })?;
        // The path of each file is read only when the new engine scans one.
        let paths = match scan.mode() {
            EngineMode::Old => Vec::new(),
            EngineMode::New | EngineMode::Both => paths_of(request.files),
        };
        Ok(Scanning {
            scan,
            session,
            emits: request.plans.iter().map(|plan| plan.emit.clone()).collect(),
            ids: request.ids,
            placed: request.placed,
            paths,
            answering: request.answering,
            files: request.files,
            read: request.read,
            unknown: self.analysis.language.unknown_reasons().0,
        })
    }

    /// A worker's state for the comparison of the two engines' rows, when the run compares them.
    pub(in crate::application::file_analysis) fn comparing(
        scanning: &Scanning<'_>,
    ) -> Option<ScanState> {
        (scanning.mode() == EngineMode::Both).then(|| scanning.state())
    }

    /// Scans the file of `job` with the new engine and compares its rows with `old`, the old engine's rows of the file as they
    /// leave: the first difference is the worker's failure, naming the rule, the file and the row each engine has there.
    pub(in crate::application::file_analysis) fn compare(
        &self,
        scanning: &Scanning<'_>,
        state: &mut ScanState,
        old: &Rendered,
        job: JobNumber,
    ) {
        self.analysis
            .scan_file(scanning, state, scanning.files, job);
        let Some(by) = state.by else {
            return;
        };
        let differing = first_difference(old, &state.rendered, &|rule| scanning.compared(by, rule));
        if let (Some(difference), None) = (differing, &state.failure) {
            let at = usize::try_from(difference.rule.0).unwrap_or(usize::MAX);
            let place = usize::try_from(job.0).unwrap_or(usize::MAX);
            let said = RowsDiffer {
                rule: scanning.ids.get(at),
                file: scanning.paths.get(place),
                old: difference.old,
                new: difference.new,
            };
            state.failure = Some(FileAnalysisError::Execution(
                OperatingError::EngineHarness {
                    reason: engine_difference(&said),
                },
            ));
        }
    }

    /// Says the two engines' rows were compared and equal, when the run compared them: how many rules the new engine ran.
    pub(in crate::application::file_analysis) fn compared(scanning: &Scanning<'_>) {
        if scanning.mode() == EngineMode::Both {
            let rules: usize = (0..scanning.session.rule_sets.len())
                .map(|at| {
                    let accepted = scanning.scan.accepted(&scanning.session, RuleSetNumber(at));
                    accepted.len()
                })
                .sum();
            engine_compared(rules);
        }
    }

    /// The run of the new engine alone: each file is scanned once on the scheduler's workers, and its rows are written to the
    /// rows port in the order of the files; then the packs of its rule sets are kept in the store it read, when it read one.
    /// The outer refusal is the scheduler's, the inner one the scan's.
    pub(crate) fn scanned(
        &self,
        scanning: &Scanning<'_>,
        only: &[FilePath],
    ) -> Result<Result<ScannedRun, FileAnalysisError>, SchedulerError> {
        let run = &StreamRun {
            files: scanning.files,
            answering: scanning.answering,
            only,
        };
        let limit = self.begun(run.answering);
        let streaming = Streaming {
            ordered: Mutex::new(Ordered {
                limit,
                ..Ordered::default()
            }),
            turn: Condvar::new(),
        };
        let counted = AtomicU64::new(0);
        let mut states: Vec<ScanState> = (0..self.workers.0).map(|_| scanning.state()).collect();
        let mut jobs: Vec<Box<dyn FnMut(JobNumber) + Send + '_>> = Vec::new();
        for state in &mut states {
            let (streaming, counted) = (&streaming, &counted);
            jobs.push(Box::new(move |job| {
                let place = FileOrder(usize::try_from(job.0).unwrap_or(usize::MAX));
                let size = size_at(run.files, place);
                Self::admit(streaming, size);
                self.analysis.scan_file(scanning, state, run.files, job);
                Self::release(streaming, size);
                let rendered = std::mem::take(&mut state.rendered);
                state.rendered = self.leave(streaming, place, rendered, run, counted);
            }));
        }
        let count = JobCount(u32::try_from(run.files.len()).unwrap_or(u32::MAX));
        self.scheduler.run(&mut jobs, count)?;
        drop(jobs);
        let (mut unread, mut unknowns) = (Vec::new(), Vec::new());
        for state in states {
            if let Some(error) = state.failure {
                return Ok(Err(error));
            }
            unread.extend(state.unread);
            unknowns.extend(state.unknowns);
        }
        unread.sort_by(|left: &Unread, right| left.path.cmp(&right.path));
        unknowns.sort();
        let shown = RowTotal(counted.load(Ordering::Relaxed));
        let total = match run.answering {
            Answering::Findings => shown,
            Answering::Rows(_) => {
                streaming
                    .ordered
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .total
            }
        };
        // The rule sets' packs are kept in the store the run read, in place of what it held: the old engine kept nothing now.
        if let (Some(read), Answering::Findings) = (scanning.read, run.answering) {
            let kept = Stored {
                packs: scanning.packs(),
                ..Stored::default()
            };
            if let Err(error) = read.keep(kept, &KeptRows::default()) {
                return Ok(Err(error.into()));
            }
        }
        Ok(Ok(ScannedRun {
            unread,
            unknowns,
            counts: AnswerCounts { total, shown },
        }))
    }
}

impl FileAnalysis {
    /// The scan of one file by the new engine, in the worker that took its job: read it, scan it, note what no language walked
    /// or a rule could not decide, and render the rows of the rules that govern it into the worker's rendering, in the order
    /// they leave in. What stops the run is the worker's failure.
    pub(in crate::application::file_analysis) fn scan_file(
        &self,
        scanning: &Scanning<'_>,
        state: &mut ScanState,
        files: &[CorpusInput],
        job: JobNumber,
    ) {
        crate::domain::ordering::service::clear(&mut state.rendered);
        state.by = None;
        if let Err(error) = self.scan(scanning, state, files, job)
            && state.failure.is_none()
        {
            state.failure = Some(error);
        }
    }

    /// The scan of one file, answering what stops the run.
    fn scan(
        &self,
        scanning: &Scanning<'_>,
        state: &mut ScanState,
        files: &[CorpusInput],
        job: JobNumber,
    ) -> Result<(), FileAnalysisError> {
        let at = FileOrder(usize::try_from(job.0).unwrap_or(usize::MAX));
        let unread = |path: &FilePath, said: ReadFailure| Unread {
            path: path.clone(),
            reason: Unreadable::Failed(said),
        };
        let entry = match file_at(files, at) {
            Some(CorpusInput::Available(entry)) => entry,
            Some(CorpusInput::ReadFailed { entry, reason }) => {
                let said = ReadFailure(reason.0.clone());
                state.unread.push(unread(&entry.path, said));
                return Ok(());
            }
            Some(CorpusInput::Unreadable { path, reason }) => {
                state
                    .unread
                    .push(unread(path, ReadFailure(reason.0.clone())));
                return Ok(());
            }
            None => return Ok(()),
        };
        state.bytes.0.clear();
        if let Err(error) = self.read_into(entry, &mut state.bytes) {
            state.bytes.0.clear();
            let said = ReadFailure(error.to_string().into());
            state.unread.push(unread(&entry.path, said));
            return Ok(());
        }
        let number = FileId(job.0);
        state.rows.clear();
        let scanned = scanning.scan.scan(
            &scanning.session,
            &mut state.worker,
            FileToScan {
                path: SourcePath(&entry.path.0),
                file: number,
                source: ByteSlice(&state.bytes.0),
            },
            &mut state.rows,
        );
        let (by, fault) = match scanned {
            Scanned::Whole(by) => (Some(by), None),
            Scanned::Faulted(by, fault) => (Some(by), Some(fault)),
            Scanned::NotWalked(fault) => (None, Some(fault)),
        };
        state.by = by;
        match fault.map(|fault| (fault.of, fault.sentence.0)) {
            Some((FaultOf::NoLanguage, _)) => state.unread.push(Unread {
                path: entry.path.clone(),
                reason: Unreadable::NoLanguage,
            }),
            Some((FaultOf::File, said)) => {
                state.unread.push(unread(&entry.path, ReadFailure(said)));
            }
            Some((FaultOf::Rule(rule), said)) => {
                let id = scanning
                    .ids
                    .get(usize::try_from(rule.0).unwrap_or(usize::MAX));
                state.unknowns.extend(id.map(|id| Unknown {
                    rule: id.clone(),
                    path: entry.path.clone(),
                    message: Message(said),
                    cause: UnknownCause::Undecided,
                }));
            }
            None => {}
        }
        let Some(by) = by.filter(|_| !state.rows.is_empty()) else {
            return Ok(());
        };
        // Which rules govern the file, once for it: a file with no row is never asked.
        MetadataUseCaseImpl.place(
            scanning.placed,
            &entry.path,
            &mut state.mask,
            &mut state.hits,
        );
        let scene = Scene {
            file: number,
            symbols: &state.symbols,
            elements: &state.elements,
            source: SourceView(&state.bytes.0),
            paths: &scanning.paths,
            unknown: &scanning.unknown,
        };
        // The rows are sorted by rule: each stretch of one rule's rows is rendered at once.
        for rows in state.rows.chunk_by(|left, right| left.rule == right.rule) {
            let Some(rule) = rows.first().map(|row| row.rule) else {
                continue;
            };
            let number = RuleNumber(rule.0);
            let at = usize::try_from(rule.0).unwrap_or(usize::MAX);
            let (Some(emit), Some(id)) = (scanning.emits.get(at), scanning.ids.get(at)) else {
                continue;
            };
            if MetadataUseCaseImpl.governs(&state.mask, number) != Governing::Governs {
                continue;
            }
            let input = FoundInput {
                rule: number,
                emit,
                kinds: scanning.scan.kinds(&scanning.session, by, rule),
                layout: shown_by(scanning.answering, number, FindingRule(&id.0)),
                scene,
                limit: rendered_limit(scanning.answering),
            };
            render_found(input, rows, &mut state.starts, &mut state.rendered)
                .map_err(|error| ryo_values::domain::rule_requests::service::failed(id, &error))?;
        }
        if *scanning.answering == Answering::Findings {
            crate::domain::ordering::service::sort_findings(&mut state.rendered);
        }
        Ok(())
    }
}
