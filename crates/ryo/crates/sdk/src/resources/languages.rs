//! `languages`: what a language author and a program that links languages use.

#[cfg(feature = "fixtures")]
pub mod contract;

mod macros;

pub use crate::domain::language::types::Language;

pub use crate::domain::language::types::LanguageBuild;

pub use crate::domain::language::types::LanguageName;

#[doc(hidden)]
pub use macros::link::link;

/// Open on purpose: a language implements it, and nothing else does.
pub use ryo_language::inbound::use_cases::parse::interface::ParseUseCase as Parse;

pub use ryo_language::inbound::use_cases::parse::types::ParseRequest;

pub use ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity;

pub use ryo_language::inbound::use_cases::parse::errors::ParseError;
pub use ryo_language::inbound::use_cases::parse::types::{
    FieldRequest, KindNames, NoteRequest, Recognition,
};

/// Open on purpose: a language implements it, and nothing else does.
pub use ryo_language::inbound::use_cases::project::interface::ProjectUseCase as Project;

pub use ryo_language::inbound::use_cases::project::types::ProjectRequest;

pub use ryo_language::inbound::use_cases::project::types::ManifestFile;

pub use ryo_language::inbound::use_cases::project::types::RootPath;

pub use ryo_language::inbound::use_cases::project::types::Repository;

pub use ryo_language::inbound::use_cases::project::types::ManifestText;

pub use ryo_language::inbound::use_cases::parse::types::SkippedDirectory;

pub use ryo_language::domain::files::types::SourcePath;

pub use ryo_language::inbound::use_cases::parse::types::SampleText;

pub use ryo_language::inbound::use_cases::parse::types::Why;

pub use ryo_language::inbound::use_cases::parse::types::KindName;

pub use ryo_language::domain::relations::types::RowKind;

pub use ryo_language::domain::answers::service::source_locations;

/// What a language's tool or library said when a use case failed.
pub use ryo_language::domain::reasons::types::ToolMessage;
pub use ryo_language::inbound::use_cases::project::errors::ProjectError;

pub use ryo_language::domain::terms::types::Symbol;

pub use ryo_language::domain::relations::types::FileId;

pub use ryo_language::domain::relations::types::NodeId;

pub use ryo_language::domain::relations::types::Span;

pub use ryo_language::domain::relations::types::Value;

pub use ryo_language::domain::relations::types::CellKind;

pub use ryo_language::domain::relations::types::Relation;

pub use ryo_language::domain::relations::types::Column;

pub use ryo_language::domain::elements::types::{
    DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, Elements, TreeImpact,
};

pub use ryo_language::domain::elements::types::Children;

pub use ryo_language::domain::relations::constants::TOKEN;

pub use ryo_language::domain::relations::constants::{NO_FILE, NO_SYMBOL};

pub use ryo_language::domain::elements::service::enter_node;
pub use ryo_language::domain::elements::service::{decoded_at, decoded_token};

pub use ryo_language::domain::completion::service::name_kind;

pub use ryo_language::domain::completion::service::write_notes;

pub use ryo_language::domain::elements::service::leave_node;

pub use ryo_language::domain::elements::service::token;

pub use ryo_language::domain::elements::service::children;

pub use ryo_language::domain::elements::service::child_of_kind;

pub use ryo_language::domain::elements::service::first_token_text;

pub use ryo_language::domain::elements::service::text_at;

/// The parent, bit flags, size and last descendant of an element of the table: the readers a language walks its tree with.
pub use ryo_language::domain::elements::service::{flags_at, last_at, parent_at, size_at};

pub use ryo_language::domain::relations::service::node_id;

pub use ryo_language::domain::elements::service::clear_elements;

/// The numbering, names and list of a language's kinds, written from its vocabularies of nodes, tokens and extras.
pub use ryo_language::kinds;

/// A tree written from a library's events and the text between them, which a language's gap tokenizer writes, so that its tokens
/// spell the whole file.
pub use ryo_language::domain::trees::service::{
    fill_to, lossless, lossless_decoded_token, lossless_enter, lossless_leave, lossless_token,
    readable_text,
};
pub use ryo_language::domain::trees::types::{Lossless, ReadableText};

/// What a target a file writes (a link, an import, a url) names, as a path of the run.
pub use ryo_language::domain::targets::service::{reach_of, resolve_target};
pub use ryo_language::domain::targets::types::{ResolvedTarget, TargetReach, WrittenTarget};

pub use ryo_language::domain::symbols::types::Symbols;

pub use ryo_language::domain::symbols::types::NameStart;

pub use ryo_language::domain::symbols::service::intern_derived;

pub use ryo_language::domain::symbols::service::vocabulary_symbol;

pub use ryo_language::domain::symbols::service::begin_name;

pub use ryo_language::domain::symbols::service::push_text;

pub use ryo_language::domain::symbols::service::push_span;

pub use ryo_language::domain::symbols::service::push_name;

pub use ryo_language::domain::symbols::service::end_name;

pub use ryo_language::domain::elements::service::{
    diagnose, diagnose_with_metadata, has_incomplete_syntax,
};

pub use ryo_language::domain::expansions::service::adopt;

pub use ryo_language::domain::expansions::service::release;

pub use ryo_language::domain::elements::types::Diagnostic;

pub use ryo_language::domain::elements::service::diagnostic_count;

pub use ryo_language::domain::elements::service::diagnostics;

pub use ryo_language::domain::elements::service::ordinal_at;

pub use ryo_language::domain::symbols::service::has_noted_words;

