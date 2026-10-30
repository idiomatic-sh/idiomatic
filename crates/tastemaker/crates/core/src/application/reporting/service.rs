// ignore: idiomatic.arch/file-length - the reporting service is one pass over one lint answer: compare, locate, ignore, identify, each step reading the sources the previous one leased; the steps are already domain functions, and what is left is their order over shared leases.
//! Builds a check report using injected source loading, pattern compilation and fingerprinting.

use super::types::{SourceFile, SourceFiles};
use crate::domain::files::types::{BorrowedText, FileText, LineStarts, SourceView};
use crate::domain::findings::types::Report;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use crate::domain::applications::service::{located, reusable_paths, source_paths};
use crate::domain::applications::types::{Compared, Plan, SpannedFinding};
use crate::domain::comments::service::ignoring;
use crate::domain::comments::types::Comment;
use crate::domain::findings::service::{error, settle};
use crate::domain::findings::types::{Finding, Fingerprint, Identifying, Suppression};
use crate::domain::governance::service::governed;
use crate::domain::import_rows::service::imports;
use crate::domain::imports::service::{by_source, message as import_message, refused, rule_of};
use crate::domain::imports::types::{
    AllowItem, ImportPlace, ImportRows, PatternMatch, WrittenPath,
};
use crate::domain::module_paths::service::{pattern, prefixes};
use crate::domain::paths::service::containment;
use crate::domain::paths::types::{Containment, RepoPath};
use crate::domain::reporting::service::{fingerprinted, incoming_of, report_of};
use crate::domain::reporting::types::{
    Candidate, Fingerprinted, Reported, Settled, Sinks, Standing,
};
use crate::domain::selectors::types::{PathRest, PathSet, PatternFailure, SetCompiler, SetGlob};
use rayon::prelude::*;

/// Renders findings, settles dependent counts, applies ignores and fingerprints the final report.
/// # Errors
/// Returns the source inventory failure without producing a partial report.
pub(super) fn report<E>(
    reported: Reported<'_>,
    read: &impl Fn(&[RepoPath]) -> Result<SourceFiles, E>,
    sets: &dyn SetCompiler,
    fingerprint: &dyn Fn(&Identifying) -> Fingerprint,
    chaining: &dyn Fn(&Identifying) -> Fingerprint,
) -> Result<Report, E> {
    let plan = reported.plan;
    let Settled {
        candidates: all,
        reused,
        sourced,
    } = settled(reported, read, sets, chaining)?;
    let identifying = tracing::info_span!("identify").entered();
    let identified = crate::domain::findings::service::identified(all.iter().map(|candidate| {
        crate::domain::findings::types::PreparedIdentity {
            finding: &candidate.finding,
            line: candidate.line.clone(),
        }
    }));
    let mut fingerprinted = fingerprinted(all, identified, fingerprint);
    fingerprinted.standing.extend(reused.standing);
    fingerprinted.standing.sort();
    fingerprinted.accepted.extend(reused.accepted);
    fingerprinted
        .accepted
        .sort_by(|left, right| left.finding.cmp(&right.finding));
    drop(identifying);
    Ok(report_of(plan, fingerprinted, sourced))
}

/// The report of `report` without its fingerprints, each the default: for a caller that compares findings by rule, path
/// and place only, as validate compares a scenario's findings with the ones it expects.
/// # Errors
/// Returns the source inventory failure without producing a partial report.
pub(super) fn unidentified<E>(
    reported: Reported<'_>,
    read: &impl Fn(&[RepoPath]) -> Result<SourceFiles, E>,
    sets: &dyn SetCompiler,
    chaining: &dyn Fn(&Identifying) -> Fingerprint,
) -> Result<Report, E> {
    let plan = reported.plan;
    let mut split = Fingerprinted::default();
    let Settled {
        candidates,
        reused,
        sourced,
    } = settled(reported, read, sets, chaining)?;
    for Candidate {
        finding,
        acceptance,
        ..
    } in candidates
    {
        match acceptance {
            Standing::Accepted(reason) => split.accepted.push(Suppression { finding, reason }),
            Standing::Stands => split.standing.push(finding),
        }
    }
    split.standing.extend(reused.standing);
    split.standing.sort();
    split.accepted.extend(reused.accepted);
    split
        .accepted
        .sort_by(|left, right| left.finding.cmp(&right.finding));
    Ok(report_of(plan, split, sourced))
}

