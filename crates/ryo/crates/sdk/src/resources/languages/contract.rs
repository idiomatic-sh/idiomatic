//! The contract of the `languages` traits an implementation runs: `project` is the manifests, and `routes_frozen` holds the walk of a
//! language to the rows recorded while its old reader still existed.

use ryo_language::inbound::use_cases::parse::constants::FIXTURES;
use ryo_language::inbound::use_cases::project::fixtures::contract as project_contract;

pub use ryo_core::inbound::use_cases::lint::coordinates::FindingCoordinates;
pub use ryo_language::domain::elements::service::{element_count, notes_of};
pub use ryo_language::domain::elements::service::{
    has_incomplete_lexical_stream, has_incomplete_syntax,
};
/// The growth harness a language's contract test holds its fillers to: the rows of a whole fill grow with the file as each
/// relation's catalog item declares, and a fill of a declared key writes that key's rows alone.
pub use ryo_language::domain::fillers::fixtures::growth::{
    Growth, GrowthFault, GrowthReport, KeyGrowth, grown,
};
pub use ryo_language::domain::names::service::name_count;
pub use ryo_language::inbound::use_cases::parse::fixtures::expected::{
    Cell, Count, Expected, ExpectedRow, FileName, Fixtures, ManifestExpected, Point,
    SemanticExpected, Stretch, Text, Unfilled, Unshown, expected_rows, file, name, node, number,
    span, unknown,
};
pub use ryo_language::inbound::use_cases::parse::fixtures::expected::{
    Leafness, LibraryWalk, Oracle, OracleRows, ReferenceRefusal, RelationRows, Shape, Walked,
};
pub use ryo_language::inbound::use_cases::parse::fixtures::expected_rows::{
    analyzed_source::AnalyzedSourceExpectedRow, annotates::AnnotatesExpectedRow,
    assigns::AssignsExpectedRow, attribute_path::AttributePathExpectedRow,
    binding_event::BindingEventExpectedRow, binding_read::BindingReadExpectedRow,
    binding_resolution::BindingResolutionExpectedRow,
    binding_shadow::BindingShadowExpectedRow, binding_use::BindingUseExpectedRow,
    brace_format_field::BraceFormatFieldExpectedRow,
    brace_format_parse::BraceFormatParseExpectedRow, bytes_literal::BytesLiteralExpectedRow,
    call_argument::CallArgumentExpectedRow, comment::CommentExpectedRow,
    compiler_feature::CompilerFeatureExpectedRow, constant_key::ConstantKeyExpectedRow,
    decorates::DecoratesExpectedRow, defines::DefinesExpectedRow, directive::DirectiveExpectedRow,
    embedded_expression::EmbeddedExpressionExpectedRow, entry::EntryExpectedRow,
    expression_shape::ExpressionShapeExpectedRow, expression_type::ExpressionTypeExpectedRow,
    file_package::FilePackageExpectedRow, filesystem_executable::FilesystemExecutableExpectedRow,
    filesystem_file::FilesystemFileExpectedRow,
    filesystem_line_count::FilesystemLineCountExpectedRow,
    filesystem_read_failure::FilesystemReadFailureExpectedRow, flow_block::FlowBlockExpectedRow,
    flow_edge::FlowEdgeExpectedRow, flow_member::FlowMemberExpectedRow,
    flow_step::FlowStepExpectedRow, format_field::FormatFieldExpectedRow,
    format_parse::FormatParseExpectedRow, format_specification::FormatSpecificationExpectedRow,
    function::FunctionExpectedRow, function_terminal::FunctionTerminalExpectedRow,
    identifier::IdentifierExpectedRow, implicit_symbol::ImplicitSymbolExpectedRow,
    indent_region::IndentRegionExpectedRow, inherits::InheritsExpectedRow,
    interpolated_string_literal::InterpolatedStringLiteralExpectedRow,
    language_target::LanguageTargetExpectedRow,
    literal_expression_parse::LiteralExpressionParseExpectedRow,
    literal_properties::LiteralPropertiesExpectedRow,
    lookup_scope::LookupScopeExpectedRow,
    manifest_diagnostic::ManifestDiagnosticExpectedRow, mapping::MappingExpectedRow,
    mention::MentionExpectedRow, module_identity::ModuleIdentityExpectedRow,
    numeric_literal::NumericLiteralExpectedRow, package_dependency::PackageDependencyExpectedRow,
    parameter::ParameterExpectedRow, pattern_child::PatternChildExpectedRow,
    pattern_parse::PatternParseExpectedRow, pattern_syntax::PatternSyntaxExpectedRow,
    project_namespace_membership::ProjectNamespaceMembershipExpectedRow, prose::ProseExpectedRow,
    qualified_name::QualifiedNameExpectedRow, raw_format_field::RawFormatFieldExpectedRow,
    raw_format_segment::RawFormatSegmentExpectedRow, reference::ReferenceExpectedRow,
    resolves::ResolvesExpectedRow, scope::ScopeExpectedRow,
    semantic_child::SemanticChildExpectedRow, semantic_syntax::SemanticSyntaxExpectedRow,
    sequence_element::SequenceElementExpectedRow, source_dependency::SourceDependencyExpectedRow,
    source_location::SourceLocationExpectedRow,
    standard_library_import::StandardLibraryImportExpectedRow, static_call::StaticCallExpectedRow,
    static_import::StaticImportExpectedRow, string_literal::StringLiteralExpectedRow,
    syntax_child::SyntaxChildExpectedRow, syntax_error::SyntaxErrorExpectedRow,
    syntax_tree::SyntaxTreeExpectedRow, token_link::TokenLinkExpectedRow,
    visibility::VisibilityExpectedRow, workspace_member::WorkspaceMemberExpectedRow,
};
pub use ryo_language::inbound::use_cases::parse::fixtures::reading::{Reading, written};
pub use ryo_language::inbound::use_cases::parse::fixtures::rows::point_of;
pub use ryo_language::inbound::use_cases::parse::fixtures::wanted::{
    Want, Wanted, kept, unlocated, want_file, want_flag, want_name, want_node, want_number,
    want_span, want_unknown,
};
pub use ryo_language::inbound::use_cases::parse::collect::Collect;
pub use ryo_language::inbound::use_cases::parse::types::KindSample;
/// The engine-side interfaces `Language::parse_engine` and `Language::project` hand a language's own tests.
pub use ryo_language::inbound::use_cases::parse::types::ParseEngine;
pub use ryo_language::inbound::use_cases::project::service::project as read_project;
pub use ryo_language::inbound::use_cases::project::types::ProjectEngine;

