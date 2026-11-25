//! Source coordinates of statements within a Python suite, and parsed components of import aliases.
use super::{ColumnFile, Kind, Node, Numbered, nodes_of_kind};
use crate::domain::node::service::id_of;
use ruff_python_ast::AnyNodeRef;
use ruff_text_size::Ranged;
use ryo::languages::{self as l, Position, node_table};
use std::collections::HashSet;

/// The parser's import names and their original spelling at one alias.
#[derive(Debug)]
pub struct ImportAlias<'a> {
    /// Alias occurrence.
    pub id: u32,
    /// Enclosing import statement.
    pub statement: u32,
    /// Zero-based position in the names list.
    pub index: usize,
    /// Source offset of the previous name in this list.
    pub previous_start: Option<u32>,
    /// Number of aliases written in the statement.
    pub count: usize,
    /// Number of explicit as-names in the statement.
    pub renames: usize,
    /// Import or from-import syntax.
    pub form: &'static str,
    /// Canonical qualified source module, including relative dots.
    pub module: &'a str,
    /// Leading relative levels.
    pub dots: &'a str,
    /// Module identifier as written, with the requested relative levels.
    pub written_module: &'a str,
    /// Canonical imported name.
    pub name: &'a str,
    /// Canonical as-name, empty if absent.
    pub rename: &'a str,
    /// Source spelling of the imported identifier.
    pub written_name: &'a str,
    /// Source spelling of the as-name, empty if absent.
    pub written_rename: &'a str,
    /// Requested module and member path.
    pub target: &'a str,
}

impl ColumnFile<'_, '_> {
    /// Each import or from-import statement supplies one statement-layout row.
    pub fn import_statement_count(&self) -> usize {
        self.import_presence.statements
    }

    /// A statement is an import exactly when the parser produced an import or from-import node.
    pub fn is_import_statement(&self, id: Position) -> bool {
        matches!(
            self.file.syntax.nodes.get(usize::from(id)),
            Some(AnyNodeRef::StmtImport(_) | AnyNodeRef::StmtImportFrom(_))
        )
    }

    /// Standard-library membership is indexed by the file's selected Python release.
    pub fn import_release(&self) -> (i64, i64) {
        (self.file.target.major.0, self.file.target.minor.0)
    }

