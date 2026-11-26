//! The Python provider's additional file-scope schemas and batched answers.
use super::{NamedFile, qualify_cells};
use ryo::languages::{self as l, CellKind, FileRelation};
use std::sync::OnceLock;

type Covered = Result<l::FileRows, l::ParseError>;
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

impl NamedFile<'_, '_> {
    /// File-scope schemas for the named Python tables.
    pub(crate) fn named_schemas() -> &'static [FileRelation] {
        static SCHEMAS: OnceLock<Vec<FileRelation>> = OnceLock::new();
        SCHEMAS.get_or_init(|| {
            let mut schemas = Vec::new();
            schemas.extend_from_slice(Self::EXPRESSION_SCHEMAS);
            schemas.extend_from_slice(Self::IMPORT_SCHEMAS);
            schemas.extend_from_slice(Self::TYPING_SCHEMAS);
            schemas
        })
    }

    /// Report an uncovered source extent for a declared relation before publishing rows.
    pub(crate) fn named_uncovered(
        schema: &FileRelation,
        syntax: l::SyntaxCompleteness,
        lexical: l::SyntaxCompleteness,
    ) -> Option<l::FileRows> {
        if syntax == l::SyntaxCompleteness::Complete && lexical == l::SyntaxCompleteness::Complete {
            return None;
        }
        let extent = if lexical == l::SyntaxCompleteness::Incomplete && Self::named_import_requires_tokens(schema) {
            "lexical stream"
        } else if syntax == l::SyntaxCompleteness::Incomplete && Self::named_schemas().contains(schema) {
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
    /// The optional package root is the enclosing regular package's root name. An
    /// absent root means the project was not observed. The prefix callback reports
    /// module, package and namespace observations under the host's declared search roots.
    pub(crate) fn answer_named_facts_covered(
        &mut self,
        schemas: &[&FileRelation],
        file: l::FileId,
        bytes: l::ByteSlice<'_>,
        symbols: &mut l::Symbols,
        package_root: Option<&str>,
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
                answers.push(Ok(self.named_expression(schema, file, bytes, symbols)));
                continue;
            }
            if Self::TYPING_SCHEMAS.contains(schema) {
                semantic_order.push(slot);
                semantic.push(*schema);
                answers.push(Err(l::ParseError::Unsupported));
                continue;
            }
            let answer = if Self::IMPORT_SCHEMAS.contains(schema) {
                self.named_import(
                    schema,
                    file,
                    bytes.0,
                    symbols,
                    &mut comments,
                    package_root,
                    &mut prefixes,
                )
                .map_err(failed)
            } else {
                Err(l::ParseError::Unsupported)
            };
            answers.push(answer.map(complete));
        }

        if !semantic.is_empty() {
            let mut slots = semantic_order.into_iter();
            self.visit_typing_relations(&semantic, file, |schema, result, words| {
                let slot = slots.next().expect("requested semantic relation");
                answers[slot] = result
                    .map(|mut columns| {
                        for (cells, (_, kind)) in columns.iter_mut().zip(schema.columns) {
                            if *kind == CellKind::Symbol {
                                for cell in cells {
                                    let word = l::facts::text(words, l::CellWord(*cell));
                                    *cell = u64::from(
                                        l::intern_derived(symbols, bytes, l::NameText(word.0)).0,
                                    );
                                }
                            }
                        }
                        complete(qualify_cells(schema, file, columns))
                    })
                    .map_err(failed);
            });
        }

        answers
    }
}

impl<'a, 'f> NamedFile<'a, 'f> {
    /// Answers the named batch of a walked file while its view is live: the language's private relations, which the
    /// walk's `named` item hands to here.
    pub(crate) fn named(
        file: &'a mut crate::domain::file_view::types::PythonFile<'f>,
        request: l::NamedRequest<'_, '_>,
    ) -> Vec<Result<l::FileRows, l::ParseError>> {
        let l::NamedRequest { table, relations: projected, request, project, words, symbols } = request;
        let import_presence = file.import_presence;
        let mut file = NamedFile { table, file, import_presence, words };
        let uses_package = |s: &l::FileRelation| s.name == "import-head" && file.import_statement_count() != 0;
        let needs_package: Vec<_> = projected.iter().map(|s| uses_package(s)).collect();
        let package = project
            .filter(|_| needs_package.iter().any(|&needed| needed))
            .map(|project| crate::inbound::contracts::walk::named::import_environment::package(project, request.path.0))
            .transpose();
        let observed_package = package.as_ref().ok().and_then(Option::as_ref).map(|(_, root)| root.as_str());
        let mut failure = None;
        let mut prefixes = |target: &str| match crate::inbound::contracts::walk::named::import_environment::prefixes(project.expect("declared roots"), target) {
            Ok(prefixes) => prefixes,
            Err(error) => {
                failure = Some(error);
                Default::default()
            }
        };
        let lookup = project
            .filter(|p| p.roots().is_some())
            .map(|_| &mut prefixes as &mut dyn FnMut(&str) -> (String, String, bool));
        let mut answers =
            file.answer_named_facts_covered(projected, request.file, request.bytes, symbols, observed_package, lookup);
        for ((schema, answer), needed) in projected.iter().zip(&mut answers).zip(needs_package) {
            if needed && let Err(error) = &package {
                *answer = Err(failed(error.as_str().into()));
                continue;
            }
            if let Some(error) = failure.as_ref().filter(|_| schema.name == "import-head") {
                *answer = Err(failed(error.as_str().into()));
            }
        }
        answers
    }
}
