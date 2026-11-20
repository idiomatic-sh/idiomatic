//! The `lexical_facts` service: the fillers of the relations of scopes, definitions and names. Track P1's placeholder of the file track P2 writes: each relation asks for the model and is not read yet.

use crate::domain::model::view::types::{ModelPart, ModelRead, ModelView};
use ryo::languages::{self, AbsenceCause, CatalogRelation, FileWords, Fill, Rows};

/// Fills `scope`: not read yet.
pub(crate) fn scope_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::Scope as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Lexical);
    Fill::Absent(AbsenceCause::NotYetRead)
}

/// Fills `defines`: not read yet.
pub(crate) fn defines_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::Defines as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Lexical);
    Fill::Absent(AbsenceCause::NotYetRead)
}

/// Fills `reference`: not read yet.
pub(crate) fn reference_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::Reference as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Lexical);
    Fill::Absent(AbsenceCause::NotYetRead)
}

/// Fills `resolves`: not read yet.
pub(crate) fn resolves_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::Resolves as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Lexical);
    Fill::Absent(AbsenceCause::NotYetRead)
}

/// Fills `qualified-name`: not read yet.
pub(crate) fn qualified_name_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::QualifiedName as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Lexical);
    Fill::Absent(AbsenceCause::NotYetRead)
}

/// Fills `implicit-symbol`: not read yet.
pub(crate) fn implicit_symbol_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::ImplicitSymbol as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Lexical);
    Fill::Absent(AbsenceCause::NotYetRead)
}
