//! Which files the `governance` service lets a placed rule answer about, on values alone.

use std::collections::BTreeMap;

use super::service::{governance, governed};
use super::types::{Answers, Governance};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::fixtures::matching;
use crate::domain::applications::types::{
    GlobText, LintAnswer, LintRuleId, Plan, PlannedRule, Purpose, Rendering,
};
use crate::domain::definitions::types::{ColumnName, Diagnostic, QueryText};
use crate::domain::findings::types::MessageTemplate;
use crate::domain::findings::types::RuleId;
use crate::domain::paths::types::RepoPath;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// A placed rule that reads `src/**` answers about a file under `src` and one outside it.
    Answers,
    /// The same rule, over a corpus with no file it reads.
    IdleCorpus,
    /// The same rule, over a corpus with a file it reads.
    GoverningCorpus,
}

type Input = Scenario;
/// What the service answered, written out.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_answers_only_about_the_files_it_reads"),
        word: Word::Succeeds,
        input: || Scenario::Answers,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_that_reads_none_of_the_corpus_is_idle"),
        word: Word::Succeeds,
        input: || Scenario::IdleCorpus,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_that_reads_a_file_of_the_corpus_governs"),
        word: Word::Succeeds,
        input: || Scenario::GoverningCorpus,
        expect: Expect::Value(String::new()),
    },
];

/// One placed rule, `r`, that reads `src/**`.
fn plan() -> Plan {
    let id = LintRuleId("r".into());
    Plan {
        rules: vec![PlannedRule {
            id: id.clone(),
            query: QueryText("q".into()),
            parameters: BTreeMap::new(),
            include: vec![GlobText("src/**".into())],
            exclude: Vec::new(),
        }],
        purposes: BTreeMap::from([(
            id,
            Purpose::Rule(Rendering {
                rule: RuleId("p/r".into()),
                diagnostic: Diagnostic {
                    primary: ColumnName("span".into()),
                    message: MessageTemplate("m".into()),
                    help: MessageTemplate("h".into()),
                },
            }),
        )]),
        ..Plan::default()
    }
}

fn answer(path: &str) -> LintAnswer {
    LintAnswer {
        rule: LintRuleId("r".into()),
        path: RepoPath(path.into()),
        cells: Vec::new(),
    }
}

/// What the scenario gave, and what it should give.
fn said(scenario: Scenario) -> (String, &'static str) {
    let plan = plan();
    let matching = matching();
    let id = LintRuleId("r".into());
    let verdict = |corpus: &[&str]| {
        let corpus = corpus
            .iter()
            .map(|path| RepoPath((*path).into()))
            .collect::<Vec<_>>();
        match governance(&plan, &id, &corpus, matching.sets) {
            Governance::Governs => "governs",
            Governance::Idle => "idle",
        }
        .to_owned()
    };
    match scenario {
        Scenario::Answers => {
            let kept = governed(
                &plan,
                Answers {
                    answers: vec![answer("src/lib.txt"), answer("docs/a.md")],
                    unknowns: Vec::new(),
                    unread: Vec::new(),
                },
                matching.sets,
            );
            (
                kept.answers
                    .iter()
                    .map(|answer| &*answer.path.0)
                    .collect::<Vec<_>>()
                    .join(","),
                "src/lib.txt",
            )
        }
        Scenario::IdleCorpus => (verdict(&["docs/a.md"]), "idle"),
        Scenario::GoverningCorpus => (verdict(&["docs/a.md", "src/lib.txt"]), "governs"),
    }
}

fn subject(scenario: Input) -> Value {
    let (said, wanted) = said(scenario);
    if said == wanted { String::new() } else { said }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
