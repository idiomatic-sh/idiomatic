//! Source coordinates of statements within a Python suite, and parsed components of import aliases.
use super::{ColumnFile, Kind, Node, Numbered, nodes_of_kind};
use crate::domain::node::service::id_of;
use ruff_python_ast::AnyNodeRef;
use ruff_text_size::Ranged;
use ryo::languages::{Position, node_table};
use std::collections::{HashMap, HashSet};

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
        use crate::domain::syntax::service::kind_bits;
        [Node::Import, Node::ImportFrom]
            .into_iter()
            .map(|k| node_table::ids_of_kind(self.table, kind_bits(Kind::Node(k))).len())
            .sum()
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

    /// An import-line association requires both an import statement and a comment token.
    pub fn can_have_import_line_comments(&self) -> bool {
        use crate::domain::{syntax::service::kind_bits, tokens::types::Token};
        !node_table::ids_of_kind(self.table, kind_bits(Kind::Token(Token::Comment))).is_empty()
            && self.import_statement_count() != 0
    }

    /// A comment occupies a physical line. Report every import whose source extent reaches that line before the comment.
    pub fn visit_import_line_comments(&self, mut visit: impl FnMut(Position, Position, bool)) {
        use crate::domain::{syntax::service::kind_bits, tokens::types::Token};
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

    /// Each statement occupies one ordered position in the suite denoted by its parent and field.
    pub fn visit_statement_suites(
        &self,
        containing_import: bool,
        mut visit: impl FnMut(&[Position]),
    ) {
        if containing_import {
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
            return;
        }
        let mut suites = HashMap::<(u32, u16), Vec<Position>>::new();
        for (i, node) in self.file.syntax.nodes.iter().enumerate() {
            if node.is_statement() {
                let id = Position(i as u32);
                let parent = node_table::parent_of(self.table, id);
                let field = node_table::field_of(self.table, id);
                suites.entry((parent.0, field.0)).or_default().push(id);
            }
        }
        for suite in suites.values() {
            visit(suite);
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
