//! The contract of the `languages` traits an implementation runs: what every language provider
//! must pass for the engine's flow to hold. `parse` is the refusals, `flow` is the table and the
//! workers, `allocations` is the allocation count, `project` is the manifests, `conformance` holds every answer to the fixtures.

use ryo_language::inbound::use_cases::parse::constants::FIXTURES;
use ryo_language::inbound::use_cases::parse::fixtures::contract as parse_contract;
use ryo_language::inbound::use_cases::parse::fixtures::samples as parse_samples;
use ryo_language::inbound::use_cases::project::fixtures::contract as project_contract;

pub use ryo_language::domain::elements::service::{element_count, notes_of};
pub use ryo_language::domain::symbols::service::name_count;
pub use ryo_language::inbound::use_cases::parse::fixtures::contract::{Collect, Discard};
pub use ryo_language::inbound::use_cases::parse::fixtures::expected::{
    Cell, Count, Expected, ExpectedRow, FileName, Fixtures, ManifestExpected, Point, Stretch, Text,
    Unfilled, Unshown, Want, Wanted, expected_rows, file, kept, name, node, number, span, unknown,
    unlocated, want_file, want_flag, want_name, want_node, want_number, want_span, want_unknown,
};
pub use ryo_language::inbound::use_cases::parse::fixtures::expected::{
    Leafness, LibraryWalk, Oracle, OracleRows, ReferenceRefusal, RelationRows, Shape, Walked,
};
pub use ryo_language::inbound::use_cases::parse::fixtures::expected_rows::{
    analyzed_source::AnalyzedSourceExpectedRow, annotates::AnnotatesExpectedRow,
    assigns::AssignsExpectedRow, binding_event::BindingEventExpectedRow,
    binding_read::BindingReadExpectedRow, binding_shadow::BindingShadowExpectedRow,
    binding_use::BindingUseExpectedRow, call_argument::CallArgumentExpectedRow,
    comment::CommentExpectedRow, decorates::DecoratesExpectedRow, defines::DefinesExpectedRow,
    directive::DirectiveExpectedRow, embedded_expression::EmbeddedExpressionExpectedRow,
    entry::EntryExpectedRow, expression_shape::ExpressionShapeExpectedRow,
    file_package::FilePackageExpectedRow, filesystem_file::FilesystemFileExpectedRow,
    filesystem_line_count::FilesystemLineCountExpectedRow, function::FunctionExpectedRow,
    identifier::IdentifierExpectedRow, implicit_symbol::ImplicitSymbolExpectedRow,
    indent_region::IndentRegionExpectedRow, inherits::InheritsExpectedRow,
    mapping::MappingExpectedRow, mention::MentionExpectedRow,
    package_dependency::PackageDependencyExpectedRow, parameter::ParameterExpectedRow,
    prose::ProseExpectedRow, qualified_name::QualifiedNameExpectedRow,
    reference::ReferenceExpectedRow, resolves::ResolvesExpectedRow, scope::ScopeExpectedRow,
    semantic_child::SemanticChildExpectedRow, semantic_syntax::SemanticSyntaxExpectedRow,
    sequence_element::SequenceElementExpectedRow, source_dependency::SourceDependencyExpectedRow,
    source_location::SourceLocationExpectedRow, static_call::StaticCallExpectedRow,
    static_import::StaticImportExpectedRow, string_literal::StringLiteralExpectedRow,
    syntax_child::SyntaxChildExpectedRow, syntax_error::SyntaxErrorExpectedRow,
    syntax_tree::SyntaxTreeExpectedRow, visibility::VisibilityExpectedRow,
    workspace_member::WorkspaceMemberExpectedRow,
};
pub use ryo_language::inbound::use_cases::parse::fixtures::items::check as check_item;
pub use ryo_language::inbound::use_cases::parse::fixtures::reading::{Reading, written};
pub use ryo_language::inbound::use_cases::parse::fixtures::rows::point_of;
pub use ryo_language::inbound::use_cases::parse::service::{build, read, walk};
pub use ryo_language::inbound::use_cases::parse::types::KindSample;
pub use ryo_language::inbound::use_cases::project::service::project as read_project;

