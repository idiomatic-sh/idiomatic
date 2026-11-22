//! The files of a run scanned by the new engine, once the request's rule sets are loaded (`rule_sets`). Each file is read and
//! scanned in the worker that took its job, and its rows are rendered as the old engine's rows are: alone, they are streamed to
//! the rows port in the order of the files; beside the old engine, they are compared with the old engine's rows of the file,
//! and the first difference stops the run. Nothing is done per rule for a file: which rules govern a file is decided once, for
//! a file that has rows. A rule a rule set does not run is answered as not decided, never as run.
use super::super::constants::OVERRAN;
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::streaming::service::FileVisits;
use super::super::types::{RowsDiffer, ScanState, ScannedRun, Scanning, StreamRun, Streaming};
use crate::domain::found_rows::service::{first_difference, render_found};
use crate::domain::found_rows::types::{Compared, ComparedRow, FoundInput};
use crate::domain::ordering::types::Ordered;
use crate::domain::render::service::{rendered_limit, shown_by};
use crate::domain::render::types::{Answering, FindingRule, Rendered};
use crate::domain::value_cells::types::Scene;
use crate::domain::workers::service::{file_at, size_at};
use crate::domain::workers::types::FileOrder;
use crate::outbound::row_sink::types::AnswerCounts;
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::types::{JobCount, JobNumber};
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::file_scan::types::{FileToScan, RuleSetNumber, Scanned};
use ryo_execution::inbound::use_cases::metadata::interface::{
    Governing, Message, MetadataUseCase as _, Unknown,
};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::files::types::{FilePath, SourcePath};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::spike::{EngineMode, engine_compared, engine_difference};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::files::types::{CorpusInput, ExecutablePermission, SourceView};
use ryo_execution_runtime::application::file_scan::types::{HostRead, ObservedPath};
use crate::outbound::language::types::ManifestReading;
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
        let ordinal = SelectedRuleOrdinal(rule.0);
        let at = usize::try_from(rule.0).unwrap_or(usize::MAX);
        // The rule set's pack says what a row of a rule it runs emits: none for a rule it does not run.
        let (emit, id) = (
            self.scan.emit(&self.session, by, ordinal)?,
            self.ids.get(at)?,
        );
        Some(Compared {
            shown: shown_by(self.answering, rule, FindingRule(&id.0)).shown,
            emit,
            kinds: self.scan.kinds(&self.session, by, ordinal),
        })
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
        let later = |rule: RuleNumber| scanning.later.binary_search(&rule).is_ok();
        let differing = first_difference(old, &state.rendered, &|rule| {
            (!later(rule)).then(|| scanning.compared(by, rule)).flatten()
        });
        if !scanning.later.is_empty() {
            let rows = crate::domain::found_rows::service::compared_rows(&state.rendered, &|rule| {
                later(rule).then(|| scanning.compared(by, rule)).flatten()
            });
            for (rule, mut each) in rows {
                state.later.rows.entry(rule).or_default().append(&mut each);
            }
            state.later.by.get_or_insert(by);
        }
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

    /// SPIKE (ryo-fmx3.1): beside the old engine, the new engine's rows of the rules today's engine runs after the files,
    /// gathered in `new`, against `old`, today's rows that left after the files: the first difference fails the run.
    pub(crate) fn compare_later(
        scanning: &Scanning<'_>,
        new: &crate::domain::found_rows::types::LaterRows,
        old: &Rendered,
    ) -> Result<(), FileAnalysisError> {
        let Some(by) = new.by.or_else(|| (!scanning.session.rule_sets.is_empty()).then_some(RuleSetNumber(0))) else {
            return Ok(());
        };
        if scanning.mode() != EngineMode::Both || scanning.later.is_empty() {
            return Ok(());
        }
        let later = |rule: RuleNumber| scanning.later.binary_search(&rule).is_ok();
        let before = crate::domain::found_rows::service::compared_rows(old, &|rule| {
            later(rule).then(|| scanning.compared(by, rule)).flatten()
        });
        // A finding's last two cells are its line and column, which today's engine leaves empty in the rows it makes after
        // the files (the report holds the span they are made from, which is compared): they are not compared here.
        let placed = |rows: Option<&Vec<ComparedRow>>| {
            let mut rows: Vec<ComparedRow> = rows
                .into_iter()
                .flatten()
                .map(|row| match scanning.answering {
                    Answering::Findings => ComparedRow(
                        row.0.rsplitn(3, '\t').last().unwrap_or_default().into(),
                    ),
                    Answering::Rows(_) => row.clone(),
                })
                .collect();
            rows.sort_unstable();
            rows
        };
        let rules: std::collections::BTreeSet<RuleNumber> =
            before.keys().chain(new.rows.keys()).copied().collect();
        for rule in rules {
            let (old, new) = (placed(before.get(&rule)), placed(new.rows.get(&rule)));
            let Some(at) = (0..old.len().max(new.len())).find(|at| old.get(*at) != new.get(*at)) else {
                continue;
            };
            let place = usize::try_from(rule.0).unwrap_or(usize::MAX);
            let said = RowsDiffer {
                rule: scanning.ids.get(place),
                file: None,
                old: old.get(at).cloned(),
                new: new.get(at).cloned(),
            };
            return Err(FileAnalysisError::Execution(OperatingError::EngineHarness {
                reason: engine_difference(&said),
            }));
        }
        Ok(())
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

    /// The run of the new engine alone, answered: its files scanned, then the end of its rows written to the rows port.
    pub(crate) fn answered(
        &self,
        scanning: &Scanning<'_>,
        only: &[FilePath],
    ) -> Result<ScannedRun, FileAnalysisError> {
        let scanned = self.scanned(scanning, only)??;
        self.rows.end(scanned.counts, &scanned.unread)?;
        Ok(scanned)
    }

    /// The run of the new engine alone: each file is scanned once on the scheduler's workers, and its rows are written to the
    /// rows port in the order of the files; then the packs of its rule sets are kept in the store it read, when it read one.
    /// The outer refusal is the scheduler's, the inner one the scan's.
    fn scanned(
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
        Self::measured(scanning, &states);
        let (mut unread, mut unknowns, mut notices) = (Vec::new(), Vec::new(), Vec::new());
        let mut walked = std::collections::BTreeSet::new();
        for state in states {
            if let Some(error) = state.failure {
                return Ok(Err(error));
            }
            unread.extend(state.unread);
            unknowns.extend(state.unknowns);
            notices.extend(state.notices);
            walked.extend(state.walked);
        }
        notices.sort();
        // A rule the engine does not run was not evaluated on the files its rule set's language walked: the answer says so.
        unknowns.extend(scanning.not_run(&walked));
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
            notices,
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
        // SPIKE (ryo-fmx3.1): a file that could not be read is noted unread and still scanned, with no bytes, so that the
        // rules that ask for it (its read failure) run, as today's engine runs them; a manifest no language walks is scanned
        // with no node table.
        let (path, executable, failed) = match file_at(files, at) {
            Some(CorpusInput::Available(entry)) => (&entry.path, entry.executable, None),
            Some(CorpusInput::ReadFailed { entry, reason }) => (
                &entry.path,
                entry.executable,
                Some((reason.0.clone(), ObservedPath::Observed)),
            ),
            Some(CorpusInput::Unreadable { path, reason }) => (
                path,
                ExecutablePermission::Unknown,
                Some((reason.0.clone(), ObservedPath::Unobserved)),
            ),
            None => return Ok(()),
        };
        state.bytes.0.clear();
        let failed = match (failed, file_at(files, at)) {
            (None, Some(CorpusInput::Available(entry))) => self
                .read_into(entry, &mut state.bytes)
                .err()
                .map(|error| (error.to_string().into(), ObservedPath::Observed)),
            (failed, _) => failed,
        };
        let read = match failed {
            Some((detail, observed)) => {
                state.bytes.0.clear();
                state.unread.push(unread(path, ReadFailure(detail.clone())));
                HostRead::Failed { detail, observed }
            }
            None if self.language.reads_manifest(path) == ManifestReading::Read => HostRead::Manifest,
            None => HostRead::Source,
        };
        let failed_read = matches!(read, HostRead::Failed { .. });
        let number = FileId(job.0);
        state.rows.clear();
        let scanned = scanning.scan.scan(
            &scanning.session,
            &mut state.worker,
            FileToScan {
                path: SourcePath(&path.0),
                file: number,
                source: ByteSlice(&state.bytes.0),
                executable,
                hosted: Some(&scanning.hosted),
                read,
            },
            &mut state.rows,
            &mut state.noticed,
        );
        // A rule that passed its allowance is warned of whatever the scan answered; a run set to fail on it stops here.
        let warned = self.warned(scanning, state, path);
        if let Scanned::Overran(_) = scanned {
            let reason = [OVERRAN, &*warned.0].concat();
            return Err(OperatingError::EngineHarness { reason }.into());
        }
        let (by, fault) = match scanned {
            Scanned::Whole(by) | Scanned::Overran(by) => (Some(by), None),
            Scanned::Faulted(by, fault) => (Some(by), Some(fault)),
            Scanned::NotWalked(fault) => (None, Some(fault)),
        };
        state.by = by;
        state.walked.extend(by);
        match fault.map(|fault| (fault.of, fault.sentence.0)) {
            // A file that could not be read is noted unread already.
            Some((FaultOf::NoLanguage | FaultOf::File, _)) if failed_read => {}
            Some((FaultOf::NoLanguage, _)) => state.unread.push(Unread {
                path: path.clone(),
                reason: Unreadable::NoLanguage,
            }),
            Some((FaultOf::File, said)) => {
                state.unread.push(unread(path, ReadFailure(said)));
            }
            Some((FaultOf::Rule(rule), said)) => {
                let id = scanning
                    .ids
                    .get(usize::try_from(rule.0).unwrap_or(usize::MAX));
                state.unknowns.extend(id.map(|id| Unknown {
                    rule: id.clone(),
                    path: path.clone(),
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
            path,
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
            let (Some(emit), Some(id)) = (
                scanning.scan.emit(&scanning.session, by, rule),
                scanning.ids.get(at),
            ) else {
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
