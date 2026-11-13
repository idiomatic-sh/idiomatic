//! The support of the `generations` cases.
use super::super::{Prepared, RuleRun, language_columns, prepare, run_rule};
use crate::application::file_dag::program::facade_runtime::file_execution::service::open_file;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Integer, Value as CellValue};
use ryo_values::domain::{plan_parts::types::WholeNumber, query_bindings::types::ParameterBinding};
use std::borrow::Cow;
use std::sync::Arc;

/// What one file's generation owner answered: the provider reads its rule took, the cells it
/// emitted, and whether finishing it answered a frontier readiness.
#[derive(Debug, PartialEq)]
pub(super) struct Generation {
    pub(super) read: bool,
    pub(super) cells: Cow<'static, [CellValue]>,
    pub(super) readiness: bool,
}

/// What two file generation owners of one prepared request answered, each over its own file's
/// language column, and the first owner's cells once both finished.
#[derive(Debug, PartialEq)]
pub(super) struct Generations {
    pub(super) first: Generation,
    pub(super) second: Generation,
    pub(super) first_cells_after_finish: Cow<'static, [CellValue]>,
}

/// Opens two generation owners of one prepared request (the rule bound to 3), runs each over
/// the language column of its file, and finishes both.
pub(super) fn two_generations(
    first_file: FileId,
    second_file: FileId,
) -> Result<Generations, OperatingError> {
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
        &language_columns(first_file, Integer(3)),
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
        &language_columns(second_file, Integer(3)),
        &mut second_symbols,
    );
    let first_readiness = first.finish()?.is_some();
    let second_readiness = second.finish()?.is_some();
    Ok(Generations {
        first_cells_after_finish: left.cells.clone().into(),
        first: Generation {
            read: first_reads.0 > 0,
            cells: left.cells.into(),
            readiness: first_readiness,
        },
        second: Generation {
            read: second_reads.0 > 0,
            cells: right.cells.into(),
            readiness: second_readiness,
        },
    })
}
