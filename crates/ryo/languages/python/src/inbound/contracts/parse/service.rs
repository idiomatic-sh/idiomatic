//! The parse contract's own operations: its construction, the kind of a source and the preparation of a file's model.
use super::constants::STUB_EXTENSION;
use crate::domain::fact_notes::service::noted;
use crate::domain::fields::service::role_of;
use crate::domain::labels::service::{field_bits, field_name, fields};
use crate::domain::model::types::Declarations;
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::outbound::notes::interface::NotesPort;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::PythonSourceKind;
use crate::outbound::syntax::types::SourceText;
use crate::settings::PythonParserSettings;
use ryo::languages;

/// Select stubs by the exact lowercase `.pyi` suffix used by the existing parser contract.
pub(super) fn source_kind(path: languages::SourcePath<'_>) -> PythonSourceKind {
    if std::path::Path::new(path.0)
        .extension()
        .is_some_and(|extension| extension == std::ffi::OsStr::new(STUB_EXTENSION))
    {
        PythonSourceKind::Stub
    } else {
        PythonSourceKind::Python
    }
}

/// Prepares the model of one file: the language target and every cache cleared, the declarations and the binding index among them.
pub(super) fn prepare_model(settings: &PythonParserSettings, model: &mut Declarations) {
    model.language_target = settings.target_version().components();
    crate::domain::expression_shapes::service::clear(model.expression_shapes.get_mut());
    crate::domain::constant_keys::service::clear(model.constant_keys.get_mut());
    crate::domain::format_parsing::service::clear(model.format_parsing.get_mut());
    crate::domain::raw_format::service::clear(model.raw_format.get_mut());
    crate::domain::literal_expression_parsing::service::clear(
        model.literal_expression_parsing.get_mut(),
    );
    crate::domain::brace_format::service::clear(model.brace_format.get_mut());
    crate::domain::expression_types::service::clear(model.expression_types.get_mut());
    crate::domain::terminal_flow::service::clear(model.terminal_flow.get_mut());
    crate::domain::indentation::service::clear(model.indentation.get_mut());
    crate::domain::qualification::service::clear(model.qualification.get_mut());
    crate::domain::binding_analysis::service::clear(model.binding_analysis.get_mut());
    model.semantic.get_mut().hidden.clear();
    model.semantic.get_mut().state = crate::domain::model::types::QualificationState::Fresh;
    // The declarations and the binding index are built on a reader's first lookup, not for every file.
    model.declared.get_mut().clear();
    model.bindings.get_mut().clear();
    model
        .declared_state
        .set(crate::domain::model::types::QualificationState::Fresh);
    model
        .bindings_state
        .set(crate::domain::model::types::QualificationState::Fresh);
}

/// Completes a file: its kind names, and, when asked, its field names, roles and notes.
pub(super) fn complete_file(
    syntax: &dyn SyntaxPort,
    notes: &dyn NotesPort,
    request: &languages::ParseRequest<'_>,
    completion: &mut languages::Completion<'_>,
) {
    crate::application::completion::service::expand(
        SourceText(request.bytes.0),
        completion,
        syntax,
    );
    for kind in kinds() {
        languages::name_kind(completion, kind_bits(kind), kind_name(kind));
    }
    if request.fields == languages::FieldRequest::Wanted {
        for field in fields() {
            languages::name_field(completion, field_bits(field), field_name(field));
        }
        languages::write_fields(completion, role_of);
    }
    if request.notes == languages::NoteRequest::Wanted {
        languages::write_notes(completion, |tree, out| {
            notes.write(&noted(tree), out);
        });
    }
}

/// Begins the semantic session of one file in `slot`, when the syntax adapter has one for its kind of source.
pub(super) fn begin_file(
    syntax: &dyn SyntaxPort,
    request: &languages::ParseRequest<'_>,
    tree: &languages::Elements,
    slot: &mut languages::SemanticSlot,
) {
    if let Some(session) =
        syntax.begin_semantics(tree, source_kind(request.path), std::mem::take(slot))
    {
        *slot = languages::SemanticSlot::Begun(session);
    }
}

/// Reads the tree of one file into the worker's table.
pub(super) fn read_file(
    syntax: &dyn SyntaxPort,
    request: &languages::ParseRequest<'_>,
    builder: &mut languages::TableBuilder<'_>,
) {
    syntax.read(
        SourceText(request.bytes.0),
        source_kind(request.path),
        builder,
    );
}
