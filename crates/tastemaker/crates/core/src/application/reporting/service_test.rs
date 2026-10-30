// ignore: idiomatic.arch/file-length - the report operation is exercised with the same injected port harness across template mismatches, imports, suppressions, fingerprints and late source failures; keeping the case setup and observations together makes substitutions auditable.
//! What the `reporting` service makes of the lint call's answer, on values alone.

use std::collections::BTreeSet;

use super::types::SourceFiles;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::fixtures::matching;
use crate::domain::applications::types::Plan;
use crate::domain::findings::service::{error, whole_identity};
use crate::domain::findings::types::{Fingerprint, Identifying, Reason};
use crate::domain::governance::types::Answers;
use crate::domain::paths::types::RepoPath;
use crate::domain::reporting::service::fingerprinted;
use crate::domain::reporting::types::{Candidate, Reported, Reuse, Standing};

/// `report` with the chain of what files had before their ignores hashed by the length of its parts: findings that differ in
/// length differ in the chain.
fn report<E>(
    reported: Reported<'_>,
    read: &impl Fn(&[RepoPath]) -> Result<SourceFiles, E>,
    sets: &dyn crate::domain::selectors::types::SetCompiler,
    fingerprint: &dyn Fn(&Identifying) -> Fingerprint,
) -> Result<crate::domain::findings::types::Report, E> {
    super::service::report(reported, read, sets, fingerprint, &|identity| {
        Fingerprint(
            u64::try_from(identity.parts.0[0].len() * 31 + identity.parts.0[1].len()).unwrap_or(0),
        )
    })
}

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// A finding made before the lint call, with no answer: it stands, fingerprinted, and `ignore` is evaluated.
    Stands,
    /// The same, with the report limited to a directory it is not in.
    Outside,
    /// One finding an ignore accepts and one that stands.
    Accepted,
}

type Input = Scenario;
/// What the report holds, written out.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_finding_made_before_the_call_stands_fingerprinted"),
        word: Word::Succeeds,
        input: || Scenario::Stands,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_finding_outside_within_is_left_out"),
        word: Word::Succeeds,
        input: || Scenario::Outside,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_accepted_finding_is_suppressed_with_its_reason"),
        word: Word::Succeeds,
        input: || Scenario::Accepted,
        expect: Expect::Value(String::new()),
    },
];

fn seven(_: &Identifying) -> Fingerprint {
    Fingerprint(7)
}

