//! The language and relation vocabulary the executor reads, under the names the SDK's `languages` resource gives it. The executor sits
//! below the SDK, so it names the owning crates' items here instead of through the SDK.

pub use ryo_relations::domain::answers::types::AbsenceCause;
pub(crate) use ryo_language::inbound::use_cases::parse::constants::UNAVAILABLE_CAPABILITY;
pub(crate) use ryo_language::domain::symbols::types::ByteSlice;
pub(crate) use ryo_relations::domain::relations::types::Bytes;
pub(crate) use ryo_relations::domain::relations::types::CellKind;
pub(crate) use ryo_language::domain::facts::types::CellWord;
pub(crate) use ryo_relations::domain::terms::types::ChildOrdinal;
pub(crate) use ryo_relations::domain::relations::types::Count;
pub(crate) use ryo_relations::domain::elements::types::EmbeddedMapping;
pub(crate) use ryo_language::domain::node_table::types::EmbeddedText;
pub(crate) use ryo_language::domain::facts::types::FactAnswer;
pub(crate) use ryo_language::domain::node_table::types::FieldCode;
pub(crate) use ryo_language::domain::fillers::types::FieldNames;
pub(crate) use ryo_language::inbound::use_cases::parse::types::FieldRequest;
pub(crate) use ryo_relations::domain::relations::types::FileId;
pub use ryo_language::domain::files::types::FilePath;
pub(crate) use ryo_language::inbound::use_cases::parse::named::FileProject;
pub(crate) use ryo_language::inbound::use_cases::parse::named::FileRelation;
pub(crate) use ryo_language::inbound::use_cases::parse::named::FileRows;
pub(crate) use ryo_language::domain::facts::types::FileWords;
pub(crate) use ryo_language::domain::facts::types::FillExtent;
pub(crate) use ryo_language::domain::facts::types::FilledColumn;
pub(crate) use ryo_language::domain::facts::types::FilledColumns;
pub(crate) use ryo_relations::domain::relations::types::KindBits;
pub(crate) use ryo_language::inbound::use_cases::project::types::ManifestFile;
pub(crate) use ryo_language::inbound::use_cases::project::types::ManifestText;
pub(crate) use ryo_relations::domain::relations::types::Missing;
pub(crate) use ryo_relations::domain::relations::constants::NO_FILE;
pub(crate) use ryo_relations::domain::relations::constants::NO_SYMBOL;
pub(crate) use ryo_relations::domain::terms::types::NameText;
pub(crate) use ryo_relations::domain::relations::types::NodeId;
pub(crate) use ryo_language::domain::node_table::types::NodeTable;
pub(crate) use ryo_language::inbound::use_cases::parse::types::NoteRequest;
pub(crate) use ryo_language::inbound::use_cases::parse::named::NumberedFacts;
pub(crate) use ryo_language::inbound::use_cases::parse::named::NumberedFile;
pub(crate) use ryo_language::inbound::use_cases::parse::named::NumberedVisit;
pub(crate) use ryo_language::domain::project_context::types::ObservedEntries;
pub(crate) use ryo_language::domain::project_context::types::ObservedEntryState;
pub(crate) use ryo_relations::domain::relations::types::Offset;
pub(crate) use ryo_language::inbound::use_cases::parse::errors::ParseError;
pub(crate) use ryo_language::inbound::use_cases::parse::types::ParseRequest;
pub(crate) use ryo_relations::domain::relations::types::Position;
pub(crate) use ryo_language::inbound::use_cases::project::types::ProjectRequest;
pub(crate) use ryo_language::inbound::use_cases::parse::types::Recognition;
pub(crate) use ryo_language::inbound::use_cases::project::types::Refused;
pub(crate) use ryo_relations::domain::relations::types::Relation;
pub(crate) use ryo_language::domain::semantics::types::RenderContext;
pub(crate) use ryo_language::domain::semantics::types::RenderOutcome;
pub(crate) use ryo_language::domain::semantics::types::RenderRequest;
pub(crate) use ryo_language::inbound::use_cases::project::types::Repository;
pub(crate) use ryo_language::inbound::use_cases::project::types::Resolved;
pub(crate) use ryo_language::inbound::use_cases::project::types::RootPath;
pub use ryo_language::domain::files::types::SourcePath;
pub(crate) use ryo_language::inbound::use_cases::parse::named::SourceWords;
pub(crate) use ryo_language::inbound::use_cases::parse::named::Spelling;
pub(crate) use ryo_relations::domain::terms::types::Symbol;
pub(crate) use ryo_language::domain::symbols::types::Symbols;
pub(crate) use ryo_language::domain::elements::types::SyntaxAcceptance;
pub(crate) use ryo_language::domain::diagnostics::types::SyntaxCompleteness;
pub(crate) use ryo_relations::domain::terms::types::UnknownName;
pub(crate) use ryo_language::domain::facts::types::Wanted;
pub(crate) use ryo_language::domain::facts::service as facts;
pub(crate) use ryo_language::domain::symbols::service::intern_derived;
pub(crate) use ryo_language::domain::node_table::service as node_table;
pub(crate) use ryo_language::domain::symbols::service::text_of;
pub(crate) use ryo_relations::domain::terms::service::unknown_symbol;

/// What the executor takes from the language crate's test-support contract: the writer of a project read and the engine a language
/// is read through (the session package builds the language crate with the same feature today).
pub(crate) mod contract {
    pub(crate) use ryo_core::inbound::use_cases::lint::coordinates::FindingCoordinates;
    pub(crate) use ryo_language::domain::names::service::name_count;
    pub(crate) use ryo_language::inbound::use_cases::parse::collect::Collect;
    pub(crate) use ryo_language::inbound::use_cases::parse::types::ParseEngine;
}
