//! The Python provider's additional file-scope schemas and batched answers.
use super::{ColumnFile, qualify_cells};
use ryo::languages::{self as l, CellKind, FileRelation};
use std::sync::OnceLock;

const RELATIVE_IMPORT: FileRelation = FileRelation {
    name: "relative-import-target",
    columns: &[
        ("occurrence", CellKind::Node),
        ("path", CellKind::Symbol),
        ("file", CellKind::File),
    ],
};

type Covered = Result<l::FileRows, l::ParseError>;
type TypingInputs<'a> = (
    &'a dyn Fn(&[u8]) -> bool,
    &'a dyn Fn(&[u8]) -> bool,
    &'a [Vec<u8>],
);
type ImportPrefixes<'a> = &'a mut dyn FnMut(&str) -> (String, String, bool);

fn failed(reason: String) -> l::ParseError {
    l::ParseError::Library {
        reason: l::ToolMessage(reason.into()),
    }
}

fn complete(columns: Vec<l::FactColumn>) -> l::FileRows {
    l::FileRows {
        columns,
        partial: None,
    }
}

impl ColumnFile<'_, '_> {
    /// File-scope schemas for the named Python tables.
    pub fn named_schemas() -> &'static [FileRelation] {
        static SCHEMAS: OnceLock<Vec<FileRelation>> = OnceLock::new();
        SCHEMAS.get_or_init(|| {
            let mut schemas = Vec::new();
            schemas.extend_from_slice(crate::fixtures::comment_positions::SCHEMAS);
            schemas.extend_from_slice(Self::EXPRESSION_SCHEMAS);
            schemas.extend_from_slice(Self::IMPORT_SCHEMAS);
            schemas.extend_from_slice(Self::SITE_SCHEMAS);
            schemas.extend_from_slice(Self::TYPING_SCHEMAS);
            schemas.push(RELATIVE_IMPORT);
            schemas
        })
    }

    /// Report an uncovered source extent for a declared relation before publishing rows.
    pub fn named_uncovered(
        schema: &FileRelation,
        syntax: l::SyntaxCompleteness,
        lexical: l::SyntaxCompleteness,
    ) -> Option<l::FileRows> {
        if syntax == l::SyntaxCompleteness::Complete && lexical == l::SyntaxCompleteness::Complete {
            return None;
        }
        let lexical_only = crate::fixtures::comment_positions::SCHEMAS.contains(schema)
            || Self::named_expression_is_lexical(schema);
        let extent = if lexical == l::SyntaxCompleteness::Incomplete
            && (lexical_only
                || Self::named_expression_requires_tokens(schema)
                || Self::named_import_requires_tokens(schema))
        {
            "lexical stream"
        } else if syntax == l::SyntaxCompleteness::Incomplete
            && !lexical_only
            && Self::named_schemas().contains(schema)
        {
            "syntax"
        } else {
            return None;
        };
        Some(l::FileRows {
            columns: vec![l::FactColumn::default(); schema.columns.len()],
            partial: Some(l::ToolMessage(
                format!("incomplete {extent} for {}", schema.name).into(),
            )),
        })
    }

    /// Answer named tables from the numbered Python file. The result follows request order.
    ///
    /// The optional package pair is the enclosing dotted package, if any, and the
    /// enclosing regular package's root name. An absent pair means the project was
    /// not observed. The prefix callback reports module, package and namespace
    /// observations under the host's declared search roots.
    pub fn answer_named_facts_covered(
        &mut self,
        schemas: &[&FileRelation],
        file: l::FileId,
        path: &str,
        bytes: l::ByteSlice<'_>,
        symbols: &mut l::Symbols,
        typing: Option<TypingInputs<'_>>,
        package: Option<(Option<&str>, &str)>,
        mut prefixes: Option<ImportPrefixes<'_>>,
    ) -> Vec<Covered> {
        let mut comments = None;
        let mut answers = Vec::with_capacity(schemas.len());
        let mut semantic = Vec::new();
        let mut semantic_order = Vec::new();

        let coverage = self.file.syntax.completeness;
        for (slot, &schema) in schemas.iter().enumerate() {
            if let Some(partial) = Self::named_uncovered(schema, coverage.syntax, coverage.lexical)
            {
                answers.push(Ok(partial));
                continue;
            }
            if Self::EXPRESSION_SCHEMAS.contains(schema) {
                answers.push(
                    self.named_expression(schema, file, bytes.0, symbols)
                        .map_err(failed),
                );
                continue;
            }
            if Self::TYPING_SCHEMAS.contains(schema) {
                semantic_order.push(slot);
                semantic.push(*schema);
                answers.push(Err(l::ParseError::Unsupported));
                continue;
            }
            let answer = if crate::fixtures::comment_positions::SCHEMAS.contains(schema) {
                crate::fixtures::comment_positions::columns(
                    schema, file, self.table, bytes, symbols,
                )
            } else if Self::IMPORT_SCHEMAS.contains(schema) {
                self.named_import(
                    schema,
                    file,
                    bytes.0,
                    symbols,
                    &mut comments,
                    package.map(|p| p.1),
                    &mut prefixes,
                )
                .map_err(failed)
            } else if Self::SITE_SCHEMAS.contains(schema) {
                self.named_site(schema, file, bytes.0, symbols)
                    .map_err(failed)
            } else if *schema == RELATIVE_IMPORT {
                self.named_relative_imports(file, bytes, symbols, package)
                    .map_err(failed)
            } else {
                Err(l::ParseError::Unsupported)
            };
            answers.push(answer.map(complete));
        }

        if !semantic.is_empty() {
            let default = |_: &[u8]| false;
            let (generic, argument_forms, modules) = typing.unwrap_or((&default, &default, &[]));
            let mut slots = semantic_order.into_iter();
            self.visit_typing_relations(
                &semantic,
                file,
                path.ends_with(".pyi"),
                generic,
                argument_forms,
                modules,
                |schema, result, words| {
                    let slot = slots.next().expect("requested semantic relation");
                    answers[slot] = result
                        .map(|mut columns| {
                            for (cells, (_, kind)) in columns.iter_mut().zip(schema.columns) {
                                if *kind == CellKind::Symbol {
                                    for cell in cells {
                                        let word = l::facts::text(words, l::CellWord(*cell));
                                        *cell = u64::from(
                                            l::intern_derived(symbols, bytes, l::NameText(word.0))
                                                .0,
                                        );
                                    }
                                }
                            }
                            complete(qualify_cells(schema, file, columns))
                        })
                        .map_err(failed);
                },
            );
        }

        answers
    }

    fn named_relative_imports(
        &self,
        file: l::FileId,
        bytes: l::ByteSlice<'_>,
        symbols: &mut l::Symbols,
        package: Option<(Option<&str>, &str)>,
    ) -> Result<Vec<l::FactColumn>, String> {
        let mut columns = vec![Vec::new(); RELATIVE_IMPORT.columns.len()];
        if self.has_relative_imports() {
            let package =
                package.ok_or("relative-import-target needs the containing package observation")?;
            self.visit_relative_import_targets(package.0, |id, path| {
                columns[0].push(id.into());
                columns[1].push(
                    l::intern_derived(symbols, bytes, l::NameText(path))
                        .0
                        .into(),
                );
                columns[2].push(file.0.into());
            });
        }
        Ok(qualify_cells(&RELATIVE_IMPORT, file, columns))
    }
}