use super::{CatalogRelation, Parse, Project, ReadItem, SourceFile, SourcePath};
use std::path::Path;

/// Runs the refusals of the `parse` contract against a language.
///
/// # Panics
///
/// When the language breaks the contract.
pub fn parse(subject: &impl Parse) {
    parse_contract::check(subject);
}

/// Runs the flow rows of the `parse` contract: every sample gives a well-formed pre-order table, and
/// the same rows whether one worker reads the samples or ten.
///
/// # Panics
///
/// When the language breaks the contract.
pub fn flow(subject: &impl Parse, samples: &[SourceFile]) {
    parse_contract::flow(subject, samples);
}

/// Runs the allocation row of the `parse` contract: after a warm-up, reading a file allocates no
/// more than the parser library `library` allocates for it alone. The caller's test binary
/// installs `dhat::Alloc` as its global allocator and has no other thread running.
///
/// # Panics
///
/// When reading the sample allocates more than the library does, or the library refuses the sample the language reads.
pub fn allocations(
    subject: &impl Parse,
    sample: SourceFile,
    library: &dyn Fn(
        ryo_language::domain::symbols::types::ByteSlice<'_>,
    ) -> Result<(), super::ParseError>,
) {
    parse_samples::allocations(subject, sample, library);
}

/// Runs the rows of the `project` contract against a language.
///
/// # Panics
///
/// When the language breaks the contract.
pub fn project(subject: &impl Project) {
    project_contract::check(subject);
}

/// The directory of a language's crate, which holds its `tests/fixtures`.
#[derive(Clone, Copy, Debug)]
pub struct ManifestDir(pub &'static str);

/// Holds a linked language to the whole catalog, with its fixtures under `tests/fixtures` of its directory `manifest_dir` and the typed
/// rows `expected` it is held to: its name is its directory's, every relation it reads is given the fixture that shows it and gives the
/// rows expected, every relation it says is not in it has no fixture that shows the construct and says why, the manifest relations it
/// reads give rows from the project fixtures, and every kind of its trees is shown by a sample or marked unread, the kinds no relation
/// reads included.
///
/// # Panics
///
/// When the name of the language is not its directory's, or an answer of the language is not held by its fixtures, listing every one.
pub fn conformance(linked: &super::Language, manifest_dir: ManifestDir, expected: &Fixtures) {
    let directory = Path::new(manifest_dir.0);
    assert_eq!(
        Some(&*linked.name().0),
        directory.file_name().and_then(|name| name.to_str()),
        "the name of a language is the name of its directory"
    );
    ryo_language::inbound::use_cases::parse::fixtures::conformance::conformance(
        &linked.name().0,
        linked.parse_engine().as_ref(),
        linked.project().as_ref(),
        &directory.join(FIXTURES),
        expected,
    );
}

/// Runs the allocation row against a linked language, with its fixtures under `tests/fixtures` of its directory `manifest_dir`: reading
/// a file allocates no more than the baseline, what the language's own `read_tree` does for the file alone through a builder that builds nothing. It runs alone in its
/// process, with `dhat` as the global allocator, so nothing else counts.
///
/// # Panics
///
/// When reading a file allocates more than the baseline does.
pub fn allocations_of_linked(linked: &super::Language, manifest_dir: ManifestDir) {
    ryo_language::inbound::use_cases::parse::fixtures::samples::allocations_of_fixtures(
        linked.parse_engine().as_ref(),
        &Path::new(manifest_dir.0).join(FIXTURES),
    );
}

/// What the linked language writes of the relation `Item` reads for `source`, read with its own parse as the file numbered 0 at `path`:
/// the rows of the relation, every cell of each, and the causes for which it says the relation is read only in part. What the tests of
/// a language's items hold each item to.
///
/// # Errors
///
/// When the language refuses the source.
pub fn reading<Item>(
    linked: &super::Language,
    path: SourcePath<'_>,
    source: super::SampleText,
) -> Result<Reading, super::ParseError>
where
    Item: ReadItem,
{
    ryo_language::inbound::use_cases::parse::fixtures::reading::reading(
        linked.parse_engine().as_ref(),
        path,
        source,
        <Item::Relation as CatalogRelation>::relation(),
    )
}