/// The findings of `reported` rendered, settled and their ignores applied, sorted, before any is fingerprinted.
fn settled<E>(
    reported: Reported<'_>,
    read: &impl Fn(&[RepoPath]) -> Result<SourceFiles, E>,
    sets: &dyn SetCompiler,
    chaining: &dyn Fn(&Identifying) -> Fingerprint,
) -> Result<Settled, E> {
    let Reported {
        plan,
        answers,
        mut findings,
        within,
        reuse,
    } = reported;
    let governing = tracing::info_span!("govern").entered();
    let answers = governed(plan, answers, sets);
    let unread = answers.unread.iter().cloned().collect();
    drop(governing);
    let answering = tracing::info_span!("answer").entered();
    let mut judged =
        crate::domain::answering::service::judge(plan, answers.answers, answers.unknowns);
    findings.append(&mut judged.findings);
    judged.unread = unread;
    drop(answering);
    let comparing = tracing::info_span!("compare").entered();
    // The imports are spelled from the rows: they are judged before any file is read, and only the files they are refused in
    // are read, to place their findings.
    imports_refused(
        plan,
        &judged.imports,
        Sinks {
            whole: &mut findings,
            spanned: &mut judged.spans,
        },
        sets,
    );
    // What each file has before its ignores are applied: a file that has what it had when it was last settled, and is as it was, is
    // not read again, and has the findings it had.
    let ignoring_paths: Vec<&RepoPath> = ignoring(&judged.comments).collect();
    let incoming = incoming_of(&findings, &judged.spans, &ignoring_paths, chaining);
    let reusable = reusable_paths(plan, &judged, &incoming, &reuse.incoming, within);
    let loading = tracing::info_span!("source_inventory").entered();
    let inventory = read(&source_paths(plan, &judged, &findings, &reusable))?;
    drop(loading);
    let source_of = |path: &RepoPath| view(&inventory, path);
    findings.extend(mismatches(
        plan,
        Compared {
            data: &judged.data,
            unread: &judged.unread,
            dropped: &judged.dropped,
        },
        &source_of,
    ));
    let spans = judged.spans;
    drop(comparing);
    let locating = tracing::info_span!("locate").entered();
    let comments = judged.comments;
    let mut grouped: BTreeMap<RepoPath, Vec<SpannedFinding>> = BTreeMap::new();
    for span in spans
        .into_iter()
        .filter(|span| !reusable.contains(&span.finding.path))
    {
        grouped
            .entry(span.finding.path.clone())
            .or_default()
            .push(span);
    }
    findings.par_extend(
        grouped
            .into_par_iter()
            .flat_map_iter(|(path, grouped_spans)| located(grouped_spans, view(&inventory, &path))),
    );
    findings.retain(|finding| !reusable.contains(&finding.path));
    settle(&mut findings);
    drop(locating);
    let ignoring = tracing::info_span!("ignore").entered();
    let candidates = prepared(plan, findings, &comments, within, &inventory, &reusable);
    drop(ignoring);
    let kept = |path: &RepoPath| reusable.contains(path);
    Ok(Settled {
        candidates,
        reused: Fingerprinted {
            standing: reuse
                .findings
                .iter()
                .filter(|finding| kept(&finding.path))
                .cloned()
                .collect(),
            accepted: reuse
                .suppressed
                .iter()
                .filter(|accepted| kept(&accepted.finding.path))
                .cloned()
                .collect(),
        },
        sourced: if within.is_empty() {
            incoming
        } else {
            BTreeMap::new()
        },
    })
}

/// The view of the source at `path` among `sources`; none when it was not read.
fn view<'a>(sources: &'a SourceFiles, path: &RepoPath) -> Option<SourceView<'a>> {
    sources.0.get(path).map(|source| SourceView {
        text: BorrowedText(&source.text.0),
        starts: LineStarts(&source.starts.0),
    })
}

/// The source of `text`, with the byte each of its lines starts at.
pub(super) fn source(text: FileText) -> SourceFile {
    let starts = std::iter::once(0)
        .chain(text.0.bytes().enumerate().filter_map(|(at, byte)| {
            (byte == b'\n').then_some(u64::try_from(at + 1).unwrap_or(u64::MAX))
        }))
        .collect();
    SourceFile {
        text,
        starts: super::types::LineOffsets(starts),
    }
}

