//! The one registrar: the settings profile of each default, its cases, definitions, modules and evidence, staged and pinned.
use super::constants;
use crate::domain::canonical_text::service::render;
use crate::domain::canonical_text::types::{Fragment, Label};
use crate::outbound::executables::types::Sealed;
use crate::outbound::inventory::interface::InventoryPort;
use crate::outbound::inventory::types::{
    ApplicabilityText, Command, ExecutablePin, FilePin, IdentityKey, ProfileProjection,
    ProviderName, RegisteredModuleName, RegisteredRule, Registration, RegistrationAuthority,
    RuleCommand, RuleDefinition, RuleInstance, RuleInstanceRecord, RuleReason,
};
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{ArtifactPath, RuleQuery};
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    CanonicalJson, PathText, ReasonText, RuleName, Sha256Text,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::{CASES_FILE, PROFILE_MANIFEST, QUERY_FILE, WORKER_SHA256};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, InputDeclaration, ParserSelection, RuleLabel,
};
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The digest of one regular file.
fn digest(suite: &dyn SuitePort, path: &Path) -> Result<Sha256Text, Error> {
    Ok(Sha256Text(suite.digest(ArtifactPath(path))?.0))
}

/// The pinned bytes of one file.
fn pin(suite: &dyn SuitePort, path: PathBuf) -> Result<FilePin, Error> {
    Ok(FilePin {
        sha256: digest(suite, &path)?,
        path,
    })
}

/// A path relative to the repository, as the registration writes it.
fn repository_relative(repository: &Path, path: &Path) -> PathText {
    PathText(
        path.strip_prefix(repository)
            .unwrap_or(path)
            .display()
            .to_string(),
    )
}


/// The logical module names a definition imports: `import <name> from "<module>";` at the start of a line.
fn imports(definition: &RuleQuery) -> Vec<RegisteredModuleName> {
    definition
        .0
        .lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix(constants::IMPORT)?;
            let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();
            let name_end = rest
                .char_indices()
                .find(|(_index, character)| !(character.is_alphanumeric() || *character == constants::UNDERSCORE))
                .map(|(index, _character)| index)?;
            let rest = rest.get(name_end..).filter(|after| after.len() != rest.len())?;
            let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();
            let rest = rest.strip_prefix(constants::FROM)?;
            let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();
            let rest = rest.strip_prefix(constants::QUOTE)?;
            let (module, after) = rest.split_once(constants::QUOTE)?;
            after
                .trim_start()
                .starts_with(constants::END_OF_IMPORT)
                .then(|| RegisteredModuleName(module.to_owned()))
        })
        .collect()
}

/// The settings profile directory of a rule: `profiles/<id>` when it holds a profile, else the bench.
fn settings_directory(suite: &dyn SuitePort, bench: &Path, rule: &ryo::linter::RuleId) -> Result<PathBuf, Error> {
    let dedicated = bench.join(constants::SETTINGS_PROFILES).join(rule.to_string());
    match suite.digest(ArtifactPath(&dedicated.join(PROFILE_MANIFEST))) {
        Ok(_digest) => Ok(dedicated),
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(bench.to_owned()),
        Err(error) => Err(error),
    }
}

/// What every rule of one registration is staged with.
struct Bench<'a> {
    suite: &'a dyn SuitePort,
    inventory: &'a dyn InventoryPort,
    /// The canonical bench directory.
    root: PathBuf,
    /// The repository the registration's relative paths are written against.
    repository: PathBuf,
    /// The bench profile.
    profile: Tool,
    /// The directory every rule is staged under.
    output: PathBuf,
}

/// The rules a rule's cases select, closed over the observations their producers declare, and their definitions.
fn cohort(bench: &Bench<'_>, settings: &Tool, rule: &ryo::linter::RuleId) -> Result<BTreeSet<ryo::linter::RuleId>, Error> {
    let cases = bench.suite.rule_cases(ArtifactPath(&bench.root.join(crate::domain::suite::constants::RULES)), rule)?;
    let names: BTreeSet<_> = cases.cases.iter().map(|case| &case.name.0).collect();
    if cases.cases.is_empty() || names.len() != cases.cases.len() {
        return Err(Error::Invalid(constants::CASES_EMPTY_OR_DUPLICATE.into()));
    }
    let mut cohort = BTreeSet::from([rule.clone()]);
    for case in &cases.cases {
        let selected = match &case.selected_rules {
            CohortSelection::Explicit(rules) => rules.clone(),
            CohortSelection::Frozen => vec![rule.clone()],
        };
        let declarations = match &case.rule_observations {
            InputDeclaration::Declared(declarations) => declarations.clone(),
            InputDeclaration::Undeclared => crate::domain::observations::service::project_profile(
                &settings.rule_observations,
                &selected,
                settings.rule_observation_selection,
            ),
        };
        cohort.extend(crate::domain::observations::service::closure(selected, |id| {
            declarations.get(id).map(Vec::as_slice)
        }));
    }
    Ok(cohort)
}