/// What the scenario gave, and what it should give.
fn said(scenario: Scenario) -> (String, &'static str) {
    let plan = Plan::default();
    let finding = error(
        crate::domain::findings::service::check_id("p/r", "count"),
        RepoPath("src/lib.txt".into()),
        "wrong",
    );
    let within = match scenario {
        Scenario::Outside => vec![RepoPath("docs".into())],
        Scenario::Stands | Scenario::Accepted => Vec::new(),
    };
    match scenario {
        Scenario::Stands | Scenario::Outside => {
            let made = report(
                Reported {
                    reuse: &Reuse::default(),
                    plan: &plan,
                    answers: Answers::default(),
                    findings: vec![finding],
                    within: &within,
                },
                &|_| Ok::<_, std::convert::Infallible>(SourceFiles::default()),
                matching().sets,
                &seven,
            );
            let made = made.unwrap_or_else(|error| panic!("report failed: {error}"));
            let findings = made
                .findings
                .iter()
                .map(|each| {
                    format!(
                        "{}@{}#{}",
                        crate::domain::findings::service::printed(&each.rule).0,
                        each.path.0,
                        each.fingerprint.0
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let evaluated = made
                .evaluated
                .iter()
                .map(|rule| crate::domain::findings::service::printed(rule).0)
                .collect::<Vec<_>>()
                .join(",");
            (
                format!("{findings}|{evaluated}"),
                match scenario {
                    Scenario::Outside => "|tastemaker#ignore",
                    Scenario::Stands | Scenario::Accepted => {
                        "p/r#count@src/lib.txt#7|tastemaker#ignore"
                    }
                },
            )
        }
        Scenario::Accepted => accepted(finding),
    }
}

/// Observe both accepted and standing findings from the same suppression decision.
fn accepted(finding: crate::domain::findings::types::Finding) -> (String, &'static str) {
    source_lines_survive_settlement();
    let other = error(
        crate::domain::findings::service::check_id("p/s", "count"),
        RepoPath("src/lib.txt".into()),
        "also wrong",
    );
    let identified = vec![whole_identity(&finding), whole_identity(&other)];
    let made = fingerprinted(
        vec![
            Candidate {
                finding,
                line: crate::domain::findings::types::FirstLine::default(),
                acceptance: Standing::Accepted(Reason("why".into())),
            },
            Candidate {
                finding: other,
                line: crate::domain::findings::types::FirstLine::default(),
                acceptance: Standing::Stands,
            },
        ],
        identified,
        &seven,
    );
    (
        format!(
            "{}:{}|{}",
            made.accepted
                .iter()
                .map(|each| crate::domain::findings::service::printed(&each.finding.rule).0)
                .collect::<Vec<_>>()
                .join(","),
            made.accepted
                .iter()
                .map(|each| &*each.reason.0)
                .collect::<Vec<_>>()
                .join(","),
            made.standing
                .iter()
                .map(|each| crate::domain::findings::service::printed(&each.rule).0)
                .collect::<Vec<_>>()
                .join(","),
        ),
        "p/r#count:why|p/s#count",
    )
}

/// The comment the shared `comment` relation gives of `path`, answered to the comments rule `comments`: an ignore of
/// `p/r`, at the span of the whole comment.
fn ignoring_comment(
    comments: crate::domain::applications::types::LintRuleId,
    path: &RepoPath,
) -> Answers {
    Answers {
        answers: vec![crate::domain::applications::types::LintAnswer {
            rule: comments,
            path: path.clone(),
            cells: vec![
                cell(
                    "span",
                    crate::domain::applications::types::CellValue::Span(
                        crate::domain::comments::types::ByteSpan {
                            start: crate::domain::findings::types::ByteOffset(10),
                            end: crate::domain::findings::types::ByteOffset(37),
                        },
                    ),
                ),
                cell("doc", text_cell("false")),
                cell("text", text_cell(" ignore: p/r - valid value")),
            ],
        }],
        ..Answers::default()
    }
}

/// Suppression and first-line identity share source text without changing global order.
fn source_lines_survive_settlement() {
    use crate::domain::files::types::FileText;
    use crate::domain::findings::types::{Column, Line, Place, Region, RuleId};
    let rule = RuleId("p/r".into());
    let path = RepoPath("a.toml".into());
    let other = RepoPath("z.toml".into());
    let unplanned = RepoPath("outside.toml".into());
    let finding_at = |located: &RepoPath, line| {
        let mut finding = error(
            crate::domain::findings::service::of_rule(rule.clone()),
            located.clone(),
            "wrong",
        );
        finding.place = Place::At(Region {
            start_line: Line(line),
            start_column: Column(1),
            end_line: Line(line),
            end_column: Column(10),
        });
        finding
    };
    let comments = crate::domain::applications::types::LintRuleId(
        crate::domain::applications::constants::COMMENTS.into(),
    );
    let mut plan = Plan {
        known: BTreeSet::from([rule.clone()]),
        files: vec![path.clone(), other.clone()],
        ..Plan::default()
    };
    drop(plan.purposes.insert(
        comments.clone(),
        crate::domain::applications::types::Purpose::Comments,
    ));
    let answers = ignoring_comment(comments, &path);
    let made = report(
        Reported {
            reuse: &Reuse::default(),
            plan: &plan,
            answers,
            findings: vec![
                finding_at(&unplanned, 1),
                finding_at(&other, 1),
                finding_at(&path, 2),
                finding_at(&path, 1),
            ],
            within: &[],
        },
        &|_| {
            Ok::<_, std::convert::Infallible>(SourceFiles(
                [
                    (
                        unplanned.clone(),
                        "outside = 4 # ignore: p/r - not in plan\n",
                    ),
                    (
                        path.clone(),
                        "first = 1 # ignore: p/r - valid value\nsecond = 2\n",
                    ),
                    (other.clone(), "last = 3\n"),
                ]
                .into_iter()
                .map(|(at, text)| (at, super::service::source(FileText(text.into()))))
                .collect(),
            ))
        },
        matching().sets,
        &|identity| match &*identity.parts.0[2] {
            "first = 1 # ignore: p/r - valid value" => Fingerprint(11),
            "second = 2" => Fingerprint(22),
            "last = 3" => Fingerprint(33),
            "outside = 4 # ignore: p/r - not in plan" => Fingerprint(44),
            text => panic!("unexpected source identity: {text}"),
        },
    )
    .unwrap_or_else(|error| panic!("report failed: {error}"));
    assert_eq!(made.suppressed.len(), 1);
    assert_eq!(made.suppressed[0].finding.fingerprint, Fingerprint(11));
    assert_eq!(made.suppressed[0].reason, Reason("valid value".into()));
    assert_eq!(
        made.findings
            .iter()
            .map(|finding| (&finding.path, finding.fingerprint))
            .collect::<Vec<_>>(),
        vec![
            (&path, Fingerprint(22)),
            (&unplanned, Fingerprint(44)),
            (&other, Fingerprint(33))
        ],
        "cross-file order and standing identities survive per-file settlement"
    );
}

/// What the report said for `scenario`: nothing when it said what the row wants, else what it said.
fn subject(scenario: Input) -> Value {
    let (said, wanted) = said(scenario);
    if said == wanted { String::new() } else { said }
}

#[test]
fn succeeds() {
    name_only_import_is_found_at_its_target();
    a_file_that_has_what_it_had_is_not_read_again();
    an_allow_pattern_that_does_not_compile_is_refused();
    injected_sources_change_identity();
    run(CASES, Word::Succeeds, |scenario| {
        Ok::<_, Failure>(subject(scenario))
    });
}

/// A file whose comment holds an ignore of a finding it has is read, and its report says what it had before the ignore. A report told
/// that the file has that still, with the findings it came to, does not read it, and says the same; a file that has more is read.
fn a_file_that_has_what_it_had_is_not_read_again() {
    use crate::domain::files::types::FileText;
    use crate::domain::findings::types::{Column, Line, Place, Region, RuleId};
    let rule = RuleId("p/r".into());
    let path = RepoPath("a.toml".into());
    let comments = crate::domain::applications::types::LintRuleId(
        crate::domain::applications::constants::COMMENTS.into(),
    );
    let mut plan = Plan {
        known: BTreeSet::from([rule.clone()]),
        files: vec![path.clone()],
        ..Plan::default()
    };
    drop(plan.purposes.insert(
        comments.clone(),
        crate::domain::applications::types::Purpose::Comments,
    ));
    let finding_at = |line| {
        let mut finding = error(
            crate::domain::findings::service::of_rule(rule.clone()),
            path.clone(),
            "wrong",
        );
        finding.place = Place::At(Region {
            start_line: Line(line),
            start_column: Column(1),
            end_line: Line(line),
            end_column: Column(10),
        });
        finding
    };
    let reads = std::cell::RefCell::new(Vec::new());
    let report_with = |reuse: &Reuse, findings: Vec<crate::domain::findings::types::Finding>| {
        reads.borrow_mut().clear();
        report(
            Reported {
                plan: &plan,
                answers: ignoring_comment(comments.clone(), &path),
                findings,
                within: &[],
                reuse,
            },
            &|wanted: &[RepoPath]| {
                reads.borrow_mut().extend(wanted.iter().cloned());
                Ok::<_, std::convert::Infallible>(SourceFiles(
                    wanted
                        .iter()
                        .map(|at| {
                            (
                                at.clone(),
                                super::service::source(FileText(
                                    "first = 1 # ignore: p/r - valid value\nsecond = 2\n".into(),
                                )),
                            )
                        })
                        .collect(),
                ))
            },
            matching().sets,
            &|identity| Fingerprint(u64::try_from(identity.parts.0[1].len()).unwrap_or(0) + 100),
        )
        .unwrap_or_else(|error| panic!("report failed: {error}"))
    };
    let first = report_with(&Reuse::default(), vec![finding_at(1)]);
    assert_eq!(
        *reads.borrow(),
        vec![path.clone()],
        "a file with an ignore is read"
    );
    assert_eq!(first.suppressed.len(), 1, "its ignore accepts its finding");
    let had = Reuse {
        incoming: first.sourced.clone(),
        findings: first.findings.clone(),
        suppressed: first.suppressed.clone(),
    };
    let again = report_with(&had, vec![finding_at(1)]);
    assert!(
        reads.borrow().is_empty(),
        "a file that has what it had is not read again"
    );
    assert_eq!(
        again.suppressed, first.suppressed,
        "and its findings are those kept"
    );
    assert_eq!(again.sourced, first.sourced, "and has what it had");
    let more = report_with(&had, vec![finding_at(1), finding_at(2)]);
    assert_eq!(*reads.borrow(), vec![path], "a file that has more is read");
    assert_eq!(more.findings.len(), 1, "and its new finding stands");
}

/// A name-only import, which has no dependency edge or comment row to request its source for it, is found at its target
/// and fingerprinted from its line.
fn name_only_import_is_found_at_its_target() {
    use crate::domain::applications::types::{LintAnswer, LintRuleId, Purpose};
    use crate::domain::entries::types::{EntryIdentity, ImportDefault};
    use crate::domain::files::types::FileText;
    use crate::domain::findings::types::{Column, Line, Place, Region};
    use crate::domain::imports::types::{
        EntryImports, InstanceName, Member, Relation, ScopeImports,
    };
    use crate::outbound::repository::interface::Repository as _;
    let repository = crate::outbound::repository::fixtures::fake::RepositoryFake::default();
    let path = RepoPath("src/only.txt".into());
    let text = FileText("use serde::Serialize;\n".into());
    let written = repository.write(&path, &text);
    assert_eq!(written.and_then(|()| repository.read(&path)), Ok(text));
    let rule = LintRuleId("imports".into());
    let mut plan = Plan::default();
    drop(
        plan.purposes
            .insert(rule.clone(), Purpose::Imports(Relation::Names)),
    );
    plan.imports.push(ScopeImports {
        checks: vec![EntryImports {
            identity: EntryIdentity("p/entry".into()),
            default: ImportDefault::Deny,
            allow: Vec::new(),
            members: vec![Member {
                instance: InstanceName("only".into()),
                files: BTreeSet::from([path.clone()]),
                extra: Vec::new(),
            }],
        }],
        ..ScopeImports::default()
    });
    let made = report(
        Reported {
            reuse: &Reuse::default(),
            plan: &plan,
            findings: Vec::new(),
            within: &[],
            answers: Answers {
                answers: vec![LintAnswer {
                    rule,
                    path,
                    cells: vec![
                        cell("target", text_cell("serde::Serialize")),
                        cell(
                            "span",
                            crate::domain::applications::types::CellValue::Span(
                                crate::domain::comments::types::ByteSpan {
                                    start: crate::domain::findings::types::ByteOffset(4),
                                    end: crate::domain::findings::types::ByteOffset(20),
                                },
                            ),
                        ),
                    ],
                }],
                ..Answers::default()
            },
        },
        &|paths| {
            Ok::<_, std::convert::Infallible>(SourceFiles(
                paths
                    .iter()
                    .filter_map(|listed| {
                        repository
                            .read(listed)
                            .ok()
                            .map(|loaded| (listed.clone(), super::service::source(loaded)))
                    })
                    .collect(),
            ))
        },
        matching().sets,
        &|identity| {
            assert_eq!(&*identity.parts.0[2], "use serde::Serialize;");
            Fingerprint(7)
        },
    );
    let made = made.unwrap_or_else(|error| panic!("report failed: {error}"));
    assert_eq!(made.findings.len(), 1);
    assert_eq!(
        made.findings[0].place,
        Place::At(Region {
            start_line: Line(1),
            start_column: Column(5),
            end_line: Line(1),
            end_column: Column(21)
        })
    );
    assert_eq!(made.findings[0].fingerprint, Fingerprint(7));
}

/// An `imports.allow` module pattern that does not compile is refused as a finding of its entry at the workspace's
/// `tastemaker.toml`, never read as a pattern that matches nothing.
fn an_allow_pattern_that_does_not_compile_is_refused() {
    use crate::domain::applications::types::{LintAnswer, LintRuleId, Purpose};
    use crate::domain::entries::types::{EntryIdentity, ImportDefault};
    use crate::domain::files::types::FileText;
    use crate::domain::imports::types::{
        EntryImports, InstanceName, Member, Relation, ScopeImports,
    };
    use crate::outbound::repository::interface::Repository as _;
    let repository = crate::outbound::repository::fixtures::fake::RepositoryFake::default();
    let path = RepoPath("src/only.txt".into());
    let text = FileText("use serde::Serialize;\n".into());
    let written = repository.write(&path, &text);
    assert_eq!(written.and_then(|()| repository.read(&path)), Ok(text));
    let rule = LintRuleId("imports".into());
    let mut plan = Plan::default();
    drop(
        plan.purposes
            .insert(rule.clone(), Purpose::Imports(Relation::Names)),
    );
    plan.imports.push(ScopeImports {
        checks: vec![EntryImports {
            identity: EntryIdentity("p/entry".into()),
            default: ImportDefault::Deny,
            allow: vec!["serde".into()],
            members: vec![Member {
                instance: InstanceName("only".into()),
                files: BTreeSet::from([path.clone()]),
                extra: Vec::new(),
            }],
        }],
        ..ScopeImports::default()
    });
    let made = report(
        Reported {
            reuse: &Reuse::default(),
            plan: &plan,
            findings: Vec::new(),
            within: &[],
            answers: Answers {
                answers: vec![LintAnswer {
                    rule,
                    path,
                    cells: vec![
                        cell("target", text_cell("serde::Serialize")),
                        cell(
                            "span",
                            crate::domain::applications::types::CellValue::Span(
                                crate::domain::comments::types::ByteSpan {
                                    start: crate::domain::findings::types::ByteOffset(4),
                                    end: crate::domain::findings::types::ByteOffset(20),
                                },
                            ),
                        ),
                    ],
                }],
                ..Answers::default()
            },
        },
        &|paths| {
            Ok::<_, std::convert::Infallible>(SourceFiles(
                paths
                    .iter()
                    .filter_map(|listed| {
                        repository
                            .read(listed)
                            .ok()
                            .map(|loaded| (listed.clone(), super::service::source(loaded)))
                    })
                    .collect(),
            ))
        },
        // A module pattern is names and `*` and always compiles with the real matching port, so a compiler that refuses
        // every set stands for one that does not.
        &|_: &[crate::domain::selectors::types::SetGlob]| -> Result<
            Arc<dyn crate::domain::selectors::types::PathSet>,
            crate::domain::selectors::types::PatternFailure,
        > {
            Err(crate::domain::selectors::types::PatternFailure(
                "refused".into(),
            ))
        },
        &|_| Fingerprint(7),
    );
    let made = made.unwrap_or_else(|error| panic!("report failed: {error}"));
    assert!(
        made.findings
            .iter()
            .any(
                |finding| &*crate::domain::findings::service::printed(&finding.rule).0
                    == "p/entry#imports"
                    && &*finding.path.0 == "tastemaker.toml"
                    && finding.message.0.contains("is no module pattern")
            ),
        "the pattern that does not compile is a finding: {:?}",
        made.findings
    );
}

use std::sync::Arc;

fn injected() -> super::service::ReportingService {
    super::service::ReportingService::new(
        Arc::new(crate::outbound::scheduler::fixtures::fake::SchedulerFake),
        Arc::new(crate::outbound::matching::factory::globset()),
        Arc::new(crate::outbound::digest::fixtures::fake::DigestFake),
    )
}

fn injected_sources_change_identity() {
    use crate::domain::findings::types::{Column, Line, Place, Region};
    let path = RepoPath("source.toml".into());
    let plan = Plan {
        files: vec![path.clone()],
        ..Plan::default()
    };
    let identity = |text| {
        let repository = Arc::new(
            crate::outbound::repository::fixtures::fake::RepositoryFake::of(&[(
                "source.toml",
                text,
            )]),
        );
        let service = injected();
        let mut finding = error(
            crate::domain::findings::service::of_rule(crate::domain::findings::types::RuleId(
                "test".into(),
            )),
            path.clone(),
            "finding",
        );
        finding.place = Place::At(Region {
            start_line: Line(1),
            end_line: Line(1),
            start_column: Column(1),
            end_column: Column(2),
        });
        let report = service
            .prepare(
                &*repository,
                Reported {
                    reuse: &Reuse::default(),
                    plan: &plan,
                    answers: Answers::default(),
                    findings: vec![finding],
                    within: &[],
                },
            )
            .unwrap_or_else(|error| panic!("prepare: {error}"));
        report.findings[0].fingerprint
    };
    assert_ne!(identity("a=1"), identity("long_name=42"));
}

/// A cell of a row the lint call answered.
fn cell(
    name: &str,
    value: crate::domain::applications::types::CellValue,
) -> crate::domain::applications::types::AnswerCell {
    crate::domain::applications::types::AnswerCell {
        name: crate::domain::definitions::types::ColumnName(name.into()),
        value,
    }
}

/// A cell's value that is text.
fn text_cell(value: &str) -> crate::domain::applications::types::CellValue {
    crate::domain::applications::types::CellValue::Text(
        crate::domain::applications::types::CellText(value.into()),
    )
}
