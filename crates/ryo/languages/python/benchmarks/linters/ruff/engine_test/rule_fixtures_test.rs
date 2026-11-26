//! The rule, module and suppress-module fixtures one engine scenario writes.
use super::prepared_engine_test::fixture_cohort;
use super::prepared_engine_test::suppress_modules;
use super::scenario_test::EngineScenario;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;

pub(super) fn write_rule_fixtures(
    rules: &std::path::Path,
    scenario: EngineScenario,
) -> Result<Vec<ryo::linter::RuleId>, Error> {
    let mut selected = fixture_cohort(&["E722"]);
    let root = rules
        .parent()
        .ok_or_else(|| Error::Invalid("rules directory has no root".into()))?;
    write_suppress_module(root)?;
    let e722 = rules.join("E722");
    std::fs::create_dir_all(&e722)?;
    std::fs::write(
        e722.join("query.ryoql"),
        include_str!("../rules/E722/query.ryoql"),
    )?;
    if matches!(
        scenario,
        EngineScenario::PreservedSemanticSyntaxError
            | EngineScenario::PreservedStubSemanticSyntaxError
            | EngineScenario::OrdinaryAnnotationNamedExpression
    ) {
        let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for path in modules_for(scenario)?.values() {
            let destination = root.join(path);
            let parent = destination
                .parent()
                .ok_or_else(|| Error::Invalid("module has no parent".into()))?;
            std::fs::create_dir_all(parent)?;
            let _copied = std::fs::copy(source.join(path), destination)?;
        }
        selected.extend(fixture_cohort(&["F821"]));
        let f821 = rules.join("F821");
        std::fs::create_dir_all(&f821)?;
        std::fs::write(
            f821.join("query.ryoql"),
            include_str!("../rules/F821/query.ryoql"),
        )?;
    }
    if scenario == EngineScenario::FutureFeatureProjection {
        selected.extend(fixture_cohort(&["F407"]));
        let f407 = rules.join("F407");
        std::fs::create_dir_all(&f407)?;
        std::fs::write(
            f407.join("query.ryoql"),
            include_str!("../rules/F407/query.ryoql"),
        )?;
    }
    if scenario == EngineScenario::LateFutureDirectiveProjection {
        selected.extend(fixture_cohort(&["F404"]));
        let f404 = rules.join("F404");
        std::fs::create_dir_all(&f404)?;
        std::fs::write(
            f404.join("query.ryoql"),
            include_str!("../rules/F404/query.ryoql"),
        )?;
    }
    if scenario == EngineScenario::FrontendRuleCollision {
        selected.extend(fixture_cohort(&["invalid-syntax"]));
        let invalid_syntax = rules.join("invalid-syntax");
        std::fs::create_dir_all(&invalid_syntax)?;
        std::fs::write(
            invalid_syntax.join("query.ryoql"),
            include_str!("../rules/E722/query.ryoql"),
        )?;
    }
    Ok(selected)
}

/// Use the query's declared dependency registry for the preserved-facts integration cases.
pub(super) fn modules_for(
    scenario: EngineScenario,
) -> Result<BTreeMap<String, std::path::PathBuf>, Error> {
    if !matches!(
        scenario,
        EngineScenario::PreservedSemanticSyntaxError
            | EngineScenario::PreservedStubSemanticSyntaxError
            | EngineScenario::OrdinaryAnnotationNamedExpression
    ) {
        return Ok(suppress_modules());
    }
    let profile: toml::Value = toml::from_str(include_str!("../profiles/F821/benchmark.toml"))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let modules = profile
        .get("query-modules")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| Error::Invalid("F821 profile has no declared modules".into()))?;
    let mut resolved: BTreeMap<String, std::path::PathBuf> = modules
        .iter()
        .map(|(name, path)| {
            path.as_str()
                .map(|path| (name.clone(), std::path::PathBuf::from(path)))
                .ok_or_else(|| Error::Invalid("F821 module path is not text".into()))
        })
        .collect::<Result<_, Error>>()?;
    resolved.extend(suppress_modules());
    // The modules the declared ones import in turn are the ones the profile's own list leaves out.
    let mut roots: Vec<String> = resolved.keys().cloned().collect();
    let query = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rules/F821/query.ryoql"))?;
    roots.extend(super::prepared_engine_test::imports_of(&query));
    resolved.extend(super::prepared_engine_test::fragments_closure(roots));
    Ok(resolved)
}

pub(super) fn write_suppress_module(root: &std::path::Path) -> Result<(), Error> {
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for path in suppress_modules().values() {
        let destination = root.join(path);
        std::fs::create_dir_all(destination.parent().ok_or_else(|| Error::Invalid("fragment has no parent".into()))?)?;
        let _copied = std::fs::copy(source.join(path), destination)?;
    }
    Ok(())
}
