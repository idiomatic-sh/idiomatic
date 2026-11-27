//! Source coordinates of statements within a Python suite, and parsed components of import aliases.
use super::{NamedFile, Kind, Node, Numbered, nodes_of_kind};
use crate::domain::node::service::id_of;
use ruff_python_ast::AnyNodeRef;
use ruff_text_size::Ranged;



/// The parser's import names and their original spelling at one alias.
#[derive(Debug)]
pub(crate) struct ImportAlias<'a> {
    /// Alias occurrence.
    pub(crate) id: u32,
    /// Import or from-import syntax.
    pub(crate) form: &'static str,
    /// Leading relative levels.
    pub(crate) dots: &'a str,
    /// Module identifier as written, with the requested relative levels.
    pub(crate) written_module: &'a str,
    /// Source spelling of the imported identifier.
    pub(crate) written_name: &'a str,
}

impl NamedFile<'_, '_> {

    fn import_statements(&self) -> impl Iterator<Item = Numbered<'_>> {
        [Node::Import, Node::ImportFrom]
            .into_iter()
            .flat_map(move |kind| nodes_of_kind(&self.file.syntax, Kind::Node(kind)))
    }

    /// An import requests a dotted path; the from-clause's leading dots select its relative level and the alias's as-name gives its binding spelling.
    pub(crate) fn visit_import_aliases(&self, mut visit: impl FnMut(ImportAlias<'_>)) {
        self.visit_import_names(self.import_statements().enumerate(), false, |_, alias| {
            visit(alias)
        });
    }

    fn visit_import_names<'n>(
        &self,
        statements: impl Iterator<Item = (usize, Numbered<'n>)>,
        first_only: bool,
        mut visit: impl FnMut(usize, ImportAlias<'_>),
    ) {
        let text = self.file.syntax.text.0;
        let mut source_module = String::new();
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
            let names = if first_only {
                &aliases[..aliases.len().min(1)]
            } else {
                &aliases[..]
            };
            for alias in names.iter() {
                let Some(id) = id_of(alias) else { continue };
                let written = |ident: &ruff_python_ast::Identifier| {
                    text.get(usize::from(ident.start())..usize::from(ident.end()))
                        .unwrap_or_default()
                };
                visit(
                    row,
                    ImportAlias {
                        id: id.0,
                        form,
                        dots: &source_module[..level as usize],
                        written_module: if form == "import" {
                            written(&alias.name)
                        } else {
                            &written_module
                        },
                        written_name: written(&alias.name),
                    },
                );
            }
        }
    }
}

