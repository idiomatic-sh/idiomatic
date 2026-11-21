//! The pack's cost check: every rule's query compiled by the query language, and the costs it answers held against the ceiling
//! and the list the manifest declares (`[cost]` of `benchmark.toml`). A rule that reaches a statement above the ceiling, or
//! above linear with sorts and not on the list, fails here, and so does a listed statement that rose or that no rule reaches
//! any more. The list names statements: a module's by its document, a rule's own by its rule.
#![cfg(test)]
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_query_language::domain::costing::service::held;
use ryo_query_language::domain::costing::types::{
    CostBreach, ListedSite, PackCeiling, PlanCostClass, RuleCost, SiteOrigin, SiteOwner,
};
use ryo_query_language::inbound::use_cases::compile::interface::CompileUseCase as _;
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_values::domain::program::service::{
    document_source, module_catalog, query_bundle_with_catalog, source_id,
};
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_requests::types::QueryRequest;
use ryo_values::domain::query_tokens::types::{QuerySourceId, QuerySpelling, StepSpelling};
use ryo_values::domain::runs::types::RuleId;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

type Input = ();
type Value = Cow<'static, str>;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_rules_hold_the_ceiling_and_the_list_the_manifest_declares"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Cow::Borrowed("held")),
}];

/// The text of a file of the pack.
fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The class a manifest writes, in the words the query language prints a class in.
fn class_named(text: &str) -> PlanCostClass {
    [
        PlanCostClass::Linear,
        PlanCostClass::LinearWithSorts,
        PlanCostClass::QuadraticInGroup,
        PlanCostClass::QuadraticInFile,
    ]
    .into_iter()
    .find(|class| class.to_string() == text)
    .unwrap_or_else(|| panic!("`{text}` names no cost class"))
}

/// The name a rule's own query is compiled under, as the benchmark's worker names it.
fn document_of(rule: &str) -> QuerySourceId {
    source_id(QuerySpelling(&format!("rules/{rule}"))).expect("a rule has a name")
}

/// What the manifest declares of the pack's cost: its ceiling and its list. An entry names a module's statement by `document`,
/// or a rule's own by `rule`.
fn declared(manifest: &toml::Value) -> PackCeiling {
    let cost = &manifest["cost"];
    let text = |entry: &toml::Value, key: &str| {
        entry[key]
            .as_str()
            .unwrap_or_else(|| panic!("`{key}` of the manifest's cost is text"))
            .to_owned()
    };
    PackCeiling {
        ceiling: class_named(&text(cost, "ceiling")),
        listed: cost["listed"]
            .as_array()
            .expect("the manifest lists its sites")
            .iter()
            .map(|entry| {
                let (document, owner) = match entry.get("rule") {
                    Some(rule) => {
                        let rule = rule.as_str().expect("a rule is named by text");
                        (document_of(rule), SiteOwner::Rule(RuleId(rule.into())))
                    }
                    None => (
                        QuerySourceId(text(entry, "document").into()),
                        SiteOwner::Pack,
                    ),
                };
                ListedSite {
                    site: SiteOrigin {
                        document,
                        statement: StepSpelling(text(entry, "statement").into()),
                    },
                    owner,
                    class: class_named(&text(entry, "class")),
                }
            })
            .collect(),
    }
}

/// The cost of every rule of the pack: each directory of `rules` is a rule, compiled with the modules the manifest names.
fn costs(root: &Path, manifest: &toml::Value) -> Vec<RuleCost> {
    let name = |text: &str| source_id(QuerySpelling(text)).expect("a document has a name");
    let catalog = module_catalog(
        manifest["query-modules"]
            .as_table()
            .expect("the manifest names its modules")
            .iter()
            .map(|(module, path)| {
                let path = path.as_str().expect("a module's path is text");
                document_source(name(module), read(root.join(path)))
            }),
    )
    .expect("the modules' names differ");
    let compiler = CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    );
    let mut rules = std::fs::read_dir(root.join("rules"))
        .expect("the pack has rules")
        .map(|entry| {
            entry
                .expect("a rule is read")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    rules.sort();
    let mut modules = ParsedModules::default();
    rules
        .iter()
        .map(|rule| {
            let query = read(root.join("rules").join(rule).join("query.ryoql"));
            let bundle = query_bundle_with_catalog(
                document_source(document_of(rule), query),
                catalog.clone(),
            )
            .expect("no module has a rule's name");
            let report = compiler
                .compile(QueryRequest {
                    text: bundle,
                    hosted: Vec::new(),
                    observations: Vec::new(),
                    modules: modules.clone(),
                })
                .unwrap_or_else(|error| panic!("{rule}: {error}"));
            modules = report.modules;
            RuleCost {
                rule: RuleId(rule.as_str().into()),
                cost: report.cost,
            }
        })
        .collect()
}

fn subject((): Input) -> Result<Value, Failure> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Value =
        toml::from_str(&read(root.join("benchmark.toml"))).expect("the manifest is TOML");
    let (costs, pack) = (costs(root, &manifest), declared(&manifest));
    assert!(!costs.is_empty(), "a pack with no rule holds nothing");
    // One line for each breach, the rules that share one counted: a module's statement is one site for every rule.
    let mut breaches: BTreeMap<String, usize> = BTreeMap::new();
    for breach in held(&costs, &pack) {
        let site = |site: &SiteOrigin| format!("`{}` of `{}`", site.statement, site.document.0);
        let reached = |rule: &RuleId, at: &SiteOrigin| {
            let class = costs
                .iter()
                .filter(|cost| cost.rule == *rule)
                .flat_map(|cost| &cost.cost.findings)
                .filter(|finding| finding.site == *at)
                .map(|finding| finding.class)
                .max()
                .expect("a breach names a finding of its rule");
            format!("{} is {class}", site(at))
        };
        let line = match &breach {
            CostBreach::AboveCeiling(rule, at) => {
                format!("above the ceiling: {}", reached(rule, at))
            }
            CostBreach::Unlisted(rule, at) => format!("not listed: {}", reached(rule, at)),
            CostBreach::Rose(listed) => {
                format!("rose above `{}`: {}", listed.class, site(&listed.site))
            }
            CostBreach::Stale(listed) => format!("stale: {}", site(&listed.site)),
        };
        *breaches.entry(line).or_default() += 1;
    }
    // A rule added or a site taken off the list changes nothing here: the answer is that the pack holds, or each breach.
    Ok(if breaches.is_empty() {
        Cow::Borrowed("held")
    } else {
        let lines = breaches
            .iter()
            .map(|(line, rules)| format!("{line} ({rules})"));
        Cow::Owned(lines.collect::<Vec<_>>().join("\n"))
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
