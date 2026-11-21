//! The rule sets of a run of the new engine. What a run was given (its rules as they were written, their fragments, what the
//! host declared, which were selected) is one digest, made before anything is compiled, and a rule set's pack is kept under it:
//! the store is asked with it first, and a run given what the store holds the packs of reads its rule sets back with no plan in
//! hand; another lowers them from its compiled rules. A rule a rule set does not run decided nothing in the files that rule
//! set's language walked: that is answered as not decided, at the repository as a whole, never as a rule that ran.
use super::super::constants::{NOT_RUN_BY, NOT_RUN_FOR};
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::streaming::service::FileVisits;
use super::super::types::{ScanRequest, Scanning};
use crate::domain::keeping::types::DigestInput;
use crate::domain::repository::service::{
    catalog_schema, given, paths_of, source_selection_identity, statistics_identity,
};
use ryo_execution::inbound::use_cases::file_scan::interface::FileScanUseCase;
use ryo_execution::inbound::use_cases::file_scan::types::{
    RuleSetNumber, ScanLoadRequest, ScanSession, ScanStoredRequest,
};
use ryo_execution::inbound::use_cases::metadata::interface::{
    Message, MetadataUseCase as _, Unknown,
};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution_runtime::outbound::digest::types::DigestBytes;
use ryo_operators::spike::{EngineMode, engine_statistics};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::hosted::service::identity as hosted_identity;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_plans::service::{compiled_step, reach_unfolded};
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::runs::types::UnknownCause;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::StoredPack;
use std::borrow::Cow;
use std::collections::BTreeSet;

impl Scanning<'_> {
    /// What the rules a rule set does not run could not decide: each of them, once, at the repository as a whole, for every
    /// rule set in `walked`, the ones whose language walked a file of the run. A rule the new engine's load refused ran on none
    /// of those files: it found nothing there because it did not look.
    pub(in crate::application::file_analysis) fn not_run(
        &self,
        walked: &BTreeSet<RuleSetNumber>,
    ) -> Vec<Unknown> {
        let mut unknowns = Vec::new();
        for by in walked {
            let Some(set) = self.session.rule_sets.get(by.0) else {
                continue;
            };
            let message: Box<str> = [NOT_RUN_BY, &*set.load.language.0, NOT_RUN_FOR]
                .concat()
                .into();
            for (place, id) in self.ids.iter().enumerate() {
                let ordinal = SelectedRuleOrdinal(u32::try_from(place).unwrap_or(u32::MAX));
                if set.load.accepted.binary_search(&ordinal).is_err() {
                    unknowns.push(Unknown {
                        rule: id.clone(),
                        path: MetadataUseCaseImpl.repository(),
                        message: Message(message.clone()),
                        cause: UnknownCause::Undecided,
                    });
                }
            }
        }
        unknowns
    }
}

impl FileVisits {
    /// The packs the store the run read kept: none for a run that read no store.
    fn packs(request: &ScanRequest<'_, '_>) -> Result<Vec<StoredPack>, FileAnalysisError> {
        Ok(match request.read {
            Some(read) => read.packs()?,
            None => Vec::new(),
        })
    }

