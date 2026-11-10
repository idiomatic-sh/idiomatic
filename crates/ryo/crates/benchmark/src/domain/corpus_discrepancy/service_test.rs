//! Corpus-level accepted differences: pinned by finding, for one corpus digest, and stale ones fail.
use super::errors::CorpusDiscrepancyError;
use super::service::{admit_accepted, corpus_digest, corpus_problems, skip_problems};
use super::types::ObservedDifferences;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::reports::types::AcceptanceReason;
use ryo_benchmark_contracts::domain::reports::types::{
    AcceptedDifferences, AcceptedRule, DifferenceSide, FindingCount, FindingPosition,
    LocatedDifference, SourceColumn, SourceLine,
};
use ryo_benchmark_contracts::domain::requests::types::ContextSeal;
use std::collections::BTreeMap;

/// What a case hands the part.
enum Check {
    /// The corpus digest of one file list and context.
    Digest(ContextSeal),
    /// The problems of observed differences under accepted differences written for `corpus`.
    Judge {
        observed: Vec<ObservedDifferences>,
        accepted: Vec<AcceptedRule>,
        corpus: ArtifactDigest,
    },
    /// The problems of the candidate's one skipped row against a reference.
    Skips(Reference),
}

/// What the reference says about the file the candidate skipped a rule on.
#[derive(Clone, Copy)]
enum Reference {
    /// It skipped the same rule there.
    Skipped,
    /// It accounts no execution and rejects the file's syntax.
    Rejects,
    /// It accounts no execution and reports nothing there.
    Silent,
    /// It accounts its execution completely and rejects the file, but skipped nothing.
    CompleteRejects,
}

type Input = Check;
type Value = &'static [&'static str];
type Failure = CorpusDiscrepancyError;

fn rule(name: &str) -> RuleId {
    RuleId::try_from(name).expect("fixture rule")
}

fn at(side: DifferenceSide, line: usize, column: usize) -> LocatedDifference {
    LocatedDifference {
        side,
        file: RelativePath("case.py".into()),
        start: FindingPosition {
            line: SourceLine(line),
            column: SourceColumn(column),
        },
    }
}

fn observed(
    name: &str,
    differences: Vec<LocatedDifference>,
    end_differences: usize,
) -> ObservedDifferences {
    ObservedDifferences {
        rule: rule(name),
        differences,
        end_differences: FindingCount(end_differences),
    }
}

fn accepted(name: &str, findings: Vec<LocatedDifference>) -> AcceptedRule {
    AcceptedRule {
        rule: rule(name),
        reason: AcceptanceReason("why".into()),
        findings,
    }
}

fn files() -> Vec<RelativePath> {
    vec![RelativePath("café.py".into())]
}

fn sealed(boundary: &str) -> ContextSeal {
    ContextSeal::Sealed(ContextManifest {
        boundary: boundary.into(),
        logical_root: ".".into(),
        auxiliary_files: BTreeMap::from([("setup.cfg".into(), ArtifactDigest("b".repeat(64)))]),
        source_files: BTreeMap::from([("café.py".into(), ArtifactDigest("a".repeat(64)))]),
        missing: vec!["__init__.py".into()],
    })
}

/// The digest of the sealed fixture corpus, as `json.dumps(sort_keys=True, separators=(",", ":"), ensure_ascii=False)` spells it.
const SEALED: &str = "97cb4ed58227a9bbc5a452fef45492814061f5def35c80060e964fdc681060bb";