/// The modules the staged definitions import, closed over the modules' own imports, by logical name.
fn module_closure(
    bench: &Bench<'_>,
    settings: &Tool,
    definitions: &[RuleQuery],
) -> Result<BTreeMap<RegisteredModuleName, PathBuf>, Error> {
    let mut registry = bench.profile.query_modules.clone();
    registry.extend(settings.query_modules.clone());
    let mut closure = BTreeMap::new();
    let mut pending = definitions.to_vec();
    while let Some(definition) = pending.pop() {
        for name in imports(&definition) {
            if closure.contains_key(&name) {
                continue;
            }
            let relative = registry
                .get(&ryo_benchmark_contracts::domain::requests::types::QueryModuleName(name.0.clone()))
                .ok_or_else(|| Error::Invalid(format!("{}: {}", constants::UNDECLARED_MODULE, name.0)))?;
            pending.push(RuleQuery(bench.suite.source_text(ArtifactPath(&bench.root.join(relative)))?.0.into()));
            let _previous = closure.insert(name, relative.clone());
        }
    }
    let mut staged: BTreeMap<_, _> = settings
        .query_modules
        .iter()
        .map(|(name, path)| (RegisteredModuleName(name.0.clone()), path.clone()))
        .collect();
    staged.extend(closure);
    Ok(staged)
}

/// Stage one implemented rule and answer its registration.
fn stage_rule(bench: &Bench<'_>, rule: &ryo::linter::RuleId) -> Result<RegisteredRule, Error> {
    let suite = bench.suite;
    let rules = bench.root.join(crate::domain::suite::constants::RULES);
    let settings_directory = settings_directory(suite, &bench.root, rule)?;
    let settings = suite.profile(ArtifactPath(&settings_directory))?;
    if settings.tool != bench.profile.tool
        || settings.version != bench.profile.version
        || settings.sha256 != bench.profile.sha256
    {
        return Err(Error::Invalid(constants::PINS_DIFFER.into()));
    }
    if !settings.defaults.contains(rule) {
        return Err(Error::Invalid(constants::NOT_SELECTED.into()));
    }
    let configuration = match &settings.parser_configuration {
        ParserSelection::Named(name) => ParserSelection::Named(name.clone()),
        ParserSelection::Unspecified => bench.profile.parser_configuration.clone(),
    };
    let cohort = cohort(bench, &settings, rule)?;
    let supporting: Vec<_> = cohort.iter().filter(|code| *code != rule).cloned().collect();
    let mut definitions = Vec::new();
    for code in &cohort {
        let query = suite
            .rule_query(ArtifactPath(&rules), code)?
            .ok_or_else(|| Error::Invalid(format!("{}: {code}", constants::NO_DEFINITION)))?;
        definitions.push(query);
    }
    let modules = module_closure(bench, &settings, &definitions)?;
    let stage = suite.ensure_directory(ArtifactPath(
        &bench.output.join(constants::SETTINGS_PROFILES).join(rule.as_ref()).join(constants::REPRO),
    ))?;
    for relative in modules.values() {
        super::evidence::service::copy(suite, &bench.root.join(relative), &stage.join(relative))?;
    }
    let mut applicability = BTreeMap::new();
    if !bench.profile.rule_applicability.is_empty() {
        for code in &cohort {
            let value = bench
                .profile
                .rule_applicability
                .get(&RuleLabel(code.to_string()))
                .ok_or_else(|| Error::Invalid(format!("{}: {code}", constants::NO_APPLICABILITY)))?;
            let _previous = applicability.insert(RuleName(code.to_string()), ApplicabilityText(value.0.clone()));
        }
    }
    let settings_file = settings_directory.join(PROFILE_MANIFEST);
    bench.inventory.stage_profile(
        &settings_file,
        &stage.join(PROFILE_MANIFEST),
        &ProfileProjection {
            defaults: vec![RuleName(rule.to_string())],
            supporting_rules: supporting.iter().map(|code| RuleName(code.to_string())).collect(),
            parser_configuration: configuration.clone(),
            query_modules: modules.clone(),
            rule_applicability: applicability,
        },
    )?;
    let source = rules.join(rule.as_ref());
    for file in [QUERY_FILE, CASES_FILE] {
        super::evidence::service::copy(suite, &source.join(file), &stage.join(crate::domain::suite::constants::RULES).join(rule.as_ref()).join(file))?;
    }
    for code in &supporting {
        super::evidence::service::copy(
            suite,
            &rules.join(code.as_ref()).join(QUERY_FILE),
            &stage.join(crate::domain::suite::constants::RULES).join(code.as_ref()).join(QUERY_FILE),
        )?;
    }
    let evidence_root =
        super::evidence::service::stage_evidence(suite, &bench.root, &settings_directory, rule, &stage)?;
    let cases = bench.inventory.case_inventory(&stage)?.0;
    let artifacts = bench
        .inventory
        .context_files(&stage)?
        .files
        .into_iter()
        .map(|path| Ok((path.clone(), digest(suite, &path)?)))
        .collect::<Result<BTreeMap<_, _>, Error>>()?;
    let instance = bench.inventory.instance(&RuleInstanceRecord {
        stage: stage.clone(),
        rule: RuleName(rule.to_string()),
        query_file: PathText(format!("{}/{rule}/{}", crate::domain::suite::constants::RULES, QUERY_FILE)),
        query_sha256: digest(suite, &source.join(QUERY_FILE))?,
        cases_file: PathText(format!("{}/{rule}/{}", crate::domain::suite::constants::RULES, CASES_FILE)),
        cases_sha256: digest(suite, &source.join(CASES_FILE))?,
        case_count: CaseCount(cases.len()),
        settings_source: repository_relative(&bench.repository, &settings_file),
        settings_source_sha256: digest(suite, &settings_file)?,
        parser_configuration: configuration,
        evidence_source_root: repository_relative(&bench.repository, &evidence_root),
        profile_sha256: digest(suite, &stage.join(PROFILE_MANIFEST))?,
        module_sha256: modules
            .iter()
            .map(|(name, relative)| Ok((name.clone(), digest(suite, &bench.root.join(relative))?)))
            .collect::<Result<_, Error>>()?,
    })?;
    Ok(RegisteredRule {
        definition: RuleDefinition::Pinned(pin(
            suite,
            stage.join(crate::domain::suite::constants::RULES).join(rule.as_ref()).join(QUERY_FILE),
        )?),
        modules: modules
            .iter()
            .map(|(name, relative)| Ok((name.clone(), pin(suite, stage.join(relative))?)))
            .collect::<Result<_, Error>>()?,
        artifacts,
        rule_id: RuleName(rule.to_string()),
        command: RuleCommand::Registered(Command::CaseVerification { context: stage }),
        reason: RuleReason::Unstated,
        cases,
        instance: RuleInstance::Declared(instance),
    })
}

