//! The support of the `generations` cases.
use super::super::{
    FRESH_GENERATION_READS, Prepared, RuleRun, TRANSFERRED_INDEPENDENT, language_columns, prepare,
    run_rule,
};
use crate::application::file_dag::program::facade_runtime::file_execution::open_file;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Integer, Value};
use ryo_values::cases::{Case, CaseName, Expect, Word};
use ryo_values::domain::{plan_parts::types::WholeNumber, query_bindings::types::ParameterBinding};
use std::sync::Arc;

pub(super) fn two_file_generation_owners_cannot_reuse_the_other_files_results()
-> Result<Answer, Failure> {
    let Prepared { prepared, services } =
        prepare(vec![vec![ParameterBinding::Number(WholeNumber(3))]]);
    let mut first_symbols = Symbols::default();
    let mut first = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        Arc::clone(&services),
        ryo_values::domain::files::types::SourceView(b""),
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        &mut first_symbols,
        None,
        None,
    )
    .unwrap();
    let RuleRun {
        table: left,
        reads: first_reads,
    } = run_rule(
        &mut first,
        RuleIndex(0),
        &language_columns(FileId(1), Integer(3)),
        &mut first_symbols,
    );
    let mut second_symbols = Symbols::default();
    let mut second = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        &mut second_symbols,
        None,
        None,
    )
    .unwrap();
    let RuleRun {
        table: right,
        reads: second_reads,
    } = run_rule(
        &mut second,
        RuleIndex(0),
        &language_columns(FileId(9), Integer(3)),
        &mut second_symbols,
    );
    assert!(first_reads.0 > 0);
    assert!(second_reads.0 > 0, "{FRESH_GENERATION_READS}");
    assert_eq!(left.cells, vec![Value::File(FileId(1))]);
    assert_eq!(right.cells, vec![Value::File(FileId(9))]);
    first.finish().unwrap();
    second.finish().unwrap();
    assert_eq!(
        left.cells,
        vec![Value::File(FileId(1))],
        "{TRANSFERRED_INDEPENDENT}"
    );
    Ok(1)
}

/// A scenario of this group: it answers how many times it held.
pub(super) type Scenario = fn() -> Result<Answer, Failure>;

/// What a scenario answers: a yes as one.
pub(super) type Answer = usize;

/// This part has no failure of its own.
pub(super) type Failure = ();

pub(super) fn subject(scenario: Scenario) -> Result<Answer, Failure> {
    scenario()
}

pub(super) const CASES: &[Case<Scenario, Answer, Failure>] = &[Case {
    name: CaseName("two_file_generation_owners_cannot_reuse_the_other_files_results"),
    word: Word::Succeeds,
    input: || two_file_generation_owners_cannot_reuse_the_other_files_results,
    expect: Expect::Value(1),
}];
