// ignore: idiomatic.arch/file-length - the check pipeline is one use case: it runs the selected tools, judges findings and perf against the lock, and keeps the runs of the groups it did not select, in the order the report holds them. Its judging rules already live in the domain; what is left is orchestration over the ports and the cache stored every step threads, which the architecture rules keep in the use case, and its two free helpers read outbound types the domain may not import.
//! The `check` use case: run the tools, reusing each run whose inputs did not move, read the accepted state, carry it
//! through git's renames, and judge the one against the other.

use std::collections::{BTreeMap, BTreeSet};

use super::errors::CheckError;
use super::interface::CheckUseCase;
use super::types::{
    Against, CheckPorts, CheckReport, CheckRequest, FailedFix, Fixing, ReadFile, Reuse,
};
use crate::domain::inputs::service::named;
use crate::domain::judgement::service::{compare, tail, verdict, with_perf};
use crate::domain::judgement::types::{Code, Failure, Inventory, Problem, Run, Status, Tail};
use crate::domain::manifest::constants::MANIFEST;
use crate::domain::manifest::service::selects;
use crate::domain::manifest::types::{Fix, Output, Pathspec, ProductRoot, Running, Tool};
use crate::domain::operations::constants::LOCK_SEGMENT;
use crate::domain::operations::types::{Operation, Record, Revision};
use crate::domain::perf::service::{judge, rewritten};
use crate::domain::perf::types::{Measurement, PerfOutcome, Rewriting};
use crate::domain::renames::types::Head;
use crate::domain::reports::service::{
    completed, failed, measured, range_ends, recorded, stamp_parts, unlisted, unread,
};
use crate::domain::reports::types::{
    DiagnosticText, Digest, FailureOutput, HashParts, Keeping, Listed, NamedInput, Report, Scanned,
    Stamped,
};
use crate::domain::violations::service::{anchor, normalize, relative, within};
use crate::domain::violations::types::{
    Fingerprinted, Located, RepoPath, ToolId, Violation, Within,
};
use crate::outbound::bmf::types::BmfText;
use crate::outbound::check_store::types::{KeptReport, Stored};
use crate::outbound::digest::types::DigestParts;
use crate::outbound::sarif::types::{Reported, SarifText};
use crate::outbound::tools::types::{Completion, Ending, Invocation};

/// `check`, over the ports the edge injects.
#[derive(Debug)]
pub struct CheckUseCaseImpl {
    /// The outside `check` reaches: the manifest, tools, readers, lock, sources, cache and git.
    ports: CheckPorts,
}

impl CheckUseCaseImpl {
    /// `check` over the ports the edge injects.
    #[must_use]
    pub const fn new(ports: CheckPorts) -> Self {
        Self { ports }
    }

    /// Every input of `tools`, listed once for each pathspec and stamped once for each file, the lock left out.
    fn listed(&self, tools: &[Tool], stored: &mut Stored) -> Listed {
        let specs: BTreeSet<&Pathspec> = tools.iter().flat_map(|tool| &tool.inputs).collect();
        let asked: Vec<Pathspec> = specs.iter().map(|spec| (*spec).clone()).collect();
        let ignored = self.ports.repository.ignored(&asked).ok(); // ignore: idiomatic.arch/no-ok-on-result - inputs that cannot be listed leave the tool unstamped, so it runs again and is never served from the cache
        let files: BTreeMap<Pathspec, Option<Vec<RepoPath>>> = specs
            .into_iter()
            .map(|spec| {
                let found = self
                    .ports
                    .repository
                    .files(std::slice::from_ref(spec))
                    .ok() // ignore: idiomatic.arch/no-ok-on-result - inputs that cannot be listed leave the tool unstamped, so it runs again and is never served from the cache
                    .zip(ignored.as_ref());
                let found = found.map(|(found, ignoring)| {
                    let beneath = ignoring.0.get(spec).into_iter().flatten().cloned();
                    let found = found.0.into_iter().chain(beneath);
                    found
                        .filter(|file| !file.0.split('/').any(|part| part == LOCK_SEGMENT))
                        .collect()
                });
                (spec.clone(), found)
            })
            .collect();
        let all: BTreeSet<&RepoPath> = files.values().flatten().flatten().collect();
        let all: Vec<RepoPath> = all.into_iter().cloned().collect();
        // A file is read only when its record cannot vouch for it.
        let stats = self.ports.sources.stats(&all);
        let read = self
            .ports
            .sources
            .digests(&unread(&stored.recorded, &stats));
        stored.recorded = recorded(&stored.recorded, &stats, &read.0);
        let stamps = stored
            .recorded
            .files
            .iter()
            .map(|(path, record)| (path.clone(), record.digest.clone()))
            .collect();
        Listed { files, stamps }
    }