/// A rule registered with only its reason.
fn unregistered(rule: &ryo::linter::RuleId, reason: ReasonText) -> RegisteredRule {
    RegisteredRule {
        definition: RuleDefinition::Unpinned,
        modules: BTreeMap::new(),
        artifacts: BTreeMap::new(),
        rule_id: RuleName(rule.to_string()),
        command: RuleCommand::Unregistered,
        reason: RuleReason::Stated(reason),
        cases: Vec::new(),
        instance: RuleInstance::Undeclared,
    }
}

/// A digest as the registration's identity writes it.
fn identity(value: &Sha256Text) -> CanonicalJson {
    CanonicalJson(render(&Fragment::Text(Label(value.0.clone()))).0)
}

/// Stage every default of the bench at `bench` under `output` and write its registration there; answer the registration's path.
///
/// # Errors
/// Refuses an unreadable bench profile and a registration that cannot be written; a rule that cannot be staged is
/// registered with its reason.
pub fn register(
    suite: &dyn SuitePort,
    inventory: &dyn InventoryPort,
    bench: &Path,
    reference: &Sealed,
    worker: &Sealed,
    output: &Path,
) -> Result<PathBuf, Error> {
    let root = suite.ensure_directory(ArtifactPath(bench))?;
    let output = suite.ensure_directory(ArtifactPath(output))?;
    let staged = Bench {
        suite,
        inventory,
        repository: root.ancestors().nth(constants::REPOSITORY_DEPTH).unwrap_or(&root).to_owned(),
        profile: suite.profile(ArtifactPath(&root))?,
        output: output.clone(),
        root,
    };
    let mut rules = Vec::new();
    for rule in &staged.profile.defaults {
        let query = suite.rule_query(ArtifactPath(&staged.root.join(crate::domain::suite::constants::RULES)), rule)?;
        rules.push(match query {
            None => unregistered(rule, ReasonText(constants::MISSING_IMPLEMENTATION.to_owned())),
            Some(_query) => stage_rule(&staged, rule).unwrap_or_else(|error| {
                unregistered(rule, ReasonText(format!("{}: {error}", constants::RULE_REGISTRATION)))
            }),
        });
    }
    let profile_sha256 = digest(suite, &staged.root.join(PROFILE_MANIFEST))?;
    let registration = Registration {
        authority: RegistrationAuthority::Execution {
            profile: staged.root.clone(),
            profile_sha256: profile_sha256.clone(),
            reference: ExecutablePin {
                path: reference.path.clone(),
                sha256: reference.sha256.clone(),
            },
            worker: ExecutablePin {
                path: worker.path.clone(),
                sha256: worker.sha256.clone(),
            },
        },
        provider: ProviderName(staged.profile.tool.0.clone()),
        defaults: staged.profile.defaults.iter().map(|rule| RuleName(rule.to_string())).collect(),
        rules,
        identity: BTreeMap::from([
            (IdentityKey(constants::INVENTORY_SHA256.to_owned()), identity(&profile_sha256)),
            (IdentityKey(WORKER_SHA256.to_owned()), identity(&worker.sha256)),
        ]),
    };
    let path = output.join(crate::domain::operations::constants::REGISTRATION);
    inventory.store(&path, &registration)?;
    Ok(path)
}
