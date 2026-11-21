//! The fillers of the relations the semantic model answers: every row of a walked file from the model, built by the first fill
//! that needs it, and for five of them the rows of one key, looked up in the model without walking the file. The items are the
//! relations' own; their `fills!` arms stand here, beside the walk, because the model's row writers read the relations' catalogs.

use crate::domain::binding_facts::service::{
    binding_event_rows, binding_read_rows, binding_shadow_rows, binding_use_rows,
};
use crate::domain::file_view::types::PythonWalk;
use crate::domain::keyed_rows::service::{
    binding_read_rows_of, binding_shadow_rows_of, reference_rows_of, resolves_rows_of,
    scope_rows_of,
};
use crate::domain::lexical_facts::service::{
    defines_rows, implicit_symbol_rows, qualified_name_rows, reference_rows, resolves_rows,
    scope_rows,
};
use crate::domain::relations::collections::relations::binding_event::item::BindingEventItem;
use crate::domain::relations::collections::relations::binding_read::item::BindingReadItem;
use crate::domain::relations::collections::relations::binding_shadow::item::BindingShadowItem;
use crate::domain::relations::collections::relations::binding_use::item::BindingUseItem;
use crate::domain::relations::collections::relations::defines::item::DefinesItem;
use crate::domain::relations::collections::relations::implicit_symbol::item::ImplicitSymbolItem;
use crate::domain::relations::collections::relations::qualified_name::item::QualifiedNameItem;
use crate::domain::relations::collections::relations::reference::item::ReferenceItem;
use crate::domain::relations::collections::relations::resolves::item::ResolvesItem;
use crate::domain::relations::collections::relations::scope::item::ScopeItem;
use ryo::languages::{
    self, BindingReadColumn, BindingShadowColumn, Column, ReferenceColumn, ResolvesColumn,
    ScopeColumn,
};

ryo::fills! {
    ScopeItem => languages::Scope, walk(PythonWalk) |file, words, rows| {
        scope_rows(file, words, rows);
    }
    key(Column::Scope(ScopeColumn::Child)) |file, _words, key, rows| {
        scope_rows_of(file, key, rows);
    }
}

ryo::fills! {
    DefinesItem => languages::Defines, walk(PythonWalk) |file, words, rows| {
        defines_rows(file, words, rows);
    }
}

ryo::fills! {
    ReferenceItem => languages::Reference, walk(PythonWalk) |file, words, rows| {
        reference_rows(file, words, rows);
    }
    key(Column::Reference(ReferenceColumn::Occurrence)) |file, words, key, rows| {
        reference_rows_of(file, words, key, rows);
    }
}

ryo::fills! {
    ResolvesItem => languages::Resolves, walk(PythonWalk) |file, words, rows| {
        resolves_rows(file, words, rows);
    }
    key(Column::Resolves(ResolvesColumn::Occurrence)) |file, words, key, rows| {
        resolves_rows_of(file, words, key, rows);
    }
}

ryo::fills! {
    QualifiedNameItem => languages::QualifiedName, walk(PythonWalk) |file, words, rows| {
        qualified_name_rows(file, words, rows);
    }
}

ryo::fills! {
    ImplicitSymbolItem => languages::ImplicitSymbol, walk(PythonWalk) |file, words, rows| {
        implicit_symbol_rows(file, words, rows);
    }
}

ryo::fills! {
    BindingEventItem => languages::BindingEvent, walk(PythonWalk) |file, words, rows| {
        binding_event_rows(file, words, rows);
    }
}

ryo::fills! {
    BindingReadItem => languages::BindingRead, walk(PythonWalk) |file, words, rows| {
        binding_read_rows(file, words, rows);
    }
    key(Column::BindingRead(BindingReadColumn::Name)) |file, words, key, rows| {
        binding_read_rows_of(file, words, key, rows);
    }
}

ryo::fills! {
    BindingUseItem => languages::BindingUse, walk(PythonWalk) |file, words, rows| {
        binding_use_rows(file, words, rows);
    }
}

ryo::fills! {
    BindingShadowItem => languages::BindingShadow, walk(PythonWalk) |file, words, rows| {
        binding_shadow_rows(file, words, rows);
    }
    key(Column::BindingShadow(BindingShadowColumn::Binding)) |file, words, key, rows| {
        binding_shadow_rows_of(file, words, key, rows);
    }
}