    /// What each pathspec of `listed` comes to: one digest over its files' paths and stamps, none where it could not
    /// be listed. It is digested from the files' stamps, so it reads no file.
    fn named(&self, listed: &Listed) -> BTreeMap<Pathspec, Option<Digest>> {
        let hash = |parts: HashParts<'_>| self.ports.digest.digest(DigestParts(&parts.0));
        listed
            .files
            .iter()
            .map(|(spec, files)| {
                let digest = files
                    .as_ref()
                    .map(|files| named(files, &listed.stamps, &hash));
                (spec.clone(), digest)
            })
            .collect()
    }

    /// What `tool`'s inputs come to now: its command line, the digest of what each of its pathspecs names, and the
    /// commits of the git range its command line reads; none when its inputs could not be listed.
    fn stamp(&self, tool: &Tool, named: &BTreeMap<Pathspec, Option<Digest>>) -> Stamped {
        let commits: Vec<Revision> = range_ends(tool)
            .iter()
            .filter_map(|end| self.ports.repository.commit(end).ok()) // ignore: idiomatic.arch/no-ok-on-result - a range end that resolves to no commit adds no commit to the stamp; a changed answer later changes the stamp
            .collect();
        stamp_parts(tool, named, &commits).map_or(Stamped::Unstamped, |parts| {
            Stamped::Digested(Digest(self.ports.digest.digest(DigestParts(&parts.0)).0))
        })
    }

    /// `reported` as a violation of `tool`: its path relative to the root, its fingerprint its tool's own or that of
    /// the line it is on.
    fn violation(
        &self,
        root: &ProductRoot,
        tool: &Tool,
        reported: Reported,
        read: &mut BTreeMap<RepoPath, Option<ReadFile>>,
    ) -> Violation {
        let path = relative(&reported.path, &normalize(&RepoPath(root.0.clone())));
        // A path that leads outside the product through a link is named by where it leads, so `up` refuses it.
        let path = match self.ports.sources.locate(&path) {
            Located::Outside(target) if within(&path) == Within::Inside => target,
            Located::Outside(_) | Located::Inside => path,
        };
        let fingerprint = match reported.fingerprint {
            Fingerprinted::Known(print) => Fingerprinted::Known(print),
            Fingerprinted::Unknown => read
                .entry(path.clone())
                .or_insert_with(|| {
                    // ignore: idiomatic.arch/no-ok-on-result - a source that cannot be read leaves its finding's fingerprint unknown, which the lock matches by place
                    self.ports.sources.read(&path).ok().map(|text| ReadFile {
                        lines: self.ports.digest.lines(&text),
                    })
                })
                .as_ref()
                .and_then(|file| {
                    let index = usize::try_from(anchor(reported.place).0)
                        .ok()? // ignore: idiomatic.arch/no-ok-on-result - a line number past what usize holds is no line of the file, so the finding has no line to fingerprint
                        .checked_sub(1)?;
                    file.lines.0.get(index).copied()
                })
                .map_or(reported.supplied, Fingerprinted::Known),
        };
        Violation {
            path,
            place: reported.place,
            tool: tool.id.clone(),
            rule: reported.rule,
            level: reported.level,
            suppression: reported.suppression,
            message: reported.message,
            fingerprint,
        }
    }

