//! The `fact_notes` service: free functions named by verb over the domain's types, with no IO and no ports. It notes the imports a
//! file writes when the parse reads it, for the project to read; the port `notes` writes them as bytes and reads them back.

use ryo::languages::{self, Position};


use super::types::{DottedPath, Form, ImportNote, Level, Note, Spelling};










fn source_spelling(
    source: languages::ByteSlice<'_>,
    start: languages::Offset,
    end: languages::Offset,
) -> Spelling {
    let text = source
        .0
        .get(
            usize::try_from(start.0).unwrap_or(usize::MAX)
                ..usize::try_from(end.0).unwrap_or(usize::MAX),
        )
        .map(String::from_utf8_lossy)
        .unwrap_or_default();
    Spelling(text.split_whitespace().collect::<String>().into())
}

/// Read the same project notes from the parser's lent numbering.
pub(crate) fn noted_walk(
    file: &crate::inbound::contracts::walk::named::NamedFile<'_, '_>,
    source: languages::ByteSlice<'_>,
) -> Note {
    let mut imports = Vec::new();
    file.visit_import_aliases(|alias| {
        let span = languages::node_table::span_of(file.table, Position(alias.id));
        let (form, path) = if alias.form == "import" {
            (Form::Module, alias.written_name.to_owned())
        } else {
            let module = alias.written_module.trim_start_matches('.');
            if alias.written_name == "*" {
                (Form::Everything, module.to_owned())
            } else if module.is_empty() {
                (Form::Name, alias.written_name.to_owned())
            } else {
                (Form::Name, format!("{module}.{}", alias.written_name))
            }
        };
        imports.push(ImportNote {
            form,
            level: Level(alias.dots.len().try_into().expect("relative import level")),
            path: DottedPath(path.into()),
            start: span.start,
            end: span.end,
            text: source_spelling(source, span.start, span.end),
        });
    });
    imports.sort_by_key(|note| note.start.0);
    Note { imports }
}
