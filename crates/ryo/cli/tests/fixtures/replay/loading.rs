//! Every rule ryo is given, loaded as `ryo lint` loads it: compiled by the compilation it composes and prepared by its execution,
//! alone, over no file. The rules are each rule of the Ruff, oxlint and clippy benchmarks and the live version of every
//! Tastemaker rule.

use crate::{base, rule_fixtures};
use ryo_core::inbound::use_cases::lint::types::{ParameterValue, Rule, RuleParameter};
use ryo_core::outbound::language::interface::LanguagePort as _;
use ryo_execution::inbound::use_cases::compile_rules::interface::CompileRulesUseCase as _;
use ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest;
use ryo_execution::inbound::use_cases::compile_rules::use_case::CompileRulesUseCaseImpl;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::prepare::interface::{
    ObservationIdentity, PrepareUseCase as _,
};
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation,
};
use ryo_values::domain::observations::types::{
    ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
};
use ryo_values::domain::plan_parts::types::{FlagState, LiteralText, ParameterName, WholeNumber};
use ryo_values::domain::program::service::{document_source, query_bundle_with_catalog, source_id};
use ryo_values::domain::query_tokens::types::{
    ModuleCatalog, QueryBundle, QueryDocumentSource, QuerySourceId, QuerySpelling,
};
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The benchmarks whose rules ryo ships, each a linter directory holding `benchmark.toml` and `rules/<rule>/query.ryoql`.
pub(crate) const BENCHMARKS: [&str; 3] = [
    "languages/python/benchmarks/linters/ruff",
    "languages/typescript/benchmarks/linters/oxlint",
    "languages/rust/benchmarks/linters/clippy",
];

/// How a rule fared: planned, refused by the plan's growth check, or not loaded for another reason, with the reason.
pub(crate) enum Loaded {
    Planned,
    Refused(String),
    Unloaded(String),
}

/// The compilation and the execution `ryo lint` composes, over the languages it links.
pub(crate) struct Production {
    compilation: CompileRulesUseCaseImpl,
    execution: ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl,
    vocabularies: ryo_values::domain::descriptions::types::KindVocabularies,
}

impl Production {
    pub(crate) fn new() -> Self {
        let compile = Arc::new(
            ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(
                Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
                Arc::new(ryo_query_language::outbound::catalog::factory::language()),
            ),
        );
        let typecheck = Arc::new(
            ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
                Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
                Arc::new(ryo_query_language::outbound::catalog::factory::language()),
            ),
        );
        let operators = ryo_execution_runtime::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl::new(),
            ryo_operators::inbound::use_cases::run::use_case::RunUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            )),
        );
        Self {
            compilation: CompileRulesUseCaseImpl::new(
                Arc::new(ryo_execution::outbound::query_language::factory::use_case(
                    compile, typecheck,
                )),
                Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
            ),
            execution:
                ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl::new(
                    Arc::new(operators),
                ),
            vocabularies: ryo_core::outbound::language::factory::use_case(base::replay_languages())
                .kind_vocabularies(),
        }
    }

    /// `rule` compiled and prepared alone, over the relations `hosted` declares.
    pub(crate) fn load(&self, rule: &Rule, hosted: &[HostRelation]) -> Loaded {
        let spec = RuleSpec::from(rule);
        let compiled = match self.compilation.compile_rules(CompileRequest {
            rules: std::slice::from_ref(&spec),
            hosted,
            vocabularies: &self.vocabularies,
        }) {
            Ok(compiled) => compiled,
            Err(error) => return Loaded::Unloaded(error.to_string()),
        };
        match self.execution.prepare(PrepareRequest {
            plans: PlanSource::Compiled(compiled),
            rules: vec![spec],
            digests: vec![ryo_values::domain::stored_facts::types::Digest([0; 16])],
            hosted: hosted.to_vec(),
            observations: ObservationIdentity::Independent,
        }) {
            Ok(_) => Loaded::Planned,
            Err(error @ OperatingError::Superlinear { .. }) => Loaded::Refused(error.to_string()),
            Err(error) => Loaded::Unloaded(error.to_string()),
        }
    }
}