    /// Runs `tool` and reads what it says.
    fn scan(
        &self,
        root: &ProductRoot,
        tool: &Tool,
        stamp: Stamped,
        inputs: Vec<NamedInput>,
        read: &mut BTreeMap<RepoPath, Option<ReadFile>>,
    ) -> Scanned {
        let completion = self.ports.tools.run(&Invocation {
            argv: tool.run.clone(),
            directory: root.clone(),
            timeout: tool.timeout,
        });
        let Completion {
            ending,
            stdout,
            stderr,
        } = completion;
        let failed = |failure| Scanned {
            failure_output: FailureOutput::Captured {
                stdout: DiagnosticText(stdout.0.clone()),
                stderr: DiagnosticText(stderr.0.clone()),
                inputs: stamp.clone(),
            },
            ..failed(&tool.id, failure)
        };
        let code = match ending {
            Ending::Exited(code) => code.0,
            Ending::Signalled => {
                return failed(Failure::Signalled {
                    stderr: tail(&Tail(stderr.0.clone())),
                });
            }
            Ending::TimedOut => {
                return failed(Failure::TimedOut {
                    after: tool.timeout,
                });
            }
            Ending::Unstartable(problem) => {
                return failed(Failure::Unstartable(Problem(problem.0)));
            }
        };
        let exited = Failure::Exited {
            code: Code(code),
            stderr: tail(&Tail(stderr.0.clone())),
        };
        let mut measurements: Vec<Measurement> = Vec::new();
        let (violations, inventory) = match tool.output {
            Output::ExitStatus if code == 0_i32 => (Vec::new(), Inventory::Undeclared),
            Output::Bmf if code == 0_i32 => match self.ports.bmf.read(&BmfText(stdout.0.clone())) {
                Ok(measured) => {
                    measurements = measured.0;
                    (Vec::new(), Inventory::Undeclared)
                }
                Err(error) => {
                    return failed(Failure::Unreadable {
                        problem: Problem(error.to_string().into()),
                        stderr: tail(&Tail(stderr.0.clone())),
                    });
                }
            },
            Output::Sarif if code < 2_i32 => {
                match self.ports.sarif.read(&SarifText(stdout.0.clone())) {
                    Ok(log) => {
                        let violations = log
                            .results
                            .into_iter()
                            .map(|reported| self.violation(root, tool, reported, read))
                            .collect();
                        (violations, log.inventory)
                    }
                    Err(error) => {
                        return failed(Failure::Unreadable {
                            problem: Problem(error.to_string().into()),
                            stderr: tail(&Tail(stderr.0.clone())),
                        });
                    }
                }
            }
            Output::ExitStatus | Output::Sarif | Output::Bmf => return failed(exited),
        };
        completed(&tool.id, violations, inventory, measurements, stamp, inputs)
    }

    /// Select the requested comparison and ask the accepted-state operation to prepare it.
    fn accepted(&self, head: &Head, against: &Against) -> Result<Vec<Record>, CheckError> {
        match against {
            Against::Lock => Ok(self.ports.accepted.current(head)?),
            Against::Since(revision) => {
                let Head::Commit(head) = head else {
                    return Err(CheckError::NoHead);
                };
                Ok(self.ports.accepted.since(head, revision)?)
            }
        }
    }

    /// Runs each of `tools`' fix commands first, when asked to, and answers the fixes that did not succeed: the check
    /// runs on over what they left, and still reports what stands.
    fn fix(&self, root: &ProductRoot, tools: &[Tool], fixing: Fixing) -> Vec<FailedFix> {
        tools
            .iter()
            .filter(|_| fixing == Fixing::Fix)
            .filter_map(|tool| {
                let Fix::Command(argv) = &tool.fix else {
                    return None;
                };
                let completion = self.ports.tools.run(&Invocation {
                    argv: argv.clone(),
                    directory: root.clone(),
                    timeout: tool.timeout,
                });
                let stderr = tail(&Tail(completion.stderr.0));
                let failure = match completion.ending {
                    Ending::Exited(code) if code.0 == 0_i32 => return None,
                    Ending::Exited(code) => Failure::Exited {
                        code: Code(code.0),
                        stderr,
                    },
                    Ending::Signalled => Failure::Signalled { stderr },
                    Ending::TimedOut => Failure::TimedOut {
                        after: tool.timeout,
                    },
                    Ending::Unstartable(problem) => Failure::Unstartable(Problem(problem.0)),
                };
                Some(FailedFix {
                    tool: tool.id.clone(),
                    failure,
                })
            })
            .collect()
    }

    /// Judges what `scanned` measured against the lock's perf records of the tools not `excluded`, and writes the
    /// records an improvement rewrites in one operation: the one place `check` writes the lock, and only a plain
    /// check writes it. A check that fixes, or that judges against a revision, leaves the lock as it is.
    fn perf(
        &self,
        scanned: &[Scanned],
        runs: &[Run],
        excluded: &[ToolId],
        request: &CheckRequest,
    ) -> Result<PerfOutcome, CheckError> {
        let folded = self.ports.accepted.folded()?;
        let ran: Vec<ToolId> = runs
            .iter()
            .filter(|run| run.status == Status::Completed)
            .map(|run| run.tool.clone())
            .collect();
        let records: Vec<_> = folded
            .perf
            .iter()
            .filter(|record| !excluded.contains(&record.key.tool))
            .cloned()
            .collect();
        let mut perf = judge(&measured(scanned), &records, &ran);
        let improved = rewritten(&perf);
        if request.fixing == Fixing::Fix || matches!(request.against, Against::Since(_)) {
            perf.rewriting = Rewriting::Withheld;
        } else if improved.is_empty() {
            // Nothing improved: the lock already holds every best.
        } else {
            self.ports.lock.append(&Operation {
                parents: folded.heads.clone(),
                covers: Vec::new(),
                accepted: Vec::new(),
                removed: Vec::new(),
                perf: improved,
                perf_removed: Vec::new(),
            })?;
        }
        Ok(perf)
    }
}

