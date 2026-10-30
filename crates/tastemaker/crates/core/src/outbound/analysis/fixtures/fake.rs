//! The fake of the `analysis` port: the answers a test gives, by rule, and every request kept for the test to read.

use crate::domain::applications::types::{AnswerCell, CellText, CellValue, LintAnswer};
use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::domain::catalog::types::{
    CatalogColumn, CatalogEntries, CatalogEntry, CatalogName, CatalogText, Grammar, OperatorAnswer,
    OperatorPage, Production, RelationAnswer, RelationPage,
};
use crate::domain::definitions::types::QueryText;
use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::analysis::interface::AnalysisPort;
use crate::outbound::analysis::types::{
    AnalysisReport, AnalysisRequest, Corpus, Hosted, QueryVerdict,
};

/// The one relation the fake's catalog has: `file`, with one column, read by no language.
fn file_relation() -> RelationPage {
    RelationPage {
        entry: CatalogEntry {
            name: CatalogName("file".into()),
            summary: CatalogText("A file.".into()),
        },
        columns: vec![CatalogColumn {
            name: CatalogName("path".into()),
            kind: CatalogName("string".into()),
            words: Vec::new(),
        }],
        languages: Vec::new(),
    }
}

/// The one operator the fake's catalog has: `select`, with no field.
fn select_operator() -> OperatorPage {
    OperatorPage {
        entry: CatalogEntry {
            name: CatalogName("select".into()),
            summary: CatalogText("Rows of a relation.".into()),
        },
        fields: Vec::new(),
        options: Vec::new(),
        example: CatalogText("let found = select * from \"file\";".into()),
    }
}

/// What the query of the comparison's rule over the scopes of a file holds.
const MODULE_SCOPE: &str = "from \"scope\"";

/// One row a rule of the fake finds: its path and its key.
pub type Row = (Box<str>, Box<str>);

/// What the core's tests stand in for `analysis` with: a query is valid unless listed, and each rule whose query holds
/// a key of `rows` finds the rows given there, in every file it reads that the row names. Every file a rule over `scope`
/// names is read as a language of code that declares nothing would read it: its module scope, and nothing else.
#[derive(Debug, Default)]
pub struct AnalysisFake {
    /// The queries refused, by a text they hold.
    pub invalid: Vec<Box<str>>,
    /// The rows a rule finds, by a text its query holds: each `(path, key)`.
    pub rows: BTreeMap<Box<str>, Vec<Row>>,
    /// The queries the engine validates but cannot run, by a text they hold.
    pub unrunnable: Vec<Box<str>>,
    /// Whether the engine fails.
    pub fails: bool,
    /// Every lint request sent, in order.
    pub sent: Mutex<Vec<AnalysisRequest>>,
}

/// Whether the lint call reads `path`: listed or given by `corpus`. A rule's rows may name any file it reads, as ryo's do: which of
/// them the rule governs is the caller's to say.
fn listed(corpus: &Corpus, path: &str) -> bool {
    match corpus {
        Corpus::Walked => true,
        Corpus::Listed(paths) => paths.iter().any(|listed| &*listed.0 == path),
        Corpus::Given(files) => files.iter().any(|given| &*given.path.0 == path),
    }
}

impl AnalysisPort for AnalysisFake {
    fn validate(&self, query: &QueryText) -> Result<QueryVerdict, AnalysisError> {
        if query.0.trim().is_empty() || self.invalid.iter().any(|text| query.0.contains(&**text)) {
            return Ok(QueryVerdict::Invalid(
                crate::outbound::analysis::types::InvalidReason("refused by the fake".into()),
            ));
        }
        Ok(QueryVerdict::Valid)
    }