    /// The new engine's part of a run over the rule sets of `session`.
    fn over<'a>(
        &self,
        scan: Box<dyn FileScanUseCase>,
        session: ScanSession,
        request: ScanRequest<'a, '_>,
        compiled: Vec<Vec<StepId>>,
    ) -> Scanning<'a> {
        // The path of each file is read only when the new engine scans one.
        let paths = match scan.mode() {
            EngineMode::Old => Vec::new(),
            EngineMode::New | EngineMode::Both => paths_of(request.files),
        };
        let queries = match scan.mode() {
            EngineMode::Old => Vec::new(),
            EngineMode::New | EngineMode::Both => {
                let rules = request.rules.iter();
                rules.map(|rule| rule.query.clone()).collect()
            }
        };
        Scanning {
            scan,
            session,
            ids: request.ids,
            placed: request.placed,
            paths,
            answering: request.answering,
            files: request.files,
            read: request.read,
            unknown: self.analysis.language.unknown_reasons().0,
            queries,
            compiled,
            hosted: std::sync::Arc::default(),
            later: Vec::new(),
        }
    }

    /// The new engine's part of a run as the store kept it, asked before anything is compiled: the packs the store kept are
    /// read, and each rule set is read back from its pack of what the run was given. None when the store lacks one of them:
    /// the run compiles its rules and asks `scanning`. Under the old engine alone nothing is read and the part holds no rule set.
    pub(crate) fn stored<'a>(
        &self,
        request: ScanRequest<'a, '_>,
    ) -> Result<Option<Scanning<'a>>, FileAnalysisError> {
        let scan = Box::new(self.analysis.scanner(request.named));
        let session = match scan.mode() {
            EngineMode::Old => Some(ScanSession::default()),
            EngineMode::New | EngineMode::Both => scan.stored(ScanStoredRequest {
                given: self.analysis.given(&request),
                selected: Count(request.rules.len()),
                stored: &Self::packs(&request)?,
            })?,
        };
        Ok(session.map(|session| self.over(scan, session, request, Vec::new())))
    }

    /// The new engine's part of a run, its rule sets loaded from the compiled rules: under the old engine alone nothing is
    /// loaded and the store's packs are not read; otherwise the packs the store kept are read, and each rule set is read back
    /// from its pack of what the run was given or lowered from `plans`, the plan of each selected rule in rule order, each with
    /// what the rule binds its parameters to (`bindings`, in the same order).
    pub(crate) fn scanning<'a>(
        &self,
        request: ScanRequest<'a, '_>,
        plans: &[&Plan],
        bindings: &[Vec<ParameterBinding>],
    ) -> Result<Scanning<'a>, FileAnalysisError> {
        let scan = Box::new(self.analysis.scanner(request.named));
        let session = match scan.mode() {
            EngineMode::Old => ScanSession::default(),
            EngineMode::New | EngineMode::Both => scan.load(ScanLoadRequest {
                plans,
                bindings,
                statistics: engine_statistics(),
                given: self.analysis.given(&request),
                stored: &Self::packs(&request)?,
            })?,
        };
        // A notice names a step of the plan the engine received: each such step's own step of the compiled plan.
        let compiled = match scan.mode() {
            EngineMode::Old => Vec::new(),
            EngineMode::New | EngineMode::Both => plans.iter().map(|plan| compiled_steps(plan)).collect(),
        };
        Ok(self.over(scan, session, request, compiled))
    }
}

/// The step of a compiled plan that each step of the plan the new engine receives is: none for a plan the engine receives
/// as it was compiled.
fn compiled_steps(plan: &Plan) -> Vec<StepId> {
    match reach_unfolded(plan) {
        Cow::Borrowed(_) => Vec::new(),
        Cow::Owned(received) => (0..received.steps.len())
            .map(|place| compiled_step(plan, StepId(u32::try_from(place).unwrap_or(u32::MAX))))
            .collect(),
    }
}

impl FileAnalysis {
    /// What a run was given, as one digest, made before anything is compiled: its selected rules as they were written, in
    /// order, with the fragments their queries name read once (`given`); what the host declared; the catalog's relations and
    /// columns; the language the request names, when it names one; and the statistics the rules are lowered with, when the
    /// run names some. A rule set's pack is kept under it, with the language's kinds and the engine's version: a changed rule
    /// or fragment, another selection, another declaration or other statistics is another pack, and a run given the same
    /// reads its pack back and compiles nothing. One pass over the bytes, and nothing per rule but its own text.
    fn given(&self, request: &ScanRequest<'_, '_>) -> Digest {
        let part = |bytes: DigestInput| self.digest.digest(DigestBytes(&bytes.0));
        let mut parts = vec![
            part(given(request.rules)),
            self.digest
                .digest(DigestBytes(&hosted_identity(request.hosted).0)),
            part(catalog_schema()),
            part(source_selection_identity(request.named)),
        ];
        // A run that names no statistics is given what it was given before statistics were: its packs keep their keys.
        parts.extend(statistics_identity(engine_statistics()).map(part));
        self.digest.combine(&parts)
    }
}