impl CheckUseCase for CheckUseCaseImpl {
    fn check(&self, request: CheckRequest) -> Result<CheckReport, CheckError> {
        let root = self.ports.manifest.find(&request.start)?;
        let manifest = self.ports.manifest.read(&root)?;
        let tools: Vec<Tool> = manifest
            .tools
            .iter()
            .filter(|tool| selects(&request.groups, &tool.group) == Running::Runs)
            .cloned()
            .collect();
        let excluded: Vec<ToolId> = manifest
            .tools
            .iter()
            .filter(|tool| selects(&request.groups, &tool.group) == Running::Skips)
            .map(|tool| tool.id.clone())
            .collect();
        let failed_fixes = self.fix(&root, &tools, request.fixing);
        let mut stored = self.ports.store.load();
        let head = self.ports.repository.head()?;
        let mut read = BTreeMap::<RepoPath, Option<ReadFile>>::new();
        let listed = self.listed(&tools, &mut stored);
        let named = self.named(&listed);
        let mut scanned = Vec::with_capacity(tools.len());
        let mut reuses = Vec::with_capacity(tools.len());
        for tool in &tools {
            let stamp = self.stamp(tool, &named);
            let kept = match (&stamp, &stored.report) {
                (Stamped::Digested(stamp), KeptReport::Kept(report)) => {
                    crate::domain::reports::service::reusable(report, &tool.id, stamp)
                }
                (Stamped::Unstamped, _) | (_, KeptReport::Nothing) => None,
            };
            let scan = if let Some(scan) = kept {
                reuses.push(Reuse::Reused);
                Scanned {
                    keeping: Keeping::Current,
                    ..scan.clone()
                }
            } else {
                reuses.push(Reuse::Ran);
                let inputs = tool
                    .inputs
                    .iter()
                    .filter_map(|spec| {
                        let digest = named.get(spec)?.clone()?;
                        Some(NamedInput {
                            spec: spec.clone(),
                            digest,
                        })
                    })
                    .collect();
                self.scan(&root, tool, stamp, inputs, &mut read)
            };
            scanned.push(scan);
        }
        let violations: Vec<Violation> = scanned
            .iter()
            .flat_map(|scanned| scanned.violations.iter().cloned())
            .collect();
        let runs: Vec<Run> = scanned.iter().map(|scanned| scanned.run.clone()).collect();
        let answered = super::service::answered(&scanned, reuses);
        let mut accepted = self.accepted(&head, &request.against)?;
        accepted.retain(|record| !excluded.contains(&record.key.tool));
        // The files the judgement rests on beyond the inputs, and the manifest, are recorded beside them, so `up` and
        // `down` refuse when any moved.
        let mut touched = unlisted(&listed, &violations, &accepted);
        touched.push(RepoPath(MANIFEST.into()));
        let stats = self.ports.sources.stats(&touched);
        let digested = self
            .ports
            .sources
            .digests(&unread(&stored.recorded, &stats));
        let judged = recorded(&stored.recorded, &stats, &digested.0);
        stored.recorded.files.extend(judged.files);
        stored.recorded.taken = stored.recorded.taken.min(judged.taken);
        let outcome = compare(&violations, &accepted, &runs);
        let perf = self.perf(&scanned, &runs, &excluded, &request)?;
        let verdict = with_perf(verdict(&runs, &outcome), &perf);
        // A failed tool's run is kept, so `up` and `down` see the failure; the run it replaces is kept beside it, so the
        // answer of the last completed run is still reusable.
        let kept = super::service::keeping(&stored.report, &manifest.tools, &tools, scanned);
        stored.report = KeptReport::Kept(std::sync::Arc::new(Report {
            head,
            scanned: kept,
        }));
        // A store that cannot be written, a read-only cache among them, costs the next check its reuse, never this one its
        // answer.
        let _: Option<()> = self.ports.store.keep(&stored).ok(); // ignore: idiomatic.arch/no-ok-on-result - a check whose store cannot be saved still answers; the next one runs every tool
        Ok(CheckReport {
            root,
            runs: answered,
            outcome,
            verdict,
            against: request.against,
            perf,
            failed_fixes,
        })
    }
}
