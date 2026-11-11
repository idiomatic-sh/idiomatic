//! Independent demand, reset and literal-input guards for brace-format parsing.

use super::fixtures::{
    brace_parser_is_not_run_during_model_initialization,
    model_reinitialization_clears_brace_results_for_replacement_sources,
    percent_only_demand_never_prepares_brace_grammar,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `percent_only_demand_never_prepares_brace_grammar` scenario.
    PercentOnlyDemandNeverPreparesBraceGrammar,
    /// The `brace_parser_is_not_run_during_model_initialization` scenario.
    BraceParserIsNotRunDuringModelInitialization,
    /// The `model_reinitialization_clears_brace_results_for_replacement_sources` scenario.
    ModelReinitializationClearsBraceResultsForReplacementSources,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("percent_only_demand_never_prepares_brace_grammar"),
        word: Word::Succeeds,
        input: || Given::PercentOnlyDemandNeverPreparesBraceGrammar,
        expect: Expect::Value("Fresh"),
    },
    Case {
        name: CaseName("brace_parser_is_not_run_during_model_initialization"),
        word: Word::Succeeds,
        input: || Given::BraceParserIsNotRunDuringModelInitialization,
        expect: Expect::Value("Fresh|Complete"),
    },
    Case {
        name: CaseName("model_reinitialization_clears_brace_results_for_replacement_sources"),
        word: Word::Succeeds,
        input: || Given::ModelReinitializationClearsBraceResultsForReplacementSources,
        expect: Expect::Value("1|Fresh|true"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::PercentOnlyDemandNeverPreparesBraceGrammar => {
            percent_only_demand_never_prepares_brace_grammar()
        }
        Given::BraceParserIsNotRunDuringModelInitialization => {
            brace_parser_is_not_run_during_model_initialization()
        }
        Given::ModelReinitializationClearsBraceResultsForReplacementSources => {
            model_reinitialization_clears_brace_results_for_replacement_sources()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