fn judge(observed: Vec<ObservedDifferences>, accepted: Vec<AcceptedRule>) -> Check {
    Check::Judge {
        observed,
        accepted,
        corpus: corpus_digest(&files(), &sealed("/corpus")),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a row both contestants skipped agrees"),
        word: Word::Succeeds,
        input: || Check::Skips(Reference::Skipped),
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName(
            "a skip agrees with a reference that has no execution accounting and rejects the file",
        ),
        word: Word::Succeeds,
        input: || Check::Skips(Reference::Rejects),
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("a skip the reference does not explain is a problem"),
        word: Word::Succeeds,
        input: || Check::Skips(Reference::Silent),
        expect: Expect::Value(&["F401: only the candidate skipped bad.py (syntax-rejected)"]),
    },
    Case {
        name: CaseName("a reference that accounts its execution must skip the row itself"),
        word: Word::Succeeds,
        input: || Check::Skips(Reference::CompleteRejects),
        expect: Expect::Value(&["F401: only the candidate skipped bad.py (syntax-rejected)"]),
    },
    Case {
        name: CaseName("a sealed corpus digest is its files and context entries, not its location"),
        word: Word::Succeeds,
        input: || Check::Digest(sealed("/elsewhere")),
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("only exactly pinned findings pass"),
        word: Word::Succeeds,
        input: || {
            judge(
                vec![
                    observed("B020", vec![at(DifferenceSide::Extra, 2, 5)], 0),
                    observed("F401", Vec::new(), 0),
                ],
                vec![accepted("B020", vec![at(DifferenceSide::Extra, 2, 5)])],
            )
        },
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("an unnamed finding and a repeat of a named one are not accepted"),
        word: Word::Succeeds,
        input: || {
            judge(
                vec![observed(
                    "B020",
                    vec![
                        at(DifferenceSide::Extra, 2, 5),
                        at(DifferenceSide::Extra, 2, 5),
                        at(DifferenceSide::Missing, 1, 1),
                    ],
                    0,
                )],
                vec![accepted("B020", vec![at(DifferenceSide::Extra, 2, 5)])],
            )
        },
        expect: Expect::Value(&[
            "B020: extra case.py:2:5 not accepted",
            "B020: missing case.py:1:1 not accepted",
        ]),
    },
    Case {
        name: CaseName("a same-count difference at another location fails"),
        word: Word::Succeeds,
        input: || {
            judge(
                vec![observed("B020", vec![at(DifferenceSide::Extra, 3, 1)], 0)],
                vec![accepted("B020", vec![at(DifferenceSide::Extra, 2, 5)])],
            )
        },
        expect: Expect::Value(&[
            "B020: extra case.py:3:1 not accepted",
            "B020: accepted extra case.py:2:5 no longer occurs",
        ]),
    },
    Case {
        name: CaseName("stale and unselected acceptances and end differences fail"),
        word: Word::Succeeds,
        input: || {
            judge(
                vec![
                    observed("B020", Vec::new(), 1),
                    observed("F401", Vec::new(), 0),
                ],
                vec![
                    accepted("F401", vec![at(DifferenceSide::Missing, 1, 5)]),
                    accepted("UP032", vec![at(DifferenceSide::Missing, 1, 1)]),
                ],
            )
        },
        expect: Expect::Value(&[
            "B020: 1 end positions differ",
            "F401: accepted missing case.py:1:5 no longer occurs",
            "UP032: accepted but not selected",
        ]),
    },
    Case {
        name: CaseName("a rule named twice is refused"),
        word: Word::Refuses,
        input: || {
            judge(
                Vec::new(),
                vec![
                    accepted("B020", vec![at(DifferenceSide::Extra, 9, 9)]),
                    accepted("B020", vec![at(DifferenceSide::Extra, 2, 5)]),
                ],
            )
        },
        expect: Expect::Error(CorpusDiscrepancyError::RuleNamedTwice(String::new())),
    },
    Case {
        name: CaseName("a finding named twice is refused"),
        word: Word::Refuses,
        input: || {
            judge(
                Vec::new(),
                vec![accepted(
                    "B020",
                    vec![
                        at(DifferenceSide::Extra, 2, 5),
                        at(DifferenceSide::Extra, 2, 5),
                    ],
                )],
            )
        },
        expect: Expect::Error(CorpusDiscrepancyError::AcceptanceIncomplete(String::new())),
    },
    Case {
        name: CaseName("acceptances written for another corpus are refused"),
        word: Word::Refuses,
        input: || Check::Judge {
            observed: Vec::new(),
            accepted: Vec::new(),
            corpus: corpus_digest(&files(), &ContextSeal::Unsealed),
        },
        expect: Expect::Error(CorpusDiscrepancyError::CorpusDiffers {
            accepted: String::new(),
            actual: String::new(),
        }),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Check::Skips(reference) => Ok(skips(reference)),
        Check::Digest(context) => {
            assert_eq!(
                corpus_digest(&files(), &context).0,
                SEALED,
                "digest of the fixture corpus"
            );
            assert_ne!(
                corpus_digest(&files(), &ContextSeal::Unsealed).0,
                SEALED,
                "an unsealed corpus is its file list alone"
            );
            Ok(&[])
        }
        Check::Judge {
            observed,
            accepted,
            corpus,
        } => {
            let written = AcceptedDifferences {
                corpus,
                rules: accepted,
            };
            let admitted = admit_accepted(&written, &corpus_digest(&files(), &sealed("/corpus")))
                .map_err(|error| match error {
                CorpusDiscrepancyError::RuleNamedTwice(_) => {
                    CorpusDiscrepancyError::RuleNamedTwice(String::new())
                }
                CorpusDiscrepancyError::AcceptanceIncomplete(_) => {
                    CorpusDiscrepancyError::AcceptanceIncomplete(String::new())
                }
                CorpusDiscrepancyError::CorpusDiffers { .. } => {
                    CorpusDiscrepancyError::CorpusDiffers {
                        accepted: String::new(),
                        actual: String::new(),
                    }
                }
            })?;
            let problems: Vec<&'static str> = corpus_problems(&observed, &admitted)
                .into_iter()
                .map(|problem| &*Box::leak(problem.0.into_boxed_str()))
                .collect();
            Ok(Box::leak(problems.into_boxed_slice()))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The problems of a candidate that skipped F401 on `bad.py`, against the reference `reference` describes.
fn skips(reference: Reference) -> Value {
    use ryo_benchmark_contracts::domain::reports::types::{
        BenchmarkFinding, CoverageAccounting, ExecutionMetadata, ProjectContext, RunReport,
        SourceByteOffset, UnavailableReason,
    };
    let syntax = rule("invalid-syntax");
    let skip = ryo::linter::Skipped {
        rule: rule("F401"),
        path: RelativePath("bad.py".into()),
        reason: ryo::linter::SkipReason::SyntaxRejected,
    };
    let report = |skipped: Vec<ryo::linter::Skipped>, diagnostics, execution_metadata| RunReport {
        execution_metadata,
        project_context: ProjectContext::NotRequested,
        diagnostics,
        skipped,
        coverage: CoverageAccounting::Unaccounted,
        files: vec![RelativePath("bad.py".into())],
    };
    let rejects = vec![BenchmarkFinding {
        rule: syntax.clone(),
        file: RelativePath("bad.py".into()),
        start: SourceByteOffset(0),
        end: SourceByteOffset(1),
    }];
    let unavailable = || ExecutionMetadata::Unavailable {
        reason: UnavailableReason("findings only".into()),
    };
    let candidate = report(
        vec![skip.clone()],
        rejects.clone(),
        ExecutionMetadata::Complete,
    );
    let reference = match reference {
        Reference::Skipped => report(vec![skip], Vec::new(), ExecutionMetadata::Complete),
        Reference::Rejects => report(Vec::new(), rejects, unavailable()),
        Reference::Silent => report(Vec::new(), Vec::new(), unavailable()),
        Reference::CompleteRejects => report(Vec::new(), rejects, ExecutionMetadata::Complete),
    };
    let problems: Vec<&'static str> = skip_problems(&reference, &candidate, &syntax)
        .into_iter()
        .map(|problem| &*Box::leak(problem.0.into_boxed_str()))
        .collect();
    Box::leak(problems.into_boxed_slice())
}
