// ignore: idiomatic.arch/file-length - the ryo adapter's one case table holds its validations and both query families tm hosts relations for (the verdict and the counts) run through ryo itself, and the counts' rows again through the fake so it answers as ryo does; the rows share the adapter's settings and subject.
//! The ryo adapter of the analysis port: the contract with no language linked, then a validation it refuses, then the
//! verdict and count queries over the relations tm hosts.

use super::adapter::AnalysisRyoAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::types::QueryText;
use crate::domain::files::types::MachinePath;
use crate::outbound::analysis::fixtures::contract::check;
use crate::outbound::analysis::interface::AnalysisPort as _;
use crate::outbound::analysis::types::QueryVerdict;

/// A query, written out or one of Tastemaker's own.
#[derive(Clone, Copy, Debug)]
enum Query {
    /// This text.
    Text(&'static str),
    /// The built-in query reading the comments that may hold an ignore.
    Comments,
    /// The built-in queries reading the shared relations the contracts compare.
    Data,
    /// The catalog's listing and page of `tastemaker.placed`, Tastemaker's own relation a rule may query.
    PlacedCatalog,
    /// The worked queries `explain ryoql` shows.
    Worked,
}
/// Which port answers a row of the count queries: ryo itself, or the fake, which must answer as ryo does.
#[derive(Clone, Copy, Debug)]
enum Port {
    /// The ryo adapter.
    Ryo,
    /// The analysis port's fake.
    Fake,
}

/// What a row asks of the adapter: a query it validates, a scenario its verdict queries judge, or counts and pairs its
/// count queries judge.
#[derive(Clone, Copy, Debug)]
enum Row {
    /// A query it validates.
    Query(Query),
    /// A scenario the verdict queries judge.
    Judged(Judged),
    /// Counts and pairs the count queries judge, through this port.
    Counted(Counted, Port),
    /// The strays of closed directories the closed query decides, through this port.
    Closed(Port),
}

type Input = Row;
/// Whether the row holds.
type Value = bool;
/// Why the query is refused, or what the queries said that the row does not expect.
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_query_over_tastemaker_placed_validates"),
        word: Word::Succeeds,
        input: || {
            Row::Query(Query::Text(
                "let p = select * from \"tastemaker.placed\" where template = \"x\"; emit p subject \"path\" key(\"path\");",
            ))
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_built_in_comments_query_is_one_this_ryo_runs"),
        word: Word::Succeeds,
        input: || Row::Query(Query::Comments),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_built_in_data_queries_are_ones_this_ryo_runs"),
        word: Word::Succeeds,
        input: || Row::Query(Query::Data),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("tastemaker_placed_is_listed_and_described_with_its_six_columns"),
        word: Word::Succeeds,
        input: || Row::Query(Query::PlacedCatalog),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_worked_queries_explain_shows_validate"),
        word: Word::Succeeds,
        input: || Row::Query(Query::Worked),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_relation_ryo_has_not_is_refused"),
        word: Word::Refuses,
        input: || {
            Row::Query(Query::Text(
                "let p = select * from \"no-such-relation\"; emit p subject \"path\" key(\"path\");",
            ))
        },
        expect: Expect::Refusal(|reason| names(reason, "`tm list relations`")),
    },
    Case {
        name: CaseName("an_operation_ryo_has_not_is_refused_naming_the_operators"),
        word: Word::Refuses,
        input: || {
            Row::Query(Query::Text(
                "let p = frobnicate(from=\"file\"); emit p subject \"path\" key(\"path\");",
            ))
        },
        expect: Expect::Refusal(|reason| names(reason, "`tm list operators`")),
    },
    Case {
        name: CaseName("a_scenario_that_finds_what_it_expects_has_no_problem"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::Expected),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_finding_a_scenario_does_not_expect_is_a_problem"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::Unexpected),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_finding_of_a_rule_the_scenario_does_not_name_is_not_compared"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::OnlyOtherRule),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rules_scenario_writes_its_own_findings_without_the_rule"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::Own),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_finding_of_its_own_rule_a_rules_scenario_does_not_expect_is_a_problem"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::OwnUnexpected),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rules_scenario_whose_rule_reached_no_file_is_a_problem"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::OwnUnreached),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_rule_only_names_that_was_not_evaluated_is_a_problem"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::OnlyUnevaluated),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("only_names_a_rule_by_its_whole_id_after_a_slash_or_a_hash"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::OnlyPartialName),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "an_expected_check_matches_the_end_of_the_found_id_and_a_whole_file_reads_with_no_line",
        ),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::CheckOfWholeFile),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_finding_on_another_line_is_two_problems"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::OtherLine),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_finding_found_twice_and_expected_once_is_one_problem"),
        word: Word::Succeeds,
        input: || Row::Judged(Judged::Twice),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_condition_that_holds_requires_its_then"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Holds, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_condition_that_fails_requires_its_else"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Fails, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_count_whose_condition_is_unanswered_is_not_judged"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Unanswered, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_template_count_in_a_directory_names_the_directory"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::InDirectory, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_template_count_at_the_root_says_the_root_not_an_empty_name"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::AtRoot, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_count_over_its_most_is_refused"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::RangeOver, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_count_with_no_most_accepts_any_number_over_its_least"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::AtLeast, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_count_under_its_least_at_one_directory_says_directory"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::ExactlyOne, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_count_met_is_no_finding"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Met, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_pair_whose_count_requires_a_file_stands"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::PairRequired, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_pair_whose_answered_count_requires_none_does_not_stand"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::PairHolds, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_pair_whose_other_branch_requires_a_file_stands"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::PairFails, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_pair_whose_condition_is_unanswered_is_not_judged"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::PairUnanswered, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("counts_and_pairs_are_judged_apart_in_one_call"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Both, Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_condition_that_holds_requires_its_then_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Holds, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_condition_that_fails_requires_its_else_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Fails, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_count_whose_condition_is_unanswered_is_not_judged_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Unanswered, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_template_count_in_a_directory_names_the_directory_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::InDirectory, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "a_template_count_at_the_root_says_the_root_not_an_empty_name_in_the_fake_as_in_ryo",
        ),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::AtRoot, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_count_over_its_most_is_refused_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::RangeOver, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "a_count_with_no_most_accepts_any_number_over_its_least_in_the_fake_as_in_ryo",
        ),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::AtLeast, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "a_count_under_its_least_at_one_directory_says_directory_in_the_fake_as_in_ryo",
        ),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::ExactlyOne, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_count_met_is_no_finding_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Met, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_pair_whose_count_requires_a_file_stands_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::PairRequired, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "a_pair_whose_answered_count_requires_none_does_not_stand_in_the_fake_as_in_ryo",
        ),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::PairHolds, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_pair_whose_other_branch_requires_a_file_stands_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::PairFails, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_pair_whose_condition_is_unanswered_is_not_judged_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::PairUnanswered, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("counts_and_pairs_are_judged_apart_in_one_call_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Counted(Counted::Both, Port::Fake),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "strays_are_named_after_the_nearest_scope_the_last_declared_on_a_tie_or_the_last_of_the_directory",
        ),
        word: Word::Succeeds,
        input: || Row::Closed(Port::Ryo),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("strays_are_named_in_the_fake_as_in_ryo"),
        word: Word::Succeeds,
        input: || Row::Closed(Port::Fake),
        expect: Expect::Value(true),
    },
];