pub use ryo_language::domain::symbols::service::note_of;

pub use ryo_language::domain::symbols::service::note_word;

pub use ryo_language::domain::symbols::service::set_note;

/// The seals of the item traits, which the declaration macro implements and nothing else does.
#[doc(hidden)]
pub use ryo_language::domain::answers::types::declared;

/// Sealed: the catalog is the only place a relation is defined, and a language reads relations, it does not add them.
pub use ryo_language::domain::relations::collections::relations::interface::RelationsItem as CatalogRelation;

pub use ryo_language::domain::relations::types::Offset;

pub use ryo_language::domain::terms::types::ChildOrdinal;

pub use ryo_language::domain::relations::types::Position;

pub use ryo_language::domain::relations::types::KindBits;

pub use ryo_language::domain::relations::types::Flags;

pub use ryo_language::domain::relations::types::Bits;

pub use ryo_language::domain::elements::service::end_at;

pub use ryo_language::domain::elements::service::kind_at;

pub use ryo_language::domain::elements::service::positions;

pub use ryo_language::domain::elements::service::start_at;

pub use ryo_language::domain::elements::service::end_of;

pub use ryo_language::domain::relations::service::node_file;

pub use ryo_language::domain::relations::service::node_position;

pub use ryo_language::domain::symbols::service::text_of;

pub use ryo_language::domain::elements::types::TableBuilder;

pub use ryo_language::domain::elements::types::Completion;

pub use ryo_language::domain::elements::service::table_builder;

pub use ryo_language::inbound::use_cases::parse::types::SourceFile;

pub use ryo_language::inbound::use_cases::parse::types::Sample;

pub use ryo_language::domain::relations::service::relation_named;

pub use ryo_language::domain::answers::types::Support;

pub use ryo_language::domain::answers::service::absent;

pub use ryo_language::domain::answers::service::reads;

pub use ryo_language::inbound::use_cases::project::service::reads_manifests;

pub use ryo_language::domain::relations::types::Count;

pub use ryo_language::domain::answers::types::Expansions;
pub use ryo_language::domain::completion::service::{name_field, write_fields};
pub use ryo_language::domain::elements::service::builder_position;
pub use ryo_language::domain::elements::service::embedded_source_at;
pub use ryo_language::domain::elements::service::{field_at, field_name_at, origin_at};
pub use ryo_language::domain::expansions::service::mark_expansion;
pub use ryo_language::domain::expansions::service::{embedded_expressions, write_embedded};
pub use ryo_language::domain::relations::constants::NO_FIELD;
pub use ryo_language::domain::relations::types::ReportText;
pub use ryo_language::domain::relations::types::{Bytes, Extent, Flag, Integer, Optionality};
pub use ryo_language::domain::relations::types::{ElementOrigin, ExpansionId, FieldId};
pub use ryo_language::domain::relations::types::{EmbeddedMapping, EmbeddedSource};
pub use ryo_language::domain::symbols::types::ByteSlice;
pub use ryo_language::domain::symbols::types::Noted;
pub use ryo_language::domain::terms::types::NameText;

pub use ryo_language::domain::relations::types::NameNote;

pub use ryo_language::domain::answers::types::AbsenceCause;

pub use ryo_language::domain::answers::types::Reads;

pub use ryo_language::domain::answers::types::Reader;
pub use ryo_language::domain::answers::types::Visiting;

pub use ryo_language::domain::answers::types::ReadItem;

pub use ryo_language::domain::elements::types::Tree;

pub use ryo_language::domain::relations::service::{diagnostic_id, provenance_of};
pub use ryo_language::domain::relations::types::{Folder, Holds, Provenance};

pub use ryo_language::domain::terms::types::DeclarationKind;

pub use ryo_language::domain::terms::types::ValueKind;

pub use ryo_language::domain::terms::types::MappingKind;

pub use ryo_language::domain::terms::types::{DependencyKind, DependencyOrigin};

pub use ryo_language::domain::terms::types::Term;

pub use ryo_language::domain::terms::service::{term_symbol, unknown_symbol};
pub use ryo_language::domain::terms::types::{ReasonName, UnknownName, UnknownReason};

pub use ryo_language::domain::relations::service::all;

pub use ryo_language::domain::relations::types::Missing;

pub use ryo_language::inbound::use_cases::project::types::ManifestReader;

pub use ryo_language::inbound::use_cases::project::types::ManifestItem;

pub use ryo_language::inbound::use_cases::project::types::ManifestInput;

pub use ryo_language::inbound::use_cases::project::types::ManifestName;

pub use ryo_language::inbound::use_cases::project::types::Resolved;

::ryo_language::each_relation!(self::macros::catalog::catalog_names);

pub use ryo_language::domain::files::types::FilePath;

pub use ryo_language::domain::terms::types::ParameterKind;

pub use ryo_language::domain::relations::types::Rows;

pub use ryo_language::inbound::use_cases::project::types::Refused;

pub use ryo_language::domain::elements::service::{
    enter_implied_node, node_property, node_property_fmt, property_at, source_order,
};
pub use ryo_language::domain::elements::types::SourceOrder;
pub use ryo_language::domain::relations::constants::{IMPLIED, NO_SPAN};
pub use ryo_language::domain::relations::service::location;
pub use ryo_language::domain::relations::types::PropertyId;

pub use ryo_language::domain::relations::types::Locatedness;

pub use ryo_language::domain::answers::service::{native_semantic_child, native_semantic_syntax};

pub use ryo_language::domain::relations::types::OptionalCapability;
