//! The inputs of the growth proof: a long derived constructor, a rule of this profile with the modules it registers, and the
//! fastest lint of a file with it, through the public SDK.
use ryo::linter::{
    CaseSource, Corpus, LintRequest, Linter, RelativePath, Rule, RuleApplicability, RuleId,
    SourceText,
};
use ryo_benchmark::outbound::suite::interface::SuitePort as _;
use ryo_benchmark::outbound::suite::types::ArtifactPath;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

/// The runs at each size; the fastest counts.
const RUNS: usize = 3;

/// One derived constructor of `size` statements after its `super()`: a branch reading `this`, a loop that may break, and a
/// try block whose handler and finalizer each call, in turn.
pub(super) fn constructor(size: usize) -> String {
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
    text
}

/// The rule `id` of this profile, with the modules the profile registers.
pub(super) fn rule(id: &str) -> Result<Rule, Cow<'static, str>> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tool = ryo_benchmark::outbound::suite::factory::local()
        .profile(ArtifactPath(directory))
        .map_err(|error| Cow::Owned(error.to_string()))?;
    let mut modules = Vec::with_capacity(tool.query_modules.len());
    for (name, path) in &tool.query_modules {
        let text = std::fs::read_to_string(directory.join(path))
            .map_err(|error| Cow::Owned(error.to_string()))?;
        modules.push(
            ryo::query::QueryModuleSource::try_new(&name.0, &text)
                .map_err(|error| Cow::Owned(error.to_string()))?,
        );
    }
    let catalog = ryo::query::QueryModuleCatalog::try_new(modules)
        .map_err(|error| Cow::Owned(error.to_string()))?;
    let query = std::fs::read_to_string(directory.join("rules").join(id).join("query.ryoql"))
        .map_err(|error| Cow::Owned(error.to_string()))?;
    Ok(Rule {
        observations: Vec::new(),
        query: ryo::query::RyoQlQuery::with_modules(&format!("rules/{id}"), &query, &catalog)
            .map_err(|error| Cow::Owned(error.to_string()))?,
        applicability: RuleApplicability::AcceptedSyntax,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    })
}

/// The fastest of `RUNS` lints of `text` with the rule `id` alone.
pub(super) fn fastest(
    linter: &Linter,
    id: &RuleId,
    rule: &Rule,
    text: &str,
) -> Result<Duration, Cow<'static, str>> {
    let mut best = Duration::MAX;
    for _ in 0..RUNS {
        let request = LintRequest {
            source_language: ryo::linter::SourceLanguage::Recognized,
            root: ".".into(),
            project_search_scope: ryo::linter::ProjectSearchScope::Undeclared,
            given_project_inventory: Default::default(),
            rules: [(id.clone(), rule.clone())].into_iter().collect(),
            files: Vec::new(),
            hosted: Vec::new(),
            corpus: Corpus::Given(vec![CaseSource {
                executable: Default::default(),
                path: RelativePath("case.ts".into()),
                text: SourceText(text.into()),
            }]),
            beside: Vec::new(),
        };
        let started = Instant::now();
        let _report = ryo::linter::lint_with(linter, request)
            .map_err(|error| Cow::Owned(error.to_string()))?;
        best = best.min(started.elapsed());
    }
    Ok(best)
}