    fn import_statements(&self) -> impl Iterator<Item = Numbered<'_>> {
        [Node::Import, Node::ImportFrom]
            .into_iter()
            .flat_map(move |kind| nodes_of_kind(&self.file.syntax, Kind::Node(kind)))
    }

    /// The lexical walk saw a physical comment within an import's lines.
    pub fn can_have_import_line_comments(&self) -> bool {
        self.import_presence.line_comment
    }

    /// A comment occupies a physical line. Report every import whose source extent reaches that line before the comment.
    pub fn visit_import_line_comments(&self, mut visit: impl FnMut(Position, Position, bool)) {
        use crate::domain::{syntax::service::kind_bits, tokens::types::Token};
        if !self.can_have_import_line_comments() {
            return;
        }
        let comments = node_table::ids_of_kind(self.table, kind_bits(Kind::Token(Token::Comment)));
        if comments.is_empty() {
            return;
        }
        let src = self.file.syntax.text.0.as_bytes();
        let line_end = |at: usize| {
            at + src[at..]
                .iter()
                .position(|b| matches!(b, b'\r' | b'\n'))
                .unwrap_or(src.len() - at)
        };
        for Numbered { id, .. } in self.import_statements() {
            let statement = node_table::span_of(self.table, id);
            let first = comments
                .partition_point(|p| node_table::span_of(self.table, *p).start < statement.start);
            let end = statement.end.0 as usize;
            let mut first_line_end = None;
            for &comment in &comments[first..] {
                let at = node_table::span_of(self.table, comment).start.0 as usize;
                if at >= end && src[end..at].iter().any(|b| matches!(b, b'\r' | b'\n')) {
                    break;
                }
                let first_end =
                    *first_line_end.get_or_insert_with(|| line_end(statement.start.0 as usize));
                visit(id, comment, at < first_end);
            }
        }
    }

    /// Catalog membership of an absolute module root for the file's configured Python release.
    pub fn import_catalog(&self, root: &str) -> &'static str {
        use crate::domain::standard_library::{
            service::contains,
            types::{ModulePath, StandardLibraryMembership},
        };
        match contains(self.file.target, ModulePath(root)) {
            StandardLibraryMembership::Standard => "standard",
            StandardLibraryMembership::Other => "other",
            StandardLibraryMembership::UnverifiedRelease => "unknown",
        }
    }

    /// Each import statement occupies one ordered position in the suite denoted by its parent and field.
    fn visit_import_suites(&self, mut visit: impl FnMut(&[Position])) {
        let mut seen = HashSet::new();
        let mut suite = Vec::new();
        for Numbered { id, .. } in self.import_statements() {
            let parent = node_table::parent_of(self.table, id);
            let field = node_table::field_of(self.table, id);
            if seen.insert((parent.0, field.0)) {
                suite.clear();
                suite.extend(
                    node_table::children_of(self.table, parent)
                        .iter()
                        .copied()
                        .filter(|p| {
                            node_table::field_of(self.table, *p) == field
                                && self
                                    .file
                                    .syntax
                                    .nodes
                                    .get(usize::from(*p))
                                    .is_some_and(|n| n.is_statement())
                        }),
                );
                if !suite.is_empty() {
                    visit(&suite);
                }
            }
        }
    }

    /// An import requests a dotted path; the from-clause's leading dots select its relative level and the alias's as-name gives its binding spelling.
    pub fn visit_import_aliases(&self, mut visit: impl FnMut(ImportAlias<'_>)) {
        self.visit_import_names(self.import_statements().enumerate(), false, |_, alias| {
            visit(alias)
        });
    }

    /// The grammar fixes the first alias of each import statement.
    pub fn visit_import_heads(&self, mut visit: impl FnMut(ImportAlias<'_>)) {
        self.visit_import_names(self.import_statements().enumerate(), true, |_, alias| {
            visit(alias)
        });
    }

    /// Read heads in the given suite order. An invalid import can lack its first alias.
    pub fn visit_ordered_import_heads(
        &self,
        statements: impl Iterator<Item = Position>,
        visit: impl FnMut(usize, ImportAlias<'_>),
    ) {
        let nodes = statements.enumerate().filter_map(|(row, id)| {
            self.file
                .syntax
                .nodes
                .get(usize::from(id))
                .map(|node| (row, Numbered { id, node: *node }))
        });
        self.visit_import_names(nodes, true, visit);
    }

    fn visit_import_names<'n>(
        &self,
        statements: impl Iterator<Item = (usize, Numbered<'n>)>,
        first_only: bool,
        mut visit: impl FnMut(usize, ImportAlias<'_>),
    ) {
        let text = self.file.syntax.text.0;
        let mut source_module = String::new();
        let mut target = String::new();
        let mut written_module = String::new();
        for (
            row,
            Numbered {
                id: statement,
                node,
            },
        ) in statements
        {
            let (form, level, module, aliases) = match node {
                AnyNodeRef::StmtImport(s) => ("import", 0, None, &s.names),
                AnyNodeRef::StmtImportFrom(s) => {
                    ("import-from", s.level, s.module.as_ref(), &s.names)
                }
                _ => continue,
            };
            source_module.clear();
            source_module.extend(std::iter::repeat_n('.', level as usize));
            if let Some(module) = module {
                source_module.push_str(module.as_str());
            }
            written_module.clear();
            written_module.push_str(&source_module[..level as usize]);
            if let Some(m) = module {
                written_module.push_str(
                    text.get(usize::from(m.start())..usize::from(m.end()))
                        .unwrap_or_default(),
                );
            }
            let renames = aliases.iter().filter(|a| a.asname.is_some()).count();
            let names = if first_only {
                &aliases[..aliases.len().min(1)]
            } else {
                &aliases[..]
            };
            for (index, alias) in names.iter().enumerate() {
                let Some(id) = id_of(alias) else { continue };
                let name = alias.name.as_str();
                let written = |ident: &ruff_python_ast::Identifier| {
                    text.get(usize::from(ident.start())..usize::from(ident.end()))
                        .unwrap_or_default()
                };
                target.clear();
                target.push_str(&source_module);
                if name != "*" {
                    if module.is_some() {
                        target.push('.');
                    }
                    target.push_str(name);
                }
                visit(
                    row,
                    ImportAlias {
                        id: id.0,
                        statement: statement.0,
                        index,
                        previous_start: index.checked_sub(1).map(|i| u32::from(aliases[i].start())),
                        count: aliases.len(),
                        renames,
                        form,
                        module: if form == "import" {
                            name
                        } else {
                            &source_module
                        },
                        dots: &source_module[..level as usize],
                        written_module: if form == "import" {
                            written(&alias.name)
                        } else {
                            &written_module
                        },
                        name,
                        rename: alias.asname.as_ref().map_or("", |n| n.as_str()),
                        written_name: written(&alias.name),
                        written_rename: alias.asname.as_ref().map_or("", written),
                        target: &target,
                    },
                );
            }
        }
    }
}

const STATEMENTS: &str = "import-statement";
const HEADS: &str = "import-head";
const ALIASES: &str = "import-alias-parts";
const CONTEXTUAL_ALIASES: &str = "import-alias-context";
const ENTRIES: &str = "import-module-context";
const LINE_COMMENTS: &str = "import-line-comment";

type ImportColumn = (&'static str, l::CellKind);

const fn extend_columns<const N: usize>(
    left: &[ImportColumn],
    right: &[ImportColumn],
) -> [ImportColumn; N] {
    assert!(left.len() + right.len() == N);
    let mut result = [("", l::CellKind::Number); N];
    let mut i = 0;
    while i < left.len() {
        result[i] = left[i];
        i += 1;
    }
    while i < N {
        result[i] = right[i - left.len()];
        i += 1;
    }
    result
}

const NEIGHBOR_COLUMNS: &[ImportColumn] = &[
    ("id", l::CellKind::Node),
    ("parent", l::CellKind::Node),
    ("field", l::CellKind::Symbol),
    ("kind", l::CellKind::Symbol),
    ("span", l::CellKind::Span),
    ("parent-kind", l::CellKind::Symbol),
    ("previous", l::CellKind::Node),
    ("previous-kind", l::CellKind::Symbol),
    ("previous-gap", l::CellKind::Span),
    ("has-previous", l::CellKind::Number),
    ("next", l::CellKind::Node),
    ("next-kind", l::CellKind::Symbol),
    ("has-next", l::CellKind::Number),
    ("gap", l::CellKind::Span),
    ("first", l::CellKind::Node),
    ("last", l::CellKind::Node),
    ("suite-span", l::CellKind::Span),
    ("file", l::CellKind::File),
];
const STATEMENT_COLUMNS: [ImportColumn; 24] = extend_columns(
    NEIGHBOR_COLUMNS,
    &[
        ("gap-text", l::CellKind::Symbol),
        ("previous-gap-text", l::CellKind::Symbol),
        ("statement-text", l::CellKind::Symbol),
        ("run-first", l::CellKind::Node),
        ("run-last", l::CellKind::Node),
        ("run-span", l::CellKind::Span),
    ],
);
const ALIAS_COLUMNS: &[ImportColumn] = &[
    ("alias", l::CellKind::Node),
    ("statement", l::CellKind::Node),
    ("index", l::CellKind::Number),
    ("count", l::CellKind::Number),
    ("renames", l::CellKind::Number),
    ("form", l::CellKind::Symbol),
    ("module", l::CellKind::Symbol),
    ("dots", l::CellKind::Symbol),
    ("name", l::CellKind::Symbol),
    ("rename", l::CellKind::Symbol),
    ("written-name", l::CellKind::Symbol),
    ("written-rename", l::CellKind::Symbol),
    ("last-segment", l::CellKind::Symbol),
    ("bound-name", l::CellKind::Symbol),
    ("target", l::CellKind::Symbol),
    ("written-module", l::CellKind::Symbol),
    ("root", l::CellKind::Symbol),
    ("alias-text", l::CellKind::Symbol),
    ("alias-span", l::CellKind::Span),
    ("statement-span", l::CellKind::Span),
    ("at", l::CellKind::Number),
    ("start-line", l::CellKind::Number),
    ("end-line", l::CellKind::Number),
    ("file", l::CellKind::File),
];
const HEADS_COLUMNS: [ImportColumn; 46] = extend_columns(
    &STATEMENT_COLUMNS,
    &[
        ("head-alias", l::CellKind::Node),
        ("head-at", l::CellKind::Number),
        ("count", l::CellKind::Number),
        ("renames", l::CellKind::Number),
        ("module", l::CellKind::Symbol),
        ("written-module", l::CellKind::Symbol),
        ("written-name", l::CellKind::Symbol),
        ("written-rename", l::CellKind::Symbol),
        ("dots", l::CellKind::Symbol),
        ("level", l::CellKind::Number),
        ("root", l::CellKind::Symbol),
        ("written-root", l::CellKind::Symbol),
        ("catalog", l::CellKind::Symbol),
        ("source-module-prefix", l::CellKind::Symbol),
        ("source-package-prefix", l::CellKind::Symbol),
        ("module-namespace", l::CellKind::Number),
        ("package-root", l::CellKind::Symbol),
        ("statement-at", l::CellKind::Number),
        ("statement-end", l::CellKind::Number),
        ("statement-start-line", l::CellKind::Number),
        ("statement-end-line", l::CellKind::Number),
        ("line-comment-count", l::CellKind::Number),
    ],
);
const ALIASES_COLUMNS: [ImportColumn; 28] = extend_columns(
    ALIAS_COLUMNS,
    &[
        ("statement-start-line", l::CellKind::Number),
        ("statement-end-line", l::CellKind::Number),
        ("statement-text", l::CellKind::Symbol),
        ("previous-start-line", l::CellKind::Number),
    ],
);
const CONTEXTUAL_ALIASES_COLUMNS: [ImportColumn; 36] = extend_columns(
    ALIAS_COLUMNS,
    &[
        ("written-root", l::CellKind::Symbol),
        ("level", l::CellKind::Number),
        ("catalog", l::CellKind::Symbol),
        ("module-prefix", l::CellKind::Symbol),
        ("package-prefix", l::CellKind::Symbol),
        ("module-namespace", l::CellKind::Number),
        ("package-root", l::CellKind::Symbol),
        ("statement-at", l::CellKind::Number),
        ("statement-end", l::CellKind::Number),
        ("statement-start-line", l::CellKind::Number),
        ("statement-end-line", l::CellKind::Number),
        ("statement-text", l::CellKind::Symbol),
    ],
);

impl ColumnFile<'_, '_> {
    /// Schemas whose rows follow from the import syntax and its project context.
    pub const IMPORT_SCHEMAS: &'static [l::FileRelation] = &[
        l::FileRelation {
            name: STATEMENTS,
            columns: &STATEMENT_COLUMNS,
        },
        l::FileRelation {
            name: HEADS,
            columns: &HEADS_COLUMNS,
        },
        l::FileRelation {
            name: ALIASES,
            columns: &ALIASES_COLUMNS,
        },
        l::FileRelation {
            name: CONTEXTUAL_ALIASES,
            columns: &CONTEXTUAL_ALIASES_COLUMNS,
        },
        l::FileRelation {
            name: ENTRIES,
            columns: &[
                ("alias", l::CellKind::Node),
                ("module", l::CellKind::Symbol),
                ("root", l::CellKind::Symbol),
                ("written-root", l::CellKind::Symbol),
                ("form", l::CellKind::Symbol),
                ("level", l::CellKind::Number),
                ("catalog", l::CellKind::Symbol),
                ("module-prefix", l::CellKind::Symbol),
                ("package-prefix", l::CellKind::Symbol),
                ("module-namespace", l::CellKind::Number),
                ("package-root", l::CellKind::Symbol),
                ("file", l::CellKind::File),
            ],
        },
        l::FileRelation {
            name: LINE_COMMENTS,
            columns: &[
                ("statement", l::CellKind::Node),
                ("comment", l::CellKind::Node),
                ("first-line", l::CellKind::Number),
                ("following", l::CellKind::Number),
                ("prefix", l::CellKind::Span),
                ("gap", l::CellKind::Span),
                ("text", l::CellKind::Symbol),
                ("file", l::CellKind::File),
            ],
        },
    ];

    /// Sorted lexical links, shared with the import head's comment count.
    pub fn import_comment_links<'a>(
        &self,
        links: &'a mut Option<Vec<(Position, Position, bool)>>,
    ) -> &'a [(Position, Position, bool)] {
        links.get_or_insert_with(|| {
            let mut rows = Vec::new();
            self.visit_import_line_comments(|statement, comment, first| {
                rows.push((statement, comment, first));
            });
            rows.sort_unstable_by_key(|row| (row.0.0, row.1.0));
            rows
        })
    }

    /// Write the import schema's columns using the file's local element positions.
    ///
    /// Project callbacks supply the enclosing package and observations of the
    /// imported module (written root, catalog, module prefix, package prefix,
    /// namespace). Syntax-only relations do not call either project callback.
    pub fn import_columns<W>(
        &self,
        schema: &l::FileRelation,
        src: &[u8],
        number: u32,
        comments: &mut Option<Vec<(Position, Position, bool)>>,
        context: &mut W,
        word: impl Fn(&mut W, &[u8]) -> u32,
        package: impl Fn(&mut W) -> Result<u64, String>,
        observe: impl Fn(&mut W, &Self, &ImportAlias<'_>, bool) -> (u64, u64, [u64; 5]),
    ) -> Result<(usize, Vec<Vec<u64>>), String> {
        if !Self::IMPORT_SCHEMAS.contains(schema) {
            return Err(format!("no import relation {}", schema.name));
        }
        let rel = schema.name;
        let estimated = match rel {
            HEADS | STATEMENTS => self.import_statement_count(),
            LINE_COMMENTS => self.import_comment_links(comments).len(),
            _ => 0,
        };
        let mut t = ImportRows {
            cols: (0..schema.columns.len())
                .map(|_| Vec::with_capacity(estimated))
                .collect(),
            n: 0,
        };
        let f = u64::from(number);
        let table = self.table;
        const NEIGHBOR_COLUMNS: usize = 18;
        const LAYOUT_COLUMNS: usize = NEIGHBOR_COLUMNS + 6;
        if matches!(rel, STATEMENTS | HEADS) {
            let kind = |context: &mut W, id: Position| {
                word(
                    context,
                    crate::domain::syntax::service::kind_name(
                        crate::domain::syntax::service::kind_of(node_table::kind_of(table, id)),
                    )
                    .0
                    .as_bytes(),
                )
            };
            let import = |id| self.is_import_statement(id);
            // Source order in each suite fixes both neighbors. A run contains consecutive import statements regardless of intervening trivia.
            self.visit_import_suites(|suite| {
                let first = suite[0];
                let last = suite[suite.len() - 1];
                let suite_span = span(bounds(table, first).0, bounds(table, last).1);
                let parent = node_table::parent_of(table, first);
                let field = crate::domain::labels::types::Field::from_repr(
                    node_table::field_of(table, first).0,
                )
                .unwrap_or_default();
                let field = word(
                    context,
                    crate::domain::labels::service::field_name(field)
                        .0
                        .as_bytes(),
                );
                let mut start = 0;
                while start < suite.len() {
                    let mut end = start + 1;
                    let starts_import = import(suite[start]);
                    if !starts_import {
                        start = end;
                        continue;
                    }
                    if starts_import {
                        while end < suite.len() && import(suite[end]) {
                            end += 1;
                        }
                    }
                    let head = suite[start];
                    let tail = suite[end - 1];
                    let run_span = span(bounds(table, head).0, bounds(table, tail).1);
                    for i in start..end {
                        let id = suite[i];
                        let (begin, finish) = bounds(table, id);
                        let previous = i.checked_sub(1).map(|p| suite[p]);
                        let next = suite.get(i + 1).copied();
                        push(
                            &mut t,
                            [
                                id.0.into(),
                                parent.0.into(),
                                field.into(),
                                kind(context, id).into(),
                                span(begin, finish),
                                kind(context, parent).into(),
                                previous.map_or(u64::MAX, |p| p.0.into()),
                                previous.map_or(u64::MAX, |p| kind(context, p).into()),
                                previous.map_or(u64::MAX, |p| span(bounds(table, p).1, begin)),
                                u64::from(previous.is_some()),
                                next.map_or(u64::MAX, |p| p.0.into()),
                                next.map_or(u64::MAX, |p| kind(context, p).into()),
                                u64::from(next.is_some()),
                                next.map_or(u64::MAX, |p| span(finish, bounds(table, p).0)),
                                first.0.into(),
                                last.0.into(),
                                suite_span,
                                f,
                            ],
                        );
                        {
                            let mut spelling = |lo: u32, hi: u32| {
                                u64::from(word(
                                    context,
                                    src.get(lo as usize..hi as usize).unwrap_or_default(),
                                ))
                            };
                            let gap =
                                next.map_or(u64::MAX, |p| spelling(finish, bounds(table, p).0));
                            let before =
                                previous.map_or(u64::MAX, |p| spelling(bounds(table, p).1, begin));
                            let text = spelling(begin, finish);
                            for (c, v) in t.cols[NEIGHBOR_COLUMNS..LAYOUT_COLUMNS]
                                .iter_mut()
                                .rev()
                                .zip(
                                    [gap, before, text, head.0.into(), tail.0.into(), run_span]
                                        .into_iter()
                                        .rev(),
                                )
                            {
                                c.push(v);
                            }
                        }
                    }
                    start = end;
                }
            });
            if rel == HEADS && t.n != 0 {
                let (layout, extras) = t.cols.split_at_mut(LAYOUT_COLUMNS);
                let (counts, alias_columns) = extras
                    .split_last_mut()
                    .expect("import head includes comment count");
                let upto = layout[4]
                    .iter()
                    .map(|s| (*s as u32) as usize)
                    .max()
                    .unwrap_or(0)
                    .min(src.len());
                // Only offsets up to the final import are queried. Read one further byte for a CRLF at the boundary.
                let starts = super::super::comment_positions::starts(
                    &src[..upto.saturating_add(1).min(src.len())],
                );
                let links = self.import_comment_links(comments);
                let containing_package = package(context)?;
                self.visit_ordered_import_heads(
                    layout[0].iter().map(|id| Position(*id as u32)),
                    |row, a| {
                        let (begin, end) = bounds(table, Position(a.statement));
                        let (module, root_id, observed) = observe(context, self, &a, true);
                        // The parser's spelling and the source spelling usually share one symbol.
                        let empty = u64::from(word(context, b""));
                        let mut sym = |s: &str| {
                            if s.is_empty() {
                                empty
                            } else {
                                u64::from(word(context, s.as_bytes()))
                            }
                        };
                        let written_module = if a.written_module == a.module {
                            module
                        } else {
                            sym(a.written_module)
                        };
                        let written_name = if a.written_name == a.written_module {
                            written_module
                        } else {
                            sym(a.written_name)
                        };
                        let values = [
                            a.id.into(),
                            bounds(table, Position(a.id)).0.into(),
                            a.count as u64,
                            a.renames as u64,
                            module,
                            written_module,
                            written_name,
                            sym(a.written_rename),
                            sym(a.dots),
                            a.dots.len() as u64,
                            root_id,
                            observed[0],
                            observed[1],
                            observed[2],
                            observed[3],
                            observed[4],
                            containing_package,
                            begin.into(),
                            end.into(),
                            starts.partition_point(|s| *s <= begin) as u64,
                            starts.partition_point(|s| *s <= end) as u64,
                        ];
                        // Parser recovery can omit the first alias. Only its skipped row needs padding.
                        if alias_columns[0].len() != row {
                            for c in alias_columns.iter_mut() {
                                c.resize(row, u64::MAX);
                            }
                        }
                        for (column, v) in alias_columns.iter_mut().zip(values) {
                            column.push(v);
                        }
                    },
                );
                for col in alias_columns {
                    col.resize(t.n, u64::MAX);
                }

                if links.is_empty() {
                    counts.resize(t.n, 0);
                } else {
                    for &id in &layout[0] {
                        let lo = links.partition_point(|link| u64::from(link.0.0) < id);
                        let hi = links[lo..].partition_point(|link| u64::from(link.0.0) == id);
                        counts.push(hi as u64);
                    }
                }
            }
        } else if rel == LINE_COMMENTS {
            // A comment beginning on a line touched by the import has a source prefix and, when it follows the statement, an intervening gap.
            for &(statement, comment, first_line) in self.import_comment_links(comments) {
                let (start, end) = bounds(table, statement);
                let (at, until) = bounds(table, comment);
                let following = end <= at;
                let text = word(context, &src[at as usize + 1..until as usize]);
                push(
                    &mut t,
                    [
                        statement.0.into(),
                        comment.0.into(),
                        first_line.into(),
                        following.into(),
                        span(start, at),
                        if following { span(end, at) } else { u64::MAX },
                        text.into(),
                        f,
                    ],
                );
            }
        } else if rel == ENTRIES {
            let package_root = package(context)?;
            self.visit_import_aliases(|a| {
                let root = a.module.split('.').next().unwrap_or_default();
                let root_id = u64::from(word(context, root.as_bytes()));
                let (_, _, observed) = observe(context, self, &a, false);
                push(
                    &mut t,
                    [
                        a.id.into(),
                        word(context, a.module.as_bytes()).into(),
                        root_id,
                        observed[0],
                        word(context, a.form.as_bytes()).into(),
                        a.dots.len() as u64,
                        observed[1],
                        observed[2],
                        observed[3],
                        observed[4],
                        package_root,
                        f,
                    ],
                );
            });
        } else {
            let contextual = rel == CONTEXTUAL_ALIASES;
            let containing_package = if contextual && self.import_statement_count() != 0 {
                package(context)?
            } else {
                0
            };
            let mut statement_text = 0;
            let starts = super::super::comment_positions::starts(src);
            let mut previous_statement = None;
            let mut statement_lines = [0; 2];
            let line = |byte| starts.partition_point(|&s| s <= byte) as u64;
            // An alias has a parser name and an optional as-name, each with its own original source spelling.
            self.visit_import_aliases(|a| {
                let (at, end) = bounds(table, Position(a.id));
                let (st, en) = bounds(table, Position(a.statement));
                let mut sym = |s: &str| u64::from(word(context, s.as_bytes()));
                let root = a.module.split('.').next().unwrap_or_default();
                let root_id = sym(root);
                let alias_text =
                    std::str::from_utf8(src.get(at as usize..end as usize).unwrap_or_default())
                        .unwrap_or_default();
                push(
                    &mut t,
                    [
                        a.id.into(),
                        a.statement.into(),
                        a.index as u64,
                        a.count as u64,
                        a.renames as u64,
                        sym(a.form),
                        sym(a.module),
                        sym(a.dots),
                        sym(a.name),
                        sym(a.rename),
                        sym(a.written_name),
                        sym(a.written_rename),
                        sym(a.name.rsplit('.').next().unwrap_or_default()),
                        sym(if !a.rename.is_empty() {
                            a.rename
                        } else if a.form == "import" {
                            root
                        } else if a.name == "*" {
                            ""
                        } else {
                            a.name
                        }),
                        sym(a.target),
                        sym(a.written_module),
                        root_id,
                        sym(alias_text),
                        span(at, end),
                        span(st, en),
                        at.into(),
                        line(at),
                        line(end),
                        f,
                    ],
                );
                // Every alias shares its enclosing statement's extent; its preceding name is fixed by the ordered alias list.
                if previous_statement != Some(a.statement) {
                    previous_statement = Some(a.statement);
                    statement_text = u64::from(word(context, &src[st as usize..en as usize]));
                    statement_lines = [line(st), line(en)];
                }
                if contextual {
                    let (_, _, observed) = observe(context, self, &a, false);
                    let extra = [
                        observed[0],
                        a.dots.len() as u64,
                        observed[1],
                        observed[2],
                        observed[3],
                        observed[4],
                        containing_package,
                        st.into(),
                        en.into(),
                        statement_lines[0],
                        statement_lines[1],
                        statement_text,
                    ];
                    for (col, v) in t.cols.iter_mut().rev().zip(extra.into_iter().rev()) {
                        col.push(v);
                    }
                } else {
                    let extra = [
                        statement_lines[0],
                        statement_lines[1],
                        statement_text,
                        a.previous_start.map_or(u64::MAX, line),
                    ];
                    for (col, v) in t.cols.iter_mut().rev().zip(extra.into_iter().rev()) {
                        col.push(v);
                    }
                }
            });
        }
        Ok((t.n, t.cols))
    }

    /// Project the observed context of an import using the same dictionary as its syntax.
    pub fn import_context<W>(
        &self,
        alias: &ImportAlias<'_>,
        target: &str,
        context: &mut W,
        word: impl Fn(&mut W, &[u8]) -> u32,
        prefixes: impl Fn(&mut W, &str) -> (u64, u64, bool),
    ) -> [u64; 5] {
        let empty = u64::from(word(context, b""));
        let (module, package, namespace) = if alias.dots.is_empty() && !target.is_empty() {
            let (module, package, namespace) = prefixes(context, target);
            let namespace = if target == alias.module {
                namespace
            } else if !alias.module.is_empty()
                && target
                    .strip_prefix(alias.module)
                    .is_some_and(|suffix| suffix.starts_with('.'))
            {
                prefixes(context, alias.module).2
            } else {
                false
            };
            (module, package, namespace)
        } else {
            (empty, empty, false)
        };
        let root = alias.module.split('.').next().unwrap_or_default();
        let written = alias.written_module.split('.').next().unwrap_or_default();
        let catalog = self.import_catalog(root);
        [
            word(context, written.as_bytes()).into(),
            word(context, catalog.as_bytes()).into(),
            module,
            package,
            namespace.into(),
        ]
    }

    pub(super) fn named_import_requires_tokens(schema: &l::FileRelation) -> bool {
        matches!(schema.name, HEADS | LINE_COMMENTS)
    }

    /// Complete file-local import facts in the same dictionary as the syntax.
    pub(super) fn named_import(
        &self,
        schema: &l::FileRelation,
        file: l::FileId,
        src: &[u8],
        symbols: &mut l::Symbols,
        comments: &mut Option<Vec<(Position, Position, bool)>>,
        package_root: Option<&str>,
        prefixes: &mut Option<&mut dyn FnMut(&str) -> (String, String, bool)>,
    ) -> Result<Vec<l::FactColumn>, String> {
        let intern = |symbols: &mut l::Symbols, text: &str| {
            l::intern_derived(symbols, l::ByteSlice(src), l::NameText(text)).0
        };
        let (_, columns) = self.import_columns(
            schema,
            src,
            file.0,
            comments,
            &mut (symbols, prefixes),
            |context, bytes| {
                intern(
                    context.0,
                    std::str::from_utf8(bytes).expect("Python source or name"),
                )
            },
            |context| {
                let root = package_root.ok_or_else(|| {
                    format!("{} needs the file's project import context", schema.name)
                })?;
                Ok(u64::from(intern(context.0, root)))
            },
            |context, file, a, head| {
                let target = if head { a.module } else { a.target };
                let root = a.module.split('.').next().unwrap_or_default();
                let observations = file.import_context(
                    a,
                    target,
                    context,
                    |context, bytes| {
                        intern(
                            context.0,
                            std::str::from_utf8(bytes).expect("Python import vocabulary"),
                        )
                    },
                    |context, target| {
                        let prefixes = context.1.as_deref_mut();
                        if let Some(prefixes) = prefixes {
                            let (module, package, namespace) = prefixes(target);
                            (
                                intern(context.0, &module).into(),
                                intern(context.0, &package).into(),
                                namespace,
                            )
                        } else {
                            let empty = u64::from(intern(context.0, ""));
                            (empty, empty, false)
                        }
                    },
                );
                (
                    intern(context.0, a.module).into(),
                    intern(context.0, root).into(),
                    observations,
                )
            },
        )?;
        Ok(super::qualify_cells(schema, file, columns))
    }
}

struct ImportRows {
    cols: Vec<Vec<u64>>,
    n: usize,
}

fn push<const N: usize>(t: &mut ImportRows, row: [u64; N]) {
    for (c, v) in t.cols.iter_mut().zip(row) {
        c.push(v);
    }
    t.n += 1;
}

fn bounds(table: &l::NodeTable, id: Position) -> (u32, u32) {
    let span = node_table::span_of(table, id);
    (span.start.0, span.end.0)
}

fn span(start: u32, end: u32) -> u64 {
    (u64::from(start) << 32) | u64::from(end)
}
