//! The shipping SDK reference, using one worker and a fresh in-memory run.
use ryo::linter::*;
use ryo::query::RyoQlQuery;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

#[derive(serde::Deserialize)]
struct Request {
    root: PathBuf,
    #[serde(default)]
    given: bool,
    files: Vec<String>,
    selected_rules: Vec<String>,
    #[serde(default)]
    parameters: BTreeMap<String, Value>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    inventory: Option<Vec<String>>,
    #[serde(default)]
    hosted: Vec<Hosted>,
}

#[derive(serde::Deserialize)]
struct Hosted {
    name: String,
    columns: Vec<(String, String)>,
    rows: Vec<Vec<Value>>,
}

impl Hosted {
    fn into_relation(self) -> HostRelation {
        let columns = self
            .columns
            .into_iter()
            .map(|(name, kind)| HostColumn {
                name: HostColumnName(name.into()),
                kind: match kind.as_str() {
                    "File" => HostKind::File,
                    "Text" => HostKind::Text,
                    "Number" => HostKind::Number,
                    _ => panic!("unknown host column kind {kind}"),
                },
            })
            .collect::<Vec<_>>();
        let rows = self
            .rows
            .into_iter()
            .map(|row| {
                assert_eq!(row.len(), columns.len(), "host row width");
                HostRow(
                    row.into_iter()
                        .zip(&columns)
                        .map(|(v, col)| match col.kind {
                            HostKind::File => Cell::File(RelativePath(
                                v["file"].as_str().expect("file cell").into(),
                            )),
                            HostKind::Text => {
                                Cell::Text(HostText(v.as_str().expect("text cell").into()))
                            }
                            HostKind::Number => {
                                Cell::Number(HostNumber(v.as_i64().expect("number cell")))
                            }
                            _ => panic!("unsupported hosted column"),
                        })
                        .collect(),
                )
            })
            .collect();
        HostRelation {
            name: HostRelationName(self.name.into()),
            columns,
            rows,
        }
    }
}

fn value(slot: &Slot) -> Value {
    match slot {
        Slot::Nothing => Value::Null,
        Slot::Is(Cell::File(file)) => json!({"file": &*file.0}),
        Slot::Is(Cell::Text(text)) => json!(&*text.0),
        Slot::Is(Cell::Number(number)) => json!(number.0),
        Slot::Is(Cell::Span(span)) => json!({"span": [span.start.0, span.end.0]}),
        other => panic!("unrepresented result cell {other:?}"),
    }
}

fn parameter(v: &Value) -> ParameterValue {
    let text = |v: &Value| ParameterText(v.as_str().expect("text parameter").into());
    match v {
        Value::String(_) => ParameterValue::Text(text(v)),
        Value::Number(n) => {
            ParameterValue::Number(ParameterNumber(n.as_i64().expect("whole number")))
        }
        Value::Bool(b) => ParameterValue::Flag(if *b { FlagState::On } else { FlagState::Off }),
        Value::Array(items) if items.iter().all(Value::is_string) => {
            ParameterValue::Texts(items.iter().map(text).collect())
        }
        Value::Array(items) if items.iter().all(Value::is_i64) => ParameterValue::Numbers(
            ParameterNumbers(items.iter().map(|n| n.as_i64().unwrap()).collect()),
        ),
        Value::Array(items) if items.iter().all(Value::is_array) => ParameterValue::TextSets(
            items
                .iter()
                .map(|a| a.as_array().unwrap().iter().map(text).collect())
                .collect(),
        ),
        Value::Array(items) if items.iter().all(Value::is_object) => ParameterValue::Tables(
            items
                .iter()
                .map(|m| {
                    ParameterTable(
                        m.as_object()
                            .unwrap()
                            .iter()
                            .map(|(k, v)| ParameterCell {
                                key: ParameterText(k.as_str().into()),
                                value: text(v),
                            })
                            .collect(),
                    )
                })
                .collect(),
        ),
        _ => panic!("unrepresented parameter {v}"),
    }
}

pub(super) fn run(languages: Languages, pack: &Path, request: &Path, output: &Path) {
    let req: Request = serde_json::from_slice(&std::fs::read(request).expect("request")).unwrap();
    let globs = |v: &[String]| {
        v.iter()
            .map(|s| Glob::try_from(s.as_str()).expect("glob"))
            .collect()
    };
    let rules = req
        .selected_rules
        .iter()
        .map(|id| {
            let query =
                std::fs::read_to_string(pack.join("rules").join(id).join("query.ryoql")).unwrap();
            (
                RuleId::try_from(id.as_str()).unwrap(),
                Rule {
                    observations: Vec::new(),
                    query: RyoQlQuery::try_from(query.as_str()).unwrap(),
                    applicability: RuleApplicability::Always,
                    parameters: req
                        .parameters
                        .iter()
                        .map(|(k, v)| (ParameterName(k.as_str().into()), parameter(v)))
                        .collect(),
                    include: globs(&req.include),
                    exclude: globs(&req.exclude),
                },
            )
        })
        .collect();
    let files = req.inventory.unwrap_or(req.files);
    let corpus = if req.given {
        Corpus::Given(
            files
                .iter()
                .map(|file| CaseSource {
                    executable: ExecutablePermission::Unknown,
                    path: RelativePath(file.as_str().into()),
                    text: SourceText(
                        std::fs::read_to_string(req.root.join(file))
                            .expect("case text")
                            .into(),
                    ),
                })
                .collect(),
        )
    } else {
        Corpus::Listed(files.into_iter().map(|f| RelativePath(f.into())).collect())
    };
    let report = lint(
        languages,
        &WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        Cache::None,
        LintRequest {
            source_language: SourceLanguage::Recognized,
            root: req.root,
            project_search_scope: Default::default(),
            given_project_inventory: Default::default(),
            rules,
            files: Vec::new(),
            hosted: req.hosted.into_iter().map(Hosted::into_relation).collect(),
            corpus,
            beside: Vec::new(),
        },
    )
    .expect("shipping query");
    assert!(
        report.unknowns.is_empty(),
        "shipping unknowns: {:?}",
        report.unknowns
    );
    let mut rows: Vec<_> = report
        .findings
        .iter()
        .map(|finding| {
            let columns: Vec<_> = finding
                .columns
                .iter()
                .map(|col| (&*col.name.0, value(&col.value)))
                .collect();
            json!({"columns": columns, "key": finding.key.0.iter().map(value).collect::<Vec<_>>(),
            "rule": finding.rule.to_string(), "subject": value(&finding.subject)})
            .to_string()
        })
        .collect();
    rows.sort();
    std::fs::write(output, rows.join("\n") + "\n").expect("write SDK rows");
    println!(
        "shipping findings {} unread {:?}",
        rows.len(),
        report.unread
    );
}