/// Whether a refusal points at the listing its author reads next.
fn names(reason: &str, listing: &str) -> crate::cases::Selection {
    if reason.contains(listing) {
        crate::cases::Selection::Matches
    } else {
        crate::cases::Selection::Differs
    }
}

/// Whether the catalog lists `tastemaker.placed` and describes it by its six columns, none a copy of another or the same on every row.
fn placed_catalogued() -> bool {
    use crate::domain::catalog::types::{CatalogName, RelationAnswer};
    let adapter = AnalysisRyoAdapter::new(
        ryo::linter::Languages::default(),
        settings(),
        ryo::linter::Cache::None,
    );
    let listed = adapter.relations().is_ok_and(|entries| {
        entries
            .0
            .iter()
            .any(|entry| &*entry.name.0 == "tastemaker.placed")
    });
    let columns = match adapter.relation(&CatalogName("tastemaker.placed".into())) {
        Ok(RelationAnswer::Found(page)) => page
            .columns
            .iter()
            .map(|column| column.name.0.to_string())
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    listed && columns == ["path", "entry", "package", "template", "version", "spec"]
}

/// Whether what the queries said is what the row expects: true when it is, else what they said.
fn agrees(said: String) -> Result<Value, Failure> {
    if said.is_empty() { Ok(true) } else { Err(said) }
}

fn subject(row: Input) -> Result<Value, Failure> {
    let query = match row {
        Row::Query(query) => query,
        Row::Judged(scenario) => return judged(scenario).and_then(agrees),
        Row::Counted(counted, Port::Ryo) => {
            return counted_by(
                &AnalysisRyoAdapter::new(
                    ryo::linter::Languages::default(),
                    settings(),
                    ryo::linter::Cache::None,
                ),
                counted,
            )
            .and_then(agrees);
        }
        Row::Counted(counted, Port::Fake) => {
            return counted_by(
                &crate::outbound::analysis::fixtures::fake::AnalysisFake::default(),
                counted,
            )
            .and_then(agrees);
        }
        Row::Closed(Port::Ryo) => {
            return closed_by(&AnalysisRyoAdapter::new(
                ryo::linter::Languages::default(),
                settings(),
                ryo::linter::Cache::None,
            ))
            .and_then(agrees);
        }
        Row::Closed(Port::Fake) => {
            return closed_by(&crate::outbound::analysis::fixtures::fake::AnalysisFake::default())
                .and_then(agrees);
        }
    };
    let text = match query {
        Query::Text(text) => text.to_owned(),
        Query::PlacedCatalog => return Ok(placed_catalogued()),
        Query::Worked => {
            for (_, worked) in crate::domain::catalog::constants::WORKED_QUERIES {
                let verdict = AnalysisRyoAdapter::new(
                    ryo::linter::Languages::default(),
                    settings(),
                    ryo::linter::Cache::None,
                )
                .validate(&QueryText((*worked).into()))
                .map_err(|error| error.to_string())?;
                if verdict != QueryVerdict::Valid {
                    return Err(format!("{worked}: {verdict:?}"));
                }
            }
            return Ok(true);
        }
        Query::Comments => crate::domain::applications::constants::COMMENTS_QUERY.to_owned(),
        Query::Data => {
            use crate::domain::contracts::types::Syntax;
            for syntax in [
                Syntax::Defines,
                Syntax::Scopes,
                Syntax::Decorates,
                Syntax::Visibility,
                Syntax::Annotates,
                Syntax::Entries,
                Syntax::Mappings,
                Syntax::Elements,
                Syntax::Prose,
                Syntax::Literals,
            ] {
                let verdict = AnalysisRyoAdapter::new(
                    ryo::linter::Languages::default(),
                    settings(),
                    ryo::linter::Cache::None,
                )
                .validate(&QueryText(
                    crate::domain::contracts::service::data_query(syntax)
                        .0
                        .into(),
                ))
                .map_err(|error| error.to_string())?;
                if verdict != QueryVerdict::Valid {
                    return Err(format!("{verdict:?}"));
                }
            }
            return Ok(true);
        }
    };
    match AnalysisRyoAdapter::new(
        ryo::linter::Languages::default(),
        settings(),
        ryo::linter::Cache::None,
    )
    .validate(&QueryText(text.into()))
    .map_err(|error| error.to_string())?
    {
        QueryVerdict::Valid => Ok(true),
        QueryVerdict::Invalid(reason) => Err(reason.0.to_string()),
    }
}

#[test]
fn succeeds() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    check(
        &AnalysisRyoAdapter::new(
            ryo::linter::Languages::default(),
            settings(),
            ryo::linter::Cache::None,
        ),
        &MachinePath(directory.path().display().to_string().into()),
    );
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The settings the adapter runs every call under: one worker, the machine's memory.
fn settings() -> crate::outbound::analysis::types::AnalysisSettings {
    crate::outbound::analysis::types::AnalysisSettings {
        pool: crate::outbound::scheduler::factory::pool(
            crate::outbound::scheduler::types::WorkerCount(1),
        ),
        memory: crate::outbound::analysis::types::Memory::Machine,
    }
}

/// A scenario the verdict queries judge, through ryo itself: what its check reported against what it expects.
#[derive(Clone, Copy, Debug)]
enum Judged {
    /// A scenario that expects the finding its check reported, at its line.
    Expected,
    /// The same scenario, expecting nothing.
    Unexpected,
    /// The same scenario, naming another rule: what the check reported of this one is not compared.
    OnlyOtherRule,
    /// A rule's scenario that expects a finding of its own rule without naming it.
    Own,
    /// A rule's scenario that expects nothing, and the check reported a finding of its own rule.
    OwnUnexpected,
    /// A rule's scenario that expects nothing, whose rule reached no file of the project: it proves nothing.
    OwnUnreached,
    /// A scenario naming a rule in `only` that the check never evaluated.
    OnlyUnevaluated,
    /// A scenario naming in `only` a rule whose id another only ends with, not after a `/` or a `#`.
    OnlyPartialName,
    /// An expected check matches the end of the found id, and a whole file reads with no line.
    CheckOfWholeFile,
    /// A finding on another line than the one expected: one expected, not found, and one found, not expected.
    OtherLine,
    /// The same finding twice, expected once: once found, not expected.
    Twice,
}

/// The finding the row's scenario's check reported, and how often.
fn reported_of(scenario: Judged) -> Vec<crate::domain::findings::types::Finding> {
    use crate::domain::findings::service::{check_id, error, of_rule};
    use crate::domain::findings::types::{Line, Place, RuleId};
    let rule = match scenario {
        Judged::Own | Judged::OwnUnexpected => of_rule(RuleId("p/r".into())),
        Judged::CheckOfWholeFile => check_id("p/errors", "count"),
        Judged::Expected
        | Judged::Unexpected
        | Judged::OnlyOtherRule
        | Judged::OwnUnreached
        | Judged::OnlyUnevaluated
        | Judged::OnlyPartialName
        | Judged::OtherLine
        | Judged::Twice => check_id("p/s", "template"),
    };
    let finding = crate::domain::findings::types::Finding {
        place: match scenario {
            Judged::CheckOfWholeFile => Place::Whole,
            Judged::OtherLine => Place::Line(Line(4)),
            Judged::Expected
            | Judged::Unexpected
            | Judged::OnlyOtherRule
            | Judged::Own
            | Judged::OwnUnexpected
            | Judged::OwnUnreached
            | Judged::OnlyUnevaluated
            | Judged::OnlyPartialName
            | Judged::Twice => Place::Line(Line(3)),
        },
        ..error(
            rule,
            crate::domain::paths::types::RepoPath("a.txt".into()),
            "wrong",
        )
    };
    if matches!(scenario, Judged::Twice) {
        vec![finding.clone(), finding]
    } else {
        vec![finding]
    }
}

/// The findings the row's scenario expects, the rules its `only` names, and the rule it proves.
fn expectations_of(
    scenario: Judged,
) -> (
    Vec<crate::domain::scenarios::types::ExpectedFinding>,
    Vec<crate::domain::scenarios::types::ExpectedRule>,
    Option<crate::domain::proofs::types::FoundOwn<'static>>,
) {
    use crate::domain::findings::types::Line;
    use crate::domain::proofs::types::FoundOwn;
    use crate::domain::scenarios::types::{
        ExpectedFinding, ExpectedLine, ExpectedRule, ExpectedRuleChoice,
    };
    let at = |rule: ExpectedRuleChoice, line: ExpectedLine| ExpectedFinding {
        rule,
        path: crate::domain::paths::types::RepoPath("a.txt".into()),
        line,
    };
    let named = |rule: &str| ExpectedRuleChoice::Named(ExpectedRule(rule.into()));
    let expected = match scenario {
        Judged::Expected | Judged::OtherLine | Judged::Twice => {
            vec![at(named("s#template"), ExpectedLine::At(Line(3)))]
        }
        Judged::Own => vec![at(ExpectedRuleChoice::Own, ExpectedLine::At(Line(3)))],
        Judged::CheckOfWholeFile => vec![at(named("#count"), ExpectedLine::Whole)],
        Judged::Unexpected
        | Judged::OnlyOtherRule
        | Judged::OwnUnexpected
        | Judged::OwnUnreached
        | Judged::OnlyUnevaluated
        | Judged::OnlyPartialName => Vec::new(),
    };
    let only = match scenario {
        Judged::OnlyOtherRule => vec![ExpectedRule("p/other#count".into())],
        Judged::OnlyUnevaluated => vec![ExpectedRule("p/never".into())],
        // `late` is not `p/s#template`'s id after a `/` or a `#`: the finding is not compared, and `late` never ran.
        Judged::OnlyPartialName => vec![ExpectedRule("late".into())],
        Judged::Expected
        | Judged::Unexpected
        | Judged::Own
        | Judged::OwnUnexpected
        | Judged::OwnUnreached
        | Judged::CheckOfWholeFile
        | Judged::OtherLine
        | Judged::Twice => Vec::new(),
    };
    let own = match scenario {
        Judged::Own | Judged::OwnUnexpected => Some(FoundOwn("p/r")),
        Judged::OwnUnreached => Some(FoundOwn("p/unreached")),
        Judged::Expected
        | Judged::Unexpected
        | Judged::OnlyOtherRule
        | Judged::OnlyUnevaluated
        | Judged::OnlyPartialName
        | Judged::CheckOfWholeFile
        | Judged::OtherLine
        | Judged::Twice => None,
    };
    (expected, only, own)
}

/// The problems ryo's verdict queries give the row's scenario, against the ones the row expects; empty when they agree.
fn judged(scenario: Judged) -> Result<String, Failure> {
    use crate::domain::findings::service::{check_id, of_rule};
    use crate::domain::findings::types::RuleId;
    use crate::domain::scenario_judging::service::{host, problems, rules, verdict_of};
    use crate::domain::scenario_judging::types::{ScenarioNumber, VerdictRows};
    use crate::domain::scenarios::types::ScenarioId;
    use crate::outbound::analysis::types::{AnalysisRequest, Corpus};
    let (expected, only, own) = expectations_of(scenario);
    let evaluated = [
        of_rule(RuleId("p/r".into())),
        check_id("p/s", "template"),
        check_id("p/other", "count"),
        check_id("p/errors", "count"),
    ]
    .into_iter()
    .collect();
    let mut rows = VerdictRows::default();
    host(
        ScenarioNumber(0),
        own,
        &expected,
        &only,
        &reported_of(scenario),
        &evaluated,
        &mut rows,
    );
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let report = AnalysisRyoAdapter::new(
        ryo::linter::Languages::default(),
        settings(),
        ryo::linter::Cache::None,
    )
    .lint(AnalysisRequest {
        root: MachinePath(directory.path().display().to_string().into()),
        rules: rules(),
        placed: Vec::new(),
        corpus: Corpus::Given(Vec::new()),
        beside: Vec::new(),
        hosted: crate::outbound::analysis::types::Hosted::Verdict(rows),
    })
    .map_err(|error| error.to_string())?;
    let verdicts = report
        .findings
        .iter()
        .filter_map(verdict_of)
        .collect::<Vec<_>>();
    if verdicts.len() != report.findings.len() {
        return Ok(format!(
            "a row of the verdict does not read: {:?}",
            report.findings
        ));
    }
    let said = problems(
        &ScenarioId("c".into()),
        &verdicts.iter().collect::<Vec<_>>(),
    )
    .iter()
    .map(|problem| problem.0.to_string())
    .collect::<Vec<_>>()
    .join("|");
    let wanted = match scenario {
        Judged::Expected | Judged::OnlyOtherRule | Judged::Own | Judged::CheckOfWholeFile => "",
        Judged::Unexpected => "scenario `c`: found p/s#template a.txt:3, not expected",
        Judged::OwnUnexpected => "scenario `c`: found a.txt:3, not expected",
        // Another rule's finding still counts against the scenario beside its own rule's reaching nothing.
        Judged::OwnUnreached => {
            "scenario `c`: `p/unreached` reached no file of the project, so the scenario proves nothing|scenario `c`: found p/s#template a.txt:3, not expected"
        }
        Judged::OnlyUnevaluated => {
            "scenario `c`: `only` names `p/never`, which the check did not evaluate"
        }
        Judged::OnlyPartialName => {
            "scenario `c`: `only` names `late`, which the check did not evaluate"
        }
        Judged::OtherLine => {
            "scenario `c`: expected s#template a.txt:3, not found|scenario `c`: found s#template a.txt:4, not expected"
        }
        Judged::Twice => "scenario `c`: found s#template a.txt:3, not expected",
    };
    Ok(if said == wanted { String::new() } else { said })
}

/// A count or a pair the count queries judge, through ryo itself and through the fake: what was found against its count
/// and what its condition answered.
#[derive(Clone, Copy, Debug)]
enum Counted {
    /// A spec's conditional count whose condition holds: held to its `then`, one, and none found.
    Holds,
    /// The same, its condition failing: held to its `else`, none.
    Fails,
    /// The same, its condition's rule refused, three found: not judged.
    Unanswered,
    /// A template's count in a directory, its condition holding, none found.
    InDirectory,
    /// A template's count at the project's root, its condition holding, none found.
    AtRoot,
    /// A spec's count of between one and two, three found.
    RangeOver,
    /// A spec's count of at least one, five found.
    AtLeast,
    /// A spec's count of exactly two, one found.
    ExactlyOne,
    /// A spec's count of exactly one, one found.
    Met,
    /// A pair whose count requires one: its finding stands.
    PairRequired,
    /// A pair whose conditional count requires none when its condition holds, which it does.
    PairHolds,
    /// The same pair, its condition failing: it requires one, and its finding stands.
    PairFails,
    /// The same pair, its condition's rule refused: it is not judged.
    PairUnanswered,
    /// A refused count and a required pair together: each numbered apart.
    Both,
}

/// The counts, the pairs and the conditions' answers of the row.
fn waiting_of(
    counted: Counted,
) -> (
    Vec<crate::domain::scopes::types::CountCheck>,
    Vec<crate::domain::scopes::types::PairCheck>,
    std::collections::BTreeMap<
        crate::domain::scopes::types::ConditionAt,
        crate::domain::scopes::types::Answer,
    >,
) {
    use crate::domain::entries::types::{Amount, Bound, ConditionName, Count};
    use crate::domain::findings::service::{check_id, error};
    use crate::domain::paths::types::RepoPath;
    use crate::domain::scopes::types::{
        Answer, ConditionAt, CountCheck, CountMessage, CountUnit, FileSpelling, Identity,
        PairCheck, ScopeIndex,
    };
    let when = |then: u32, otherwise: u32| Count::When {
        condition: ConditionName("c".into()),
        then: Amount(then),
        otherwise: Amount(otherwise),
    };
    let spec = CountMessage::Spec {
        identity: Identity("p/t".into()),
    };
    let template = |directory: &str| CountMessage::Template {
        identity: Identity("p/t".into()),
        file: FileSpelling("a.txt".into()),
        unit: CountUnit::Files,
        directory: RepoPath(directory.into()),
    };
    let count = |count: Count, found: u32, message: CountMessage| CountCheck {
        rule: check_id("p/t", "count"),
        path: RepoPath("tastemaker.toml".into()),
        scope: ScopeIndex(0),
        count,
        found: Amount(found),
        message,
    };
    let pair = |held: Count| PairCheck {
        scope: ScopeIndex(0),
        count: held,
        findings: vec![error(
            check_id("p/t", "pair"),
            RepoPath("a.txt".into()),
            "missing",
        )],
    };
    let answer = match counted {
        Counted::Fails | Counted::PairFails => Answer::Fails,
        Counted::Unanswered | Counted::PairUnanswered => Answer::Unanswered,
        Counted::Holds
        | Counted::InDirectory
        | Counted::AtRoot
        | Counted::RangeOver
        | Counted::AtLeast
        | Counted::ExactlyOne
        | Counted::Met
        | Counted::PairRequired
        | Counted::PairHolds
        | Counted::Both => Answer::Holds,
    };
    let (counts, pairs) = match counted {
        Counted::Holds | Counted::Fails => (vec![count(when(1, 0), 0, spec)], Vec::new()),
        Counted::Unanswered => (vec![count(when(1, 0), 3, spec)], Vec::new()),
        Counted::InDirectory => (vec![count(when(1, 0), 0, template("src"))], Vec::new()),
        Counted::AtRoot => (vec![count(when(1, 0), 0, template(""))], Vec::new()),
        Counted::RangeOver => (
            vec![count(
                Count::Range {
                    min: Amount(1),
                    max: Bound::AtMost(Amount(2)),
                },
                3,
                spec,
            )],
            Vec::new(),
        ),
        Counted::AtLeast => (
            vec![count(
                Count::Range {
                    min: Amount(1),
                    max: Bound::Unbounded,
                },
                5,
                spec,
            )],
            Vec::new(),
        ),
        Counted::ExactlyOne => (vec![count(Count::Exactly(Amount(2)), 1, spec)], Vec::new()),
        Counted::Met => (vec![count(Count::Exactly(Amount(1)), 1, spec)], Vec::new()),
        Counted::PairRequired => (Vec::new(), vec![pair(Count::Exactly(Amount(1)))]),
        Counted::PairHolds | Counted::PairFails | Counted::PairUnanswered => {
            (Vec::new(), vec![pair(when(0, 1))])
        }
        Counted::Both => (
            vec![count(when(1, 0), 0, spec)],
            vec![pair(Count::Exactly(Amount(1)))],
        ),
    };
    let holds = std::iter::once((
        ConditionAt {
            scope: ScopeIndex(0),
            condition: ConditionName("c".into()),
        },
        answer,
    ))
    .collect();
    (counts, pairs, holds)
}

/// The findings `port`'s count queries give the row's counts and pairs, against the ones the row expects; empty when
/// they agree.
fn counted_by(
    port: &dyn crate::outbound::analysis::interface::AnalysisPort,
    counted: Counted,
) -> Result<String, Failure> {
    use crate::domain::judging::service::{findings, host, judgment_of, rules};
    use crate::outbound::analysis::types::{AnalysisRequest, Corpus};
    let closings = crate::domain::closing::types::Closings::default();
    let (counts, pairs, holds) = waiting_of(counted);
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let report = port
        .lint(AnalysisRequest {
            root: MachinePath(directory.path().display().to_string().into()),
            rules: rules(),
            placed: Vec::new(),
            corpus: Corpus::Given(Vec::new()),
            beside: Vec::new(),
            hosted: crate::outbound::analysis::types::Hosted::Counts(host(
                &counts, &pairs, &holds, &closings,
            )),
        })
        .map_err(|error| error.to_string())?;
    let judgments = report
        .findings
        .iter()
        .filter_map(judgment_of)
        .collect::<Vec<_>>();
    if judgments.len() != report.findings.len() {
        return Ok(format!(
            "a row of the count queries does not read: {:?}",
            report.findings
        ));
    }
    let said = findings(
        &counts,
        &pairs,
        &closings,
        &std::collections::BTreeSet::new(),
        &judgments,
    )
    .iter()
    .map(|finding| finding.message.0.to_string())
    .collect::<Vec<_>>()
    .join("|");
    let refused = "`p/t` applies at 0 directories, but its count requires exactly 1";
    let wanted = match counted {
        Counted::Holds => refused,
        Counted::InDirectory => "`a.txt` matches 0 file(s) in `src`, but `p/t` requires exactly 1",
        Counted::AtRoot => {
            "`a.txt` matches 0 file(s) at the project's root, but `p/t` requires exactly 1"
        }
        Counted::RangeOver => {
            "`p/t` applies at 3 directories, but its count requires between 1 and 2"
        }
        Counted::ExactlyOne => "`p/t` applies at 1 directory, but its count requires exactly 2",
        Counted::PairRequired | Counted::PairFails => "missing",
        Counted::Both => "`p/t` applies at 0 directories, but its count requires exactly 1|missing",
        Counted::Fails
        | Counted::Unanswered
        | Counted::AtLeast
        | Counted::Met
        | Counted::PairHolds
        | Counted::PairUnanswered => "",
    };
    Ok(if said == wanted { String::new() } else { said })
}

/// Closed directories `a` and `b`: three scopes own `a`, one `b`, and `a` holds a placed file, its own `tastemaker.toml`, a nested
/// instance and files placed by two scopes at two depths.
fn closed_scene() -> crate::domain::closing::types::Closings {
    use crate::domain::closing::types::{
        ClosedDirectory, ClosedOwner, Closings, DirectoryDepth, EnclosedFile, GroupNumber, Nested,
        Occupancy, Prefix, StrayNumber,
    };
    use crate::domain::paths::types::RepoPath;
    use crate::domain::scopes::types::{Identity, ScopeIndex};
    let owner = |scope: usize, name: &str| ClosedOwner {
        scope: ScopeIndex(scope),
        identity: Identity(name.into()),
    };
    let candidate = |stray: u32, group: u32, path: &str, home: &str| EnclosedFile {
        stray: StrayNumber(stray),
        group: GroupNumber(group),
        path: RepoPath(path.into()),
        home: Prefix(home.into()),
    };
    let occupancy = |group: u32, scope: usize, prefix: &str, depth: u32| Occupancy {
        group: GroupNumber(group),
        owner: ScopeIndex(scope),
        prefix: Prefix(prefix.into()),
        depth: DirectoryDepth(depth),
    };
    Closings {
        evaluated: std::collections::BTreeSet::new(),
        directories: vec![
            ClosedDirectory {
                group: GroupNumber(0),
                directory: RepoPath("a".into()),
                owners: vec![owner(1, "p/one"), owner(2, "p/two"), owner(3, "p/three")],
            },
            ClosedDirectory {
                group: GroupNumber(1),
                directory: RepoPath("b".into()),
                owners: vec![owner(4, "p/four")],
            },
        ],
        candidates: vec![
            candidate(0, 0, "a/x.txt", "a/"),
            candidate(1, 0, "a/placed.txt", "a/"),
            candidate(2, 0, "a/nested/y.txt", "a/nested/"),
            candidate(3, 0, "a/deep/q.txt", "a/deep/"),
            candidate(4, 0, "a/tastemaker.toml", "a/"),
            candidate(5, 0, "a/other/z.txt", "a/other/"),
            candidate(6, 1, "b/w.txt", "b/"),
        ],
        placed: vec![RepoPath("a/placed.txt".into())],
        own: vec![RepoPath("a/tastemaker.toml".into())],
        nested: vec![Nested {
            group: GroupNumber(0),
            prefix: Prefix("a/nested/".into()),
        }],
        occupied: vec![
            occupancy(0, 1, "a/", 1),
            occupancy(0, 3, "a/", 1),
            occupancy(0, 2, "a/deep/", 2),
        ],
    }
}

/// The strays `port`'s closed query decides over closed directories `a` and `b`, in words: a file in `a` that no template places is
/// named after the scope that placed files in the nearest directory above it (the last declared of two that did in one), else after
/// the directory's last declared scope; a placed file, a `tastemaker.toml` and a file in a nested instance are no strays. Empty when
/// they are what the row expects.
fn closed_by(
    port: &dyn crate::outbound::analysis::interface::AnalysisPort,
) -> Result<String, Failure> {
    use crate::domain::judging::service::{findings, host, judgment_of, rules};
    use crate::outbound::analysis::types::{AnalysisRequest, Corpus};
    let closings = closed_scene();
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let report = port
        .lint(AnalysisRequest {
            root: MachinePath(directory.path().display().to_string().into()),
            rules: rules(),
            placed: Vec::new(),
            corpus: Corpus::Given(Vec::new()),
            beside: Vec::new(),
            hosted: crate::outbound::analysis::types::Hosted::Counts(host(
                &[],
                &[],
                &std::collections::BTreeMap::new(),
                &closings,
            )),
        })
        .map_err(|error| error.to_string())?;
    let judgments = report
        .findings
        .iter()
        .filter_map(judgment_of)
        .collect::<Vec<_>>();
    if judgments.len() != report.findings.len() {
        return Ok(format!(
            "a row of the queries does not read: {:?}",
            report.findings
        ));
    }
    let said = findings(
        &[],
        &[],
        &closings,
        &std::collections::BTreeSet::new(),
        &judgments,
    )
    .iter()
    .map(|finding| finding.message.0.to_string())
    .collect::<Vec<_>>()
    .join("|");
    let wanted = [
        "`a/x.txt` is not placed by any template, and `p/three` at `a` is closed",
        "`a/deep/q.txt` is not placed by any template, and `p/two` at `a` is closed",
        "`a/other/z.txt` is not placed by any template, and `p/three` at `a` is closed",
        "`b/w.txt` is not placed by any template, and `p/four` at `b` is closed",
    ]
    .join("|");
    Ok(if said == wanted { String::new() } else { said })
}