/// Performs source-dependent post-settlement work with one lease per file; a file in `skip` is not settled again.
fn prepared(
    plan: &Plan,
    findings: Vec<Finding>,
    comments: &BTreeMap<RepoPath, Vec<Comment>>,
    within: &[RepoPath],
    inventory: &SourceFiles,
    skip: &BTreeSet<RepoPath>,
) -> Vec<Candidate> {
    let mut by_path: BTreeMap<RepoPath, Vec<Finding>> = BTreeMap::new();
    for finding in findings {
        by_path
            .entry(finding.path.clone())
            .or_default()
            .push(finding);
    }
    // A file with no finding is read for its ignores only when one of its comments may hold one.
    for path in ignoring(comments).filter(|path| !skip.contains(*path)) {
        let _: &mut Vec<Finding> = by_path.entry(path.clone()).or_default();
    }
    let mut all = by_path
        .into_par_iter()
        .filter(|(path, _)| {
            within.is_empty()
                || within
                    .iter()
                    .any(|scope| containment(path, scope) == Containment::Inside)
        })
        .flat_map_iter(|(path, path_findings)| {
            crate::domain::reporting::service::prepared_file(
                plan,
                &path,
                path_findings,
                comments.get(&path),
                view(inventory, &path),
            )
        })
        .collect::<Vec<_>>();
    all.sort_by(|left, right| left.finding.cmp(&right.finding));
    all
}

/// Each placed file's mismatches with its template's content, read from the syntax the lint call gave of both: a
/// `#template` error for each declaration the content makes that the file does not, and for an exact template each the
/// file makes that the content does not.
#[must_use]
pub fn mismatches<'a>(
    plan: &Plan,
    compared: Compared<'_>,
    read: &(dyn Fn(&RepoPath) -> Option<SourceView<'a>> + Sync),
) -> Vec<Finding> {
    plan.contracts
        .par_iter()
        .flat_map_iter(|contract| {
            let text = read(&contract.target)
                .map(|source| source.text)
                .unwrap_or_default();
            crate::domain::reporting::service::mismatch(contract, &compared, text)
        })
        .collect()
}

/// Each module pattern an `imports.allow` names that does not compile, refused as a finding of its entry at the workspace's
/// `tastemaker.toml`: a pattern that matches nothing would deny every import it was written to allow, and say nothing of why.
fn unreadable_allows(plan: &Plan, sets: &dyn SetCompiler, whole: &mut Vec<Finding>) {
    let mut unreadable = BTreeSet::new();
    for scope in &plan.imports {
        for check in &scope.checks {
            let items = check
                .allow
                .iter()
                .chain(check.members.iter().flat_map(|member| &member.extra));
            for item in items.filter(|item| !scope.names.contains(&*item.0)) {
                if let Err(failure) = sets.compile_set(&[SetGlob(pattern(item).0)])
                    && unreadable.insert((check.identity.0.clone(), item.0.clone()))
                {
                    whole.push(error(
                        crate::domain::findings::service::check_id(
                            &check.identity.0,
                            crate::domain::imports::constants::IMPORTS,
                        ),
                        crate::domain::paths::service::join(
                            &plan.base,
                            crate::domain::paths::types::PathText(
                                crate::domain::definitions::constants::WORKSPACE_FILE,
                            ),
                        )
                        .unwrap_or_default(),
                        crate::domain::imports::service::unreadable_allow(
                            &check.identity,
                            item,
                            &failure,
                        ),
                    ));
                }
            }
        }
    }
}

