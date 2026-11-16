//! oxlint's growth plan: how the control-flow rules grow with a derived constructor.
//!
//! Each rule is linted alone over one derived constructor of 1,000, 2,000 and 4,000 statements of branches, loops and try
//! blocks: two shapes per rule, 1,000 to 2,000 and 2,000 to 4,000 statements. The shared growth service holds each to the one
//! CPU limit and to Ruff's floor and memory bound; a rule whose work, or whose reading, is quadratic in the constructor grows
//! four times.
use super::constants;
use super::locations::directory;
use ryo_benchmark::domain::growth::constants::{CPU_FLOOR, MEMORY_BOUND};
use ryo_benchmark::domain::growth::types::{
    GrownProgram, GrowthModule, GrowthPlan, GrowthSize, RuleModules, ShapePrograms, SourceFileName,
};
use ryo_benchmark::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::measurements::types::GrowthShape;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, InputDeclaration, ParserSelection, ProviderSelection,
    RuleContextPreparation, SearchRoots, WorkerRequest, WorkerState,
};
use std::collections::BTreeMap;

/// The control-flow rules grown over the constructor.
const RULES: [&str; 3] = [
    "eslint.constructor-super",
    "eslint.no-this-before-super",
    "eslint.no-unreachable",
];
/// The whole sizes of the constructor, in statements; each shape's half is half of it.
const WHOLE_SIZES: [usize; 2] = [2_000, 4_000];
/// The shape's place in its name.
const CONSTRUCTOR: &str = "constructor";

/// One derived constructor of `size` statements after its `super()`: a branch reading `this`, a loop that may break, and a
/// try block whose handler and finalizer each call, in turn.
fn constructor(size: usize) -> GrownProgram {
    let mut text = String::from("class A extends B {\n  constructor() {\n    super();\n");
    for at in 0..size {
        let statement = match at % 3 {
            0 => format!("    if (a{at}) {{ this.x{at} = 1; }} else {{ g(); }}\n"),
            1 => format!("    while (a{at}) {{ if (b()) {{ break; }} g(); }}\n"),
            _ => "    try { f(); } catch (e) { g(); } finally { h(); }\n".to_owned(),
        };
        text.push_str(&statement);
    }
    text.push_str("  }\n}\n");
    GrownProgram(text)
}

/// oxlint's plan: each control-flow rule over the constructor at each doubling, with the modules this profile registers,
/// linted with the bench's parser configuration.
///
/// # Errors
/// A rule identity the SDK refuses, or a profile the suite cannot read.
pub(super) fn plan(suite: &dyn SuitePort, parser: &ParserSelection) -> Result<GrowthPlan, Error> {
    let bench = directory();
    let profile = suite.profile(ArtifactPath(&bench))?;
    let modules = RuleModules(
        profile
            .query_modules
            .into_iter()
            .map(|(name, path)| (GrowthModule(name.0), path))
            .collect(),
    );
    let mut shapes = BTreeMap::new();
    for rule in RULES {
        let id = ryo::linter::RuleId::try_from(rule)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        for whole in WHOLE_SIZES {
            let _previous = shapes.insert(
                GrowthShape(format!("{rule}/{CONSTRUCTOR}/{whole}")),
                ShapePrograms {
                    rule: id.clone(),
                    modules: modules.clone(),
                    half: constructor(whole / 2),
                    whole: constructor(whole),
                    size: GrowthSize(whole),
                },
            );
        }
    }
    Ok(GrowthPlan {
        floor: CPU_FLOOR,
        memory: MEMORY_BOUND,
        file: SourceFileName(constants::GROWTH_SOURCE.into()),
        request: WorkerRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Explicit(Vec::new()),
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Unconfigured,
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode: Default::default(),
            context_manifest: ContextSeal::Unsealed,
            rules: bench.join(constants::RULES_DIRECTORY),
            root: bench,
            query_modules: BTreeMap::new(),
            rule_applicability: profile.rule_applicability,
            parser_configuration: parser.clone(),
            source_language: ProviderSelection::Unspecified,
            files: Vec::new(),
            state: WorkerState::Fresh,
        },
        shapes,
    })
}