    fn lint(&self, request: AnalysisRequest) -> Result<AnalysisReport, AnalysisError> {
        self.sent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(request.clone());
        if self.fails {
            return Err(AnalysisError::Engine {
                reason: "the fake fails".to_owned(),
            });
        }
        let hosted = match &request.hosted {
            Hosted::Verdict(rows) => Some(super::answers::verdict_answers(rows)),
            Hosted::Counts(rows) => Some(super::answers::count_answers(rows)),
            Hosted::Nothing => None,
        };
        if let Some(findings) = hosted {
            return Ok(AnalysisReport {
                findings,
                ..AnalysisReport::default()
            });
        }
        if let Some(rule) = request.rules.iter().find(|rule| {
            self.unrunnable
                .iter()
                .any(|text| rule.query.0.contains(&**text))
        }) {
            return Err(AnalysisError::Rule {
                rule: rule.id.clone(),
                reason: "the fake cannot run it".to_owned(),
            });
        }
        let mut report = AnalysisReport::default();
        for rule in request
            .rules
            .iter()
            .filter(|rule| rule.query.0.contains(MODULE_SCOPE))
        {
            for glob in &rule.include {
                report.findings.push(LintAnswer {
                    rule: rule.id.clone(),
                    path: crate::domain::paths::types::RepoPath(glob.0.clone()),
                    cells: ["child", "parent"]
                        .map(|name| AnswerCell {
                            name: crate::domain::definitions::types::ColumnName(name.into()),
                            value: CellValue::Text(CellText("0".into())),
                        })
                        .into(),
                });
            }
        }
        for rule in &request.rules {
            for (text, rows) in &self.rows {
                if !rule.query.0.contains(&**text) {
                    continue;
                }
                for (path, key) in rows {
                    if !listed(&request.corpus, path) {
                        continue;
                    }
                    let file = crate::domain::paths::types::RepoPath(path.clone());
                    // The row's span, `start..end` as the fixture writes it, then, for a comment, whether it documents and
                    // its text: `start..end,doc,text`.
                    let mut parts = key.splitn(3, ',');
                    let span = parts.next().unwrap_or_default().split_once("..").and_then(
                        |(start, end)| {
                            Some(crate::domain::comments::types::ByteSpan {
                                start: crate::domain::findings::types::ByteOffset(
                                    start.parse().ok()?,
                                ),
                                end: crate::domain::findings::types::ByteOffset(end.parse().ok()?),
                            })
                        },
                    );
                    report.findings.push(LintAnswer {
                        rule: rule.id.clone(),
                        path: file.clone(),
                        cells: [
                            Some(AnswerCell {
                                name: crate::domain::definitions::types::ColumnName("file".into()),
                                value: CellValue::File(file),
                            }),
                            span.map(|span| AnswerCell {
                                name: crate::domain::definitions::types::ColumnName("span".into()),
                                value: CellValue::Span(span),
                            }),
                        ]
                        .into_iter()
                        .flatten()
                        .chain(["doc", "text"].into_iter().zip(parts).map(|(name, said)| {
                            AnswerCell {
                                name: crate::domain::definitions::types::ColumnName(name.into()),
                                value: CellValue::Text(CellText(said.into())),
                            }
                        }))
                        .collect(),
                    });
                }
            }
        }
        Ok(report)
    }

    fn relations(&self) -> Result<CatalogEntries, AnalysisError> {
        Ok(CatalogEntries(vec![file_relation().entry]))
    }

    fn relation(&self, name: &CatalogName) -> Result<RelationAnswer, AnalysisError> {
        let page = file_relation();
        Ok(if &page.entry.name == name {
            RelationAnswer::Found(page)
        } else {
            RelationAnswer::Unknown
        })
    }

    fn operators(&self) -> Result<CatalogEntries, AnalysisError> {
        Ok(CatalogEntries(vec![select_operator().entry]))
    }

    fn operator(&self, name: &CatalogName) -> Result<OperatorAnswer, AnalysisError> {
        let page = select_operator();
        Ok(if &page.entry.name == name {
            OperatorAnswer::Found(page)
        } else {
            OperatorAnswer::Unknown
        })
    }

    fn grammar(&self) -> Result<Grammar, AnalysisError> {
        Ok(Grammar(vec![Production {
            name: CatalogName("emit".into()),
            form: CatalogText("emit <step> subject <column> key(<columns>);".into()),
            example: CatalogText("emit found subject \"file\" key(\"span\");".into()),
        }]))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