/// Each import an entry's `imports` refuses, as a finding of `<identity>#imports` at the import, read from the rows of
/// the import relations: each is spelled as the rows say the file writes it, and no file is read for it.
pub(crate) fn imports_refused(
    plan: &Plan,
    rows: &ImportRows,
    Sinks { whole, spanned }: Sinks<'_>,
    sets: &dyn SetCompiler,
) {
    if plan.imports.is_empty() {
        return;
    }
    let found = imports(rows);
    let made = by_source(&found, &plan.excluded);
    unreadable_allows(plan, sets, whole);
    let compiled =
        Mutex::new(BTreeMap::<Box<str>, Result<Arc<dyn PathSet>, PatternFailure>>::new());
    let matches = |item: &AllowItem, spelled: &WrittenPath| {
        let mut compiled = compiled.lock().unwrap_or_else(PoisonError::into_inner);
        let set = compiled
            .entry(item.0.clone())
            .or_insert_with(|| sets.compile_set(&[SetGlob(pattern(item).0)]));
        // An allow that does not compile matches nothing: `unreadable_allows` above reports it.
        let Ok(set) = set else {
            return PatternMatch::Misses;
        };
        let mut passing = Vec::new();
        let hit = prefixes(spelled).iter().any(|prefix| {
            passing.clear();
            set.passing(PathRest(&prefix.0), &mut passing);
            !passing.is_empty()
        });
        if hit {
            PatternMatch::Matches
        } else {
            PatternMatch::Misses
        }
    };
    for each in plan
        .imports
        .iter()
        .flat_map(|scope| refused(scope, &made, &matches))
    {
        let finding = error(
            rule_of(&each),
            each.import.source.clone(),
            import_message(&each),
        );
        match each.import.span {
            ImportPlace::At(span) => spanned.push(SpannedFinding { finding, span }),
            ImportPlace::Unplaced => whole.push(finding),
        }
    }
}

/// Shared reporting orchestration over the resources chosen by composition.
pub struct ReportingService {
    /// Runs the report in the invocation's pool.
    scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
    /// Compiles path-pattern sets for import allowances.
    matching: Arc<dyn crate::outbound::matching::interface::MatchingPort>,
    /// Fingerprints findings.
    digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
}

impl std::fmt::Debug for ReportingService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl ReportingService {
    /// Bind the outbound contracts once; domain functions receive ordinary values.
    #[must_use]
    pub fn new(
        scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
        matching: Arc<dyn crate::outbound::matching::interface::MatchingPort>,
        digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
    ) -> Self {
        Self {
            scheduler,
            matching,
            digest,
        }
    }

    /// Prepares the report in the invocation's pool, the files a finding is placed in read from `repository`.
    /// # Errors
    /// Returns a scheduling failure without producing a report.
    pub fn prepare(
        &self,
        repository: &dyn crate::outbound::repository::interface::Repository,
        reported: Reported<'_>,
    ) -> Result<Report, super::errors::ReportingError> {
        let mut request = Some(reported);
        let mut result = None;
        self.scheduler.run(&mut || {
            if let Some(request) = request.take() {
                result = Some(report(
                    request,
                    &|paths| {
                        Ok::<_, super::errors::ReportingError>(Self::sources(repository, paths))
                    },
                    &|globs: &[SetGlob]| self.matching.compile_set(globs),
                    &|identity| self.digest.fingerprint(identity),
                    &|identity| self.digest.fingerprint(identity),
                ));
            }
        })?;
        result.unwrap_or_else(|| {
            Err(super::errors::ReportingError::Scheduler(
                crate::outbound::scheduler::errors::SchedulerError::Unavailable {
                    reason: super::constants::UNEXECUTED_REPORT.into(),
                },
            ))
        })
    }

    /// The report `prepare` makes without its fingerprints, in the caller's pool: for a caller that compares findings,
    /// as validate compares each scenario's with the ones it expects, and shows none.
    /// # Errors
    /// Returns the source inventory failure without producing a report.
    pub fn findings(
        &self,
        repository: &dyn crate::outbound::repository::interface::Repository,
        reported: Reported<'_>,
    ) -> Result<Report, super::errors::ReportingError> {
        unidentified(
            reported,
            &|paths| Ok::<_, super::errors::ReportingError>(Self::sources(repository, paths)),
            &|globs: &[SetGlob]| self.matching.compile_set(globs),
            &|identity| self.digest.fingerprint(identity),
        )
    }

    /// Each of `paths` read once, at once; a file that does not read is not a source.
    fn sources(
        repository: &dyn crate::outbound::repository::interface::Repository,
        paths: &[RepoPath],
    ) -> SourceFiles {
        SourceFiles(
            paths
                .par_iter()
                .filter_map(|path| {
                    let text = repository.read(path).ok()?; // ignore: idiomatic.arch/no-ok-on-result - a file that does not read is not a source (the doc above): it has no text to show beside a finding, and the run that reads it reports why
                    Some((path.clone(), source(text)))
                })
                .collect(),
        )
    }
}
