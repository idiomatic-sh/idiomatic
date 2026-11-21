//! What a run says of the work the new engine noticed. A rule that passes its work allowance in a file is never stopped:
//! the run warns of it, when it is noticed, and lists the warning in its report; only a run set to fail on it stops, with
//! the same warning as its reason. A measuring run says, after its files, what its workers counted and its work ledger.
use super::super::constants::{COUNTERS, LEDGER, WARNINGS_BETWEEN};
use super::super::service::FileAnalysis;
use super::super::streaming::service::FileVisits;
use super::super::types::{Notice, ScanState, Scanning};
use ryo_execution::inbound::use_cases::file_scan::types::FileScanWorker;
use ryo_execution::inbound::use_cases::metadata::interface::Message;
use ryo_language::domain::files::types::{FilePath, SourcePath};
use ryo_operators::spike::{EngineMeasure, engine_measure, engine_said, engine_warned};
use ryo_values::domain::runs::types::RuleId;

impl FileAnalysis {
    /// Words what the engine noticed of the rules' work in the file at `path`, just scanned: each warning names its rule and
    /// the file, and the statement where the query language says it; it is printed now, when it is noticed, and kept for
    /// the report. Answers the file's warnings, one after the other: nothing for a file nothing was noticed in.
    pub(in crate::application::file_analysis) fn warned(
        &self,
        scanning: &Scanning<'_>,
        state: &mut ScanState,
        path: &FilePath,
    ) -> Message {
        let mut said: Vec<Box<str>> = Vec::new();
        // A rule the request does not name has no id: the warning is said all the same, of a rule with none.
        let unnamed = RuleId::default();
        for noticed in std::mem::take(&mut state.noticed) {
            let at = usize::try_from(noticed.rule.0).unwrap_or(usize::MAX);
            let (id, query) = (
                scanning.ids.get(at).unwrap_or(&unnamed),
                scanning.queries.get(at),
            );
            let asked = self.statements.as_ref().zip(query);
            let written = asked.and_then(|(port, query)| port.written(query, noticed.step));
            let file = SourcePath(&path.0);
            let warning = scanning
                .scan
                .worded(&noticed, id, file, written.as_ref(), query);
            engine_warned(&warning.0);
            said.push(warning.0.clone());
            state.notices.push(Notice {
                rule: id.clone(),
                path: path.clone(),
                warning: Message(warning.0),
            });
        }
        Message(said.join(WARNINGS_BETWEEN).into())
    }
}

impl FileVisits {
    /// Says what a measuring run's workers counted over its files, and its work ledger, a line a unit; a run that does not
    /// measure says nothing.
    pub(in crate::application::file_analysis) fn measured(
        scanning: &Scanning<'_>,
        states: &[ScanState],
    ) {
        if engine_measure() == EngineMeasure::Off {
            return;
        }
        let workers: Vec<&FileScanWorker> = states.iter().map(|state| &state.worker).collect();
        let measured = scanning.scan.measured(&scanning.session, &workers);
        let counted = COUNTERS.iter().zip(measured.counted);
        let counted: Vec<String> = counted
            .map(|(named, count)| [*named, &count.0.to_string()].concat())
            .collect();
        engine_said(scanning.mode(), &counted.concat());
        for line in &measured.ledger {
            engine_said(scanning.mode(), &[LEDGER, &*line.0].concat());
        }
    }
}