use super::{CatalogRelation, Parse, Project, ReadItem, SourceFile, SourcePath};
use std::path::Path;

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

/// The frozen row of the language contract: the walk's rows over the language's fixtures equal the rows FROZEN while the old reader
/// still existed (`tests/frozen/`): the digests of every sample and relation, the completeness of every sample, the node-table
/// relations and the word of every position as the numbered adapter lends them, and what the language states of itself.
///
/// DETECTS CHANGE, NOT CORRECTNESS: the files hold what the walk wrote when it equalled the old reader (the tag
/// `pre-old-readers-deletion` can make both routes' rows again); correctness is held by the byte oracles, the three scores and the 294
/// cases. With `RYO_ROUTES_RECORD` set to a reason, the files are written from the walk instead (the recorder, `routes-rerecord.sh`): a
/// decision of whoever runs it, whose diff is its review. With `RYO_ROUTES_CORPUS` and `RYO_ROUTES_CORPUS_NAME` set, the corpus named is
/// held to `rows.<name>.digests` too (a line for a file and relation only where the relation has rows; a missing line means no rows).
///
/// # Panics
///
/// When the walk differs from the frozen files.
pub fn routes_frozen(linked: &super::Language, manifest_dir: ManifestDir, language: &str) {
    use ryo_language::inbound::use_cases::parse::fixtures::frozen;
    let (parser, walk) = (linked.parse_engine(), linked.walk_engine());
    let fixtures = Path::new(manifest_dir.0).join(FIXTURES);
    let folder = Path::new(manifest_dir.0).join("tests/frozen");
    let corpus = std::env::var_os("RYO_ROUTES_CORPUS").zip(std::env::var("RYO_ROUTES_CORPUS_NAME").ok()).map(|(roots, name)| {
        let roots: Vec<std::path::PathBuf> = std::env::split_paths(&roots).filter(|root| !root.as_os_str().is_empty()).collect();
        (frozen::corpus_files(parser.as_ref(), &roots), name)
    });
    use ryo_language::inbound::use_cases::parse::fixtures::routes::{UNREACHABLE_KINDS, coverage, coverage_failures};
    let covered = coverage(parser.as_ref(), walk.as_ref(), &fixtures);
    let exemptions = std::fs::read_to_string(fixtures.join(UNREACHABLE_KINDS)).unwrap_or_default();
    let uncovered = coverage_failures(&covered, &exemptions);
    eprintln!("COVERAGE spelled {} | numbered {} | failures {}", covered.spelled.len(), covered.numbered, uncovered.len());
    let walked = frozen::record(parser.as_ref(), walk.as_ref(), &fixtures);
    if let Ok(reason) = std::env::var("RYO_ROUTES_RECORD") {
        let write = |name: &str, what: &str, body: &str| frozen::write_file(&folder.join(name), what, language, &reason, body);
        write("rows.digests", "the digest of the sorted rows of every sample and relation", &walked.rows);
        write("completeness.tsv", "the completeness of every sample", &walked.completeness);
        write("adapter.digests", "the node-table relations and the words of the positions as the numbered adapter lends them", &walked.adapter);
        write("metadata.tsv", "what the language states of itself", &walked.metadata);
        if let Some((files, name)) = &corpus {
            write(&format!("rows.{name}.digests"), "the digest of the sorted rows of every corpus file and relation that has rows", &frozen::corpus_text(parser.as_ref(), walk.as_ref(), files));
        }
        return;
    }
    let mut found = Vec::new();
    for (name, body) in [("rows.digests", &walked.rows), ("completeness.tsv", &walked.completeness), ("adapter.digests", &walked.adapter), ("metadata.tsv", &walked.metadata)] {
        found.extend(frozen::differences_from_file(&folder.join(name), body));
    }
    if let Some((files, name)) = &corpus {
        found.extend(frozen::differences_from_file(&folder.join(format!("rows.{name}.digests")), &frozen::corpus_text(parser.as_ref(), walk.as_ref(), files)));
    }
    assert!(uncovered.is_empty(), "the fixtures leave {} spelled kinds without text on the walk:\n{}", uncovered.len(), uncovered.join("\n"));
    assert!(found.is_empty(), "{} differences between the walk and the frozen rows of {language}:\n{}", found.len(), found.join("\n"));
}

/// Keyed and per-node columns for provider measurements.
pub mod columns;
