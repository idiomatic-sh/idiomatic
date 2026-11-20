//! The `binding_facts` service: the fillers of the relations of binding events and reads. Track P1's placeholder of the file track P2 writes: each relation asks for the model and is not read yet.

use crate::domain::model::view::types::{ModelPart, ModelRead, ModelView};
use ryo::languages::{self, AbsenceCause, CatalogRelation, FileWords, Fill, Rows};

/// Fills `binding-event`: not read yet.
pub(crate) fn binding_event_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::BindingEvent as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Flow);
    Fill::Absent(AbsenceCause::NotYetRead)
}

/// Fills `binding-read`: not read yet.
pub(crate) fn binding_read_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::BindingRead as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Flow);
    Fill::Absent(AbsenceCause::NotYetRead)
}

/// Fills `binding-use`: not read yet.
pub(crate) fn binding_use_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::BindingUse as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Flow);
    Fill::Absent(AbsenceCause::NotYetRead)
}

/// Fills `binding-shadow`: not read yet.
pub(crate) fn binding_shadow_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::BindingShadow as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Flow);
    Fill::Absent(AbsenceCause::NotYetRead)
}

/// Fills `binding-resolution`: not read yet.
pub(crate) fn binding_resolution_rows<V: ModelView>(
    file: &mut V,
    _words: &mut FileWords,
    _rows: &mut dyn Rows<<languages::BindingResolution as CatalogRelation>::Row>,
) -> Fill {
    let _unread: ModelRead<'_> = file.semantics(ModelPart::Flow);
    Fill::Absent(AbsenceCause::NotYetRead)
}