/// A rule of `query` named `id`, its modules `modules`, over every file.
fn rule(id: &str, query: QueryBundle) -> Rule {
    Rule {
        observations: Vec::new(),
        id: RuleId(id.into()),
        query,
        applicability: RuleApplicability::Always,
        parameters: Vec::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

/// A rule of the query `text` alone, named `id`, over every file.
pub(crate) fn query_rule(id: &str, text: &str) -> Rule {
    rule(
        id,
        QueryBundle::from(ryo_values::domain::plan_parts::types::QueryText(
            text.into(),
        )),
    )
}

/// The root of this repository's ryo workspace.
pub(crate) fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Each rule of the benchmark at `linter`: its query with the modules its `[query-modules]` registers, the observations its
/// `rule-observations` declares (no producer: each producer plans as its own rule), and the tables its `rule-context` names.
pub(crate) fn benchmark_rules(linter: &Path) -> Vec<(Rule, Vec<HostRelation>)> {
    let profile: toml::Table = std::fs::read_to_string(linter.join("benchmark.toml"))
        .expect("the benchmark's profile")
        .parse()
        .expect("a profile that reads");
    let modules: BTreeMap<QuerySourceId, QueryDocumentSource> = profile
        .get("query-modules")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flatten()
        .map(|(name, path)| {
            let id = source_id(QuerySpelling(name)).expect("a module name");
            let text = std::fs::read_to_string(linter.join(path.as_str().expect("a module path")))
                .expect("the module");
            (id.clone(), document_source(id, text))
        })
        .collect();
    let modules = Arc::new(modules);
    let hosted = context_tables(&profile);
    let mut codes: Vec<String> = std::fs::read_dir(linter.join("rules"))
        .expect("the benchmark's rules")
        .map(|entry| {
            entry
                .expect("a rule")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    codes.sort();
    codes
        .into_iter()
        .map(|code| {
            let text =
                std::fs::read_to_string(linter.join("rules").join(&code).join("query.ryoql"))
                    .expect("the rule's query");
            let root = document_source(
                source_id(QuerySpelling(&format!("rules/{code}"))).expect("a rule id"),
                text,
            );
            let query = query_bundle_with_catalog(root, ModuleCatalog(Arc::clone(&modules)))
                .expect("the rule's modules");
            let mut each = rule(&code, query);
            each.observations = observations(&profile, &code);
            (each, hosted.clone())
        })
        .collect()
}

/// The tables a benchmark's `rule-context` names, each declared with the columns the benchmark hosts it with and no row.
fn context_tables(profile: &toml::Table) -> Vec<HostRelation> {
    let Some(tables) = profile
        .get("rule-context")
        .and_then(|context| context.get("tables"))
        .and_then(toml::Value::as_table)
    else {
        return Vec::new();
    };
    tables
        .iter()
        .map(|(table, name)| {
            let columns: &[(&str, HostKind)] = match table.as_str() {
                "selected" => &[("code", HostKind::Text)],
                "disabled" => &[("file", HostKind::File), ("code", HostKind::Text)],
                "external" => &[("prefix", HostKind::Text)],
                other => panic!("a rule-context table this test does not declare: {other}"),
            };
            HostRelation {
                name: HostName(name.as_str().expect("a table name").into()),
                columns: columns
                    .iter()
                    .map(|(column, kind)| HostColumn {
                        name: HostColumnName((*column).into()),
                        kind: *kind,
                    })
                    .collect(),
                rows: Vec::new(),
            }
        })
        .collect()
}

/// The observations the profile's `rule-observations` declares for `code`, with no producer.
fn observations(profile: &toml::Table, code: &str) -> Vec<ObservationBinding> {
    profile
        .get("rule-observations")
        .and_then(|declared| declared.get(code))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .map(|binding| ObservationBinding {
            name: ObservationBindingName(
                binding
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .expect("a name")
                    .into(),
            ),
            columns: binding
                .get("columns")
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .map(|column| ObservationColumn {
                    name: ObservationColumnName(
                        column
                            .get("name")
                            .and_then(toml::Value::as_str)
                            .expect("a column")
                            .into(),
                    ),
                    kind: match column.get("kind").and_then(toml::Value::as_str) {
                        Some("node") => CellKind::Node,
                        Some("symbol") => CellKind::Symbol,
                        Some("file") => CellKind::File,
                        Some("span") => CellKind::Span,
                        Some("number") => CellKind::Number,
                        Some("flag") => CellKind::Flag,
                        other => panic!("an observation kind this test does not read: {other:?}"),
                    },
                })
                .collect(),
            producers: Vec::new(),
        })
        .collect()
}

/// The live version of every Tastemaker rule that has a query: the highest version of each rule and benchmark rule, and each
/// rule of the highest version of each spec. Each is bound to the params of its `[given]` and its first scenario, over
/// `tastemaker.placed` when it reads it.
pub(crate) fn tastemaker_rules() -> Vec<(String, Rule, Vec<HostRelation>)> {
    let package = base::rules::Roots::here().package;
    let mut directories: Vec<PathBuf> = Vec::new();
    for kept in ["rules", "benchmarks/rules"] {
        for entry in std::fs::read_dir(package.join(kept)).expect("the package's rules") {
            let entry = entry.expect("a rule of the package");
            if entry.path().is_dir() {
                directories.push(base::rules::latest_version(&entry.path()));
            }
        }
    }
    for spec in std::fs::read_dir(package.join("specs")).expect("the package's specs") {
        let spec = spec.expect("a spec of the package");
        // A spec that holds no rule of its own has no `rules` directory.
        let rules = base::rules::latest_version(&spec.path()).join("rules");
        if !rules.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(rules).expect("the spec's rules") {
            directories.push(entry.expect("a rule of the spec").path());
        }
    }
    directories.sort();
    let empty = tempfile::tempdir().expect("a directory with no file");
    directories
        .into_iter()
        .filter_map(|directory| {
            let table: toml::Table = std::fs::read_to_string(directory.join("rule.toml"))
                .expect("the rule's rule.toml")
                .parse()
                .expect("a rule that reads");
            let query = table.get("query").and_then(toml::Value::as_str)?.to_owned();
            let mut given = table
                .get("given")
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default();
            let first = table
                .get("scenarios")
                .and_then(toml::Value::as_array)
                .and_then(|scenarios| scenarios.first())
                .and_then(|scenario| scenario.get("given"))
                .and_then(|given| given.get("params"))
                .and_then(toml::Value::as_table);
            if let Some(params) = first {
                match given.get_mut("params") {
                    Some(toml::Value::Table(shared)) => shared.extend(params.clone()),
                    _ => drop(given.insert("params".into(), toml::Value::Table(params.clone()))),
                }
            }
            let name = directory
                .strip_prefix(&package)
                .unwrap_or(&directory)
                .to_string_lossy()
                .into_owned();
            let mut each = rule(
                &name,
                QueryBundle::from(ryo_values::domain::plan_parts::types::QueryText(
                    query.as_str().into(),
                )),
            );
            each.parameters = rule_fixtures::parameters_of(Some(&given));
            each.parameters.extend(unbound(&query, &each.parameters));
            let hosted = if query.contains(rule_fixtures::PLACED) {
                vec![rule_fixtures::placed_manifests(empty.path())]
            } else {
                Vec::new()
            };
            Some((name, each, hosted))
        })
        .collect()
}

/// A value for each param `query` declares (`param <name>: <type>;`) that `bound` lacks: the empty value of its type, which a
/// placement fills. The plan of a rule reads a param's type, not its rows.
fn unbound(query: &str, bound: &[RuleParameter]) -> Vec<RuleParameter> {
    query
        .lines()
        .filter_map(|line| line.trim().strip_prefix("param "))
        .filter_map(|declared| declared.trim_end().strip_suffix(';')?.split_once(':'))
        .map(|(name, kind)| (name.trim(), kind.trim()))
        .filter(|(name, _)| !bound.iter().any(|each| each.name.0.as_ref() == *name))
        .map(|(name, kind)| RuleParameter {
            name: ParameterName(name.into()),
            value: match kind {
                "list<table>" => ParameterValue::Tables(Vec::new()),
                "set<string>" => ParameterValue::Texts(Vec::new()),
                "list<set<string>>" => ParameterValue::TextSets(Vec::new()),
                "string" => ParameterValue::Text(LiteralText("".into())),
                "integer" => ParameterValue::Number(WholeNumber(0)),
                "boolean" => ParameterValue::Flag(FlagState::Lowered),
                other => panic!("a param type this test does not bind: {other}"),
            },
        })
        .collect()
}
