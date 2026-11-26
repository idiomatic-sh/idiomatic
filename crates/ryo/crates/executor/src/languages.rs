//! The language and relation vocabulary the executor reads, under the names the SDK's `languages` resource gives it. The executor sits
//! below the SDK, so it names the owning crates' items here instead of through the SDK.

pub use ryo_relations::domain::answers::types::AbsenceCause;
pub use ryo_language::inbound::use_cases::parse::constants::UNAVAILABLE_CAPABILITY;
pub use ryo_language::domain::symbols::types::ByteSlice;
pub use ryo_relations::domain::relations::types::Bytes;
pub use ryo_relations::domain::relations::types::CellKind;
pub use ryo_language::domain::facts::types::CellWord;
pub use ryo_relations::domain::terms::types::ChildOrdinal;
pub use ryo_relations::domain::relations::types::Count;
pub use ryo_relations::domain::elements::types::EmbeddedMapping;
pub use ryo_language::domain::node_table::types::EmbeddedText;
pub use ryo_language::domain::facts::types::FactAnswer;
pub use ryo_language::domain::node_table::types::FieldCode;
pub use ryo_language::domain::fillers::types::FieldNames;
pub use ryo_language::inbound::use_cases::parse::types::FieldRequest;
pub use ryo_relations::domain::relations::types::FileId;
pub use ryo_language::domain::files::types::FilePath;
pub use ryo_language::inbound::use_cases::parse::named::FileProject;
pub use ryo_language::inbound::use_cases::parse::named::FileRelation;
pub use ryo_language::inbound::use_cases::parse::named::FileRows;
pub use ryo_language::domain::facts::types::FileWords;
pub use ryo_language::domain::facts::types::FillExtent;
pub use ryo_language::domain::facts::types::FilledColumn;
pub use ryo_language::domain::facts::types::FilledColumns;
pub use ryo_relations::domain::relations::types::KindBits;
pub use ryo_language::inbound::use_cases::project::types::ManifestFile;
pub use ryo_language::inbound::use_cases::project::types::ManifestText;
pub use ryo_relations::domain::relations::types::Missing;
pub use ryo_relations::domain::relations::constants::NO_FILE;
pub use ryo_relations::domain::relations::constants::NO_SYMBOL;
pub use ryo_relations::domain::terms::types::NameText;
pub use ryo_relations::domain::relations::types::NodeId;
pub use ryo_language::domain::node_table::types::NodeTable;
pub use ryo_language::inbound::use_cases::parse::types::NoteRequest;
pub use ryo_language::inbound::use_cases::parse::named::NumberedFacts;
pub use ryo_language::inbound::use_cases::parse::named::NumberedFile;
pub use ryo_language::inbound::use_cases::parse::named::NumberedVisit;
pub use ryo_language::domain::project_context::types::ObservedEntryState;
pub use ryo_relations::domain::relations::types::Offset;
pub use ryo_language::inbound::use_cases::parse::errors::ParseError;
pub use ryo_language::inbound::use_cases::parse::types::ParseRequest;
pub use ryo_relations::domain::relations::types::Position;
pub use ryo_language::inbound::use_cases::project::types::ProjectRequest;
pub use ryo_language::inbound::use_cases::parse::types::Recognition;
pub use ryo_language::inbound::use_cases::project::types::Refused;
pub use ryo_relations::domain::relations::types::Relation;
pub use ryo_language::domain::semantics::types::RenderContext;
pub use ryo_language::domain::semantics::types::RenderOutcome;
pub use ryo_language::domain::semantics::types::RenderRequest;
pub use ryo_language::inbound::use_cases::project::types::Repository;
pub use ryo_language::inbound::use_cases::project::types::Resolved;
pub use ryo_language::inbound::use_cases::project::types::RootPath;
pub use ryo_language::domain::files::types::SourcePath;
pub use ryo_language::inbound::use_cases::parse::named::SourceWords;
pub use ryo_language::inbound::use_cases::parse::named::Spelling;
pub use ryo_relations::domain::terms::types::Symbol;
pub use ryo_language::domain::symbols::types::Symbols;
pub use ryo_language::domain::elements::types::SyntaxAcceptance;
pub use ryo_language::domain::diagnostics::types::SyntaxCompleteness;
pub use ryo_relations::domain::terms::types::UnknownName;
pub use ryo_language::domain::facts::types::Wanted;
pub use ryo_language::domain::facts::service as facts;
pub use ryo_language::domain::symbols::service::intern_derived;
pub use ryo_language::domain::node_table::service as node_table;
pub use ryo_language::domain::symbols::service::text_of;
pub use ryo_relations::domain::terms::service::unknown_symbol;

/// What the executor takes from the language crate's test-support contract: the writer of a project read and the engine a language
/// is read through (the session package builds the language crate with the same feature today).
pub mod contract {
    pub use ryo_core::inbound::use_cases::lint::coordinates::FindingCoordinates;
    pub use ryo_language::domain::names::service::name_count;
    pub use ryo_language::inbound::use_cases::parse::collect::Collect;
    pub use ryo_language::inbound::use_cases::parse::types::ParseEngine;
}
