// Cold benchmark image. Only the final finding stream is encoded here.
mod request;
use ryo::linter as sdk;
use ryo::query::{QueryModuleCatalog, QueryModuleSource, RyoQlQuery};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufWriter, Write},
    num::NonZeroUsize,
    path::PathBuf,
};

#[derive(Deserialize)]
struct Input {
    root: PathBuf,
    files: Vec<Box<str>>,
    selected_rules: Vec<String>,
    #[serde(default)]
    file_rules: BTreeMap<String, String>,
    query_modules: Option<BTreeMap<String, PathBuf>>,
    #[serde(default)]
    rule_observations: BTreeMap<String, Vec<request::Observation>>,
    rule_context: Option<request::Context>,
    project_search_roots: Option<Vec<Box<str>>>,
    parser_configuration: Option<String>,
    #[serde(default)]
    rule_applicability: BTreeMap<String, String>,
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
struct Finding {
    rule: String,
    file: String,
    start: u32,
    end: u32,
}

pub fn run(
    configured_provider: fn(Option<&str>, &std::path::Path) -> Result<ryo::languages::Language, String>,
    distinct_locations: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let (retain, args) = match args.split_first() {
        Some((mode, rest)) if mode == "--retain-findings" => (true, rest),
        _ => (false, args.as_slice()),
    };
    let [pack, input, output] = args else {
        return Err("expected [--retain-findings] PACK REQUEST OUTPUT".into());
    };
    let mut request: Input = serde_json::from_reader(fs::File::open(input)?)?;
    let pack = std::path::Path::new(pack);
    let languages = sdk::Languages(vec![configured_provider(
        request.parser_configuration.as_deref(), pack,
    )?]);
    let hosted = request
        .rule_context
        .map(|ctx| ctx.tables(&request.selected_rules))
        .transpose()?
        .unwrap_or_default();
    #[derive(Deserialize, Default)]
    struct Pack {
        #[serde(rename = "query-modules", default)]
        modules: BTreeMap<String, PathBuf>,
    }
    let sources = match request.query_modules {
        Some(modules) => modules,
        None => match fs::read_to_string(pack.join("benchmark.toml")) {
            Ok(text) => toml::from_str::<Pack>(&text)?.modules,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => return Err(error.into()),
        },
    };
    let mut modules = Vec::new();
    for (name, path) in sources {
        modules.push(QueryModuleSource::try_new(
            &name,
            &fs::read_to_string(pack.join(path))?,
        )?);
    }
    let catalog = QueryModuleCatalog::try_new(modules)?;
    let mut rules = BTreeMap::new();
    for name in request.selected_rules {
        let text = fs::read_to_string(pack.join("rules").join(&name).join("query.ryoql"))?;
        let query = RyoQlQuery::with_modules(&name, &text, &catalog)?;
        let applicability = match request.rule_applicability.get(&name).map(String::as_str) {
            Some("accepted-syntax") => sdk::RuleApplicability::AcceptedSyntax,
            None | Some("always") => sdk::RuleApplicability::Always,
            Some(other) => return Err(format!("unknown applicability {other}").into()),
        };
        let include = request
            .file_rules
            .iter()
            .filter(|(_, rule)| **rule == name)
            .map(|(path, _)| sdk::Glob::try_from(globset::escape(path).as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        if !request.file_rules.is_empty() && include.is_empty() {
            continue;
        }
        rules.insert(
            sdk::RuleId::try_from(name.as_str())?,
            sdk::Rule {
                query,
                applicability,
                observations: request
                    .rule_observations
                    .remove(&name)
                    .unwrap_or_default()
                    .into_iter()
                    .map(request::Observation::binding)
                    .collect::<Result<_, _>>()?,
                parameters: BTreeMap::new(),
                include,
                exclude: Vec::new(),
            },
        );
    }
    let cache = if retain {
        sdk::Cache::At(request.root.join(".ryo_cache"))
    } else {
        sdk::Cache::None
    };
    let request = sdk::LintRequest {
        root: request.root,
        corpus: sdk::Corpus::Listed(request.files.into_iter().map(sdk::RelativePath).collect()),
        rules,
        hosted,
        beside: Vec::new(),
        files: Vec::new(),
        given_project_inventory: Default::default(),
        project_search_scope: request.project_search_roots.map_or(
            sdk::ProjectSearchScope::Undeclared,
            |roots| {
                sdk::ProjectSearchScope::Declared(
                    roots.into_iter().map(sdk::RelativePath).collect(),
                )
            },
        ),
        source_language: sdk::SourceLanguage::Named(languages.0[0].name().clone()),
    };
    tm_project_session::link_executor();
    let engine = sdk::Linter::new(
        languages,
        sdk::Workers(NonZeroUsize::MIN).into(),
        cache,
    );
    eprintln!("HARNESS_STARTUP process_cpu={:.6}", cpu_time::ProcessTime::now().as_duration().as_secs_f64());
    let [answer]: [Result<sdk::LintReport, sdk::LintError>; 1] = engine
        .lint(vec![request])
        .try_into()
        .map_err(|_| "one request requires one answer")?;
    tm_project_session::print_measurement();
    let report = answer?;
    let mut findings = Vec::new();
    for row in report.findings {
        let get = |name: &str| {
            row.columns
                .iter()
                .find(|c| c.name.0.as_ref() == name)
                .map(|c| &c.value)
        };
        let sdk::Slot::Is(sdk::Cell::File(path)) = row.subject else {
            return Err("location stream requires a file subject".into());
        };
        let (start, end) = match get("span") {
            Some(sdk::Slot::Is(sdk::Cell::Span(span))) => (span.start.0, span.end.0),
            None => (0, 0),
            Some(_) => return Err("span column is not a byte range".into()),
        };
        findings.push(serde_json::to_vec(&Finding {
            rule: row.rule.to_string(),
            file: path.0.to_string(),
            start,
            end,
        })?);
    }
    findings.sort_unstable();
    if distinct_locations {
        findings.dedup();
    }
    let mut writer = BufWriter::new(fs::File::create(output)?);
    if findings.is_empty() {
        writer.write_all(b"\n")?;
    }
    for finding in findings {
        writer.write_all(&finding)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    // A partial report still contains established findings. Preserve them for
    // the stream comparison while returning a failure to the receipt gate.
    if !report.unknowns.is_empty() || !report.unread.is_empty() {
        return Err(format!("{} unknowns {:?}, {} unread {:?}",
            report.unknowns.len(), report.unknowns, report.unread.len(), report.unread).into());
    }
    Ok(())
}
