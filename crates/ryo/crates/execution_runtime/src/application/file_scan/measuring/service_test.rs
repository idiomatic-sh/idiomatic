//! The cases of a measuring run's report: a rule set with no rule has no ledger line; and a worker made for a measuring run
//! asks the engine for its counting state, and its ledger has a line for each rule the rule set runs, with what was counted.
//! Over the engine itself for the empty rule set, and the engine port's fake, whose ledger says the rows its scans found. A
//! load that recorded where its plans' steps were written names each unit's rules and statements; one that did not says it
//! knows none.

use super::service::ledger_lines;
use crate::application::file_scan::service::{emit_columns, place_columns, ran, worker};
use crate::application::file_scan::types::{LedgerLine, ScanMeasuring, ScanRun};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::factory::use_case;
use crate::outbound::engine::fixtures::fake::{EngineFake, no_kinds};
use crate::outbound::engine::interface::EnginePort as _;
use ryo_engine::domain::lowering::fixtures::{digests_of, named};
use ryo_engine::domain::lowering::types::{LoadGiven, SelectedRule, SelectedRules};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::statistics::types::StatisticsBytes;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::types::{WalkEngine, WalkRequest};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_tokens::types::{QuerySourceId, StepSpelling};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::written_sites::types::{
    RuleWritten, RulesWritten, StepWritten, WrittenSite,
};
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// What a row measures.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The engine itself, no rule, no worker.
    NoRule,
    /// The fake's one tree rule, two workers of a measuring run, a file of two words and a file of three.
    Measuring,
    /// The same, the workers made for a run that does not measure.
    Plain,
    /// The measuring run, its load's record naming the rule `R1` and its first step's statement, `words`, of its own query.
    Named,
}

type Input = Scenario;
/// The ledger's lines, then how many counting states the fake was asked for.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_set_with_no_rule_has_no_ledger_line"),
        word: Word::Succeeds,
        input: || Scenario::NoRule,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a_measuring_runs_workers_count_and_its_ledger_has_a_line_a_rule"),
        word: Word::Succeeds,
        input: || Scenario::Measuring,
        expect: Expect::Value(Cow::Borrowed(
            "unit=Rule:0 class=Navigation rows=5 steps=0 nanos=0 idle_nanos=0 owners=0 decide_nanos=0 provider_nanos=0 rules=0 written=?\ncounting states asked for: 2",
        )),
    },
    Case {
        name: CaseName("a_run_that_does_not_measure_asks_for_no_counting_state"),
        word: Word::Succeeds,
        input: || Scenario::Plain,
        expect: Expect::Value(Cow::Borrowed(
            "unit=Rule:0 class=Navigation rows=5 steps=0 nanos=0 idle_nanos=0 owners=0 decide_nanos=0 provider_nanos=0 rules=0 written=?\ncounting states asked for: 0",
        )),
    },
    Case {
        name: CaseName(
            "a_load_that_recorded_where_its_steps_were_written_names_each_units_rules_and_statements",
        ),
        word: Word::Succeeds,
        input: || Scenario::Named,
        expect: Expect::Value(Cow::Borrowed(
            "unit=Rule:0 class=Navigation rows=5 steps=0 nanos=0 idle_nanos=0 owners=0 decide_nanos=0 provider_nanos=0 rules=R1 written=R1:words\ncounting states asked for: 2",
        )),
    },
];

/// The record of a load of the fake's one rule, `R1`, whose first step was written as `words` in the rule's own query.
fn named_load() -> RulesWritten {
    let root = QuerySourceId("<query>".into());
    let site = WrittenSite {
        document: root.clone(),
        statement: StepSpelling("words".into()),
        trace: Arc::from([]),
    };
    RulesWritten(vec![RuleWritten {
        id: RuleId("R1".into()),
        root,
        steps: vec![StepWritten::Written(Arc::new(site))],
    }])
}

/// The ledger's lines, one a line.
fn said(lines: &[LedgerLine]) -> String {
    let lines: Vec<&str> = lines.iter().map(|line| &*line.0).collect();
    lines.join("\n")
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let measuring = match scenario {
        Scenario::NoRule => {
            let lines = ledger_lines(
                &use_case(),
                &Programs::default(),
                &RulesWritten::default(),
                &[],
            );
            return Ok(Cow::Owned(said(&lines)));
        }
        Scenario::Measuring | Scenario::Named => ScanMeasuring::Measuring,
        Scenario::Plain => ScanMeasuring::Plain,
    };
    let written = match scenario {
        Scenario::Named => named_load(),
        Scenario::NoRule | Scenario::Measuring | Scenario::Plain => RulesWritten::default(),
    };
    let (engine, plan) = (EngineFake::default(), named("one tree rule"));
    let digests = digests_of(&plan, RuleBindings(&[]));
    let rule = SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan: &plan,
        bindings: RuleBindings(&[]),
        digests: &digests,
    };
    let source = LoadSource::Plans(
        SelectedRules(&[rule]),
        StatisticsBytes(&[]),
        LoadGiven::default(),
    );
    let programs = engine
        .load(source, &no_kinds())
        .map(|loaded| loaded.programs)
        .unwrap_or_default();
    let (columns, places) = (
        emit_columns(&engine, &programs),
        place_columns(&[SelectedRuleOrdinal(0)], std::slice::from_ref(&plan.emit)),
    );
    let run = ScanRun {
        programs: &programs,
        columns: &columns,
        places: &places,
    };
    let words: Arc<dyn WalkEngine> = Arc::new(Walking);
    let sources: [&[u8]; 2] = [b"one two", b"three four five"];
    let mut workers = Vec::new();
    for (number, source) in (0..).zip(sources) {
        let mut worker = worker(&words, &engine, &programs, measuring);
        let file = WalkRequest {
            selection: ryo_language::inbound::use_cases::walk::types::WalkSelection::Recognized,
            path: SourcePath("a.walk"),
            file: FileId(number),
            source: ByteSlice(source),
        };
        let _counted = ran(&engine, &run, &mut worker, &[file]);
        workers.push(worker);
    }
    let held: Vec<_> = workers.iter().collect();
    let lines = ledger_lines(&engine, &programs, &written, &held);
    Ok(Cow::Owned(format!(
        "{}\ncounting states asked for: {}",
        said(&lines),
        engine.measuring.load(Ordering::Relaxed)
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
