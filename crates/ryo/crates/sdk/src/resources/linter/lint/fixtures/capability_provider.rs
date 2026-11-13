//! The test data of `linter lint` that fixes capability provider.

use crate::domain::capacity::types::MemoryBudget;
use crate::domain::capacity::types::WorkerPool;
use crate::domain::language::types::Language;
use crate::domain::language::types::LanguageBuild;
use crate::domain::language::types::LanguageName;
use crate::domain::language::types::Languages;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::Linter;
use crate::resources::linter::lint::types::Workers;
use ryo_relations::domain::catalog::types::OptionalCapability;
use std::num::NonZeroUsize;

/// A provider with the same name/build whose advertised optional contract changes.
pub(crate) struct CapabilityProvider(
    pub(crate) &'static [ryo_relations::domain::catalog::types::OptionalCapability],
    pub(crate) &'static str,
    pub(crate) &'static str,
);

impl ryo_language::inbound::use_cases::parse::types::ParseEngine for CapabilityProvider {
    fn begin_semantics(
        &self,
        _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        _: &ryo_language::domain::elements::types::Elements,
        _: &mut ryo_language::domain::semantics::types::SemanticSlot,
    ) {
    }
    fn capabilities(&self) -> Vec<ryo_relations::domain::catalog::types::OptionalCapability> {
        self.0.to_vec()
    }
    fn configuration_identity(
        &self,
    ) -> ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity(self.1)
    }
    fn recognition(
        &self,
        _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
    ) -> ryo_language::inbound::use_cases::parse::types::Recognition {
        ryo_language::inbound::use_cases::parse::types::Recognition::Reads
    }
    fn skips(&self) -> &'static [ryo_language::inbound::use_cases::parse::types::SkippedDirectory] {
        &[]
    }
    fn kind_count(&self) -> ryo_relations::domain::relations::types::Count {
        ryo_relations::domain::relations::types::Count(1)
    }
    fn kind_names(&self) -> ryo_language::inbound::use_cases::parse::types::KindNames {
        ryo_language::inbound::use_cases::parse::types::KindNames(vec![
            ryo_relations::domain::terms::types::NameText("root"),
        ])
    }
    fn unknown_reasons(&self) -> &'static [ryo_relations::domain::terms::types::UnknownName] {
        &[]
    }
    fn reader_kinds(&self) -> Vec<ryo_relations::domain::relations::types::KindBits> {
        vec![ryo_relations::domain::relations::types::KindBits(0)]
    }
    fn build_file(
        &self,
        request: ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        table: &mut ryo_language::domain::elements::types::Elements,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
        use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder};
        use ryo_relations::domain::relations::types::{KindBits, Offset};
        let mut builder = table_builder(table, symbols, request.bytes);
        enter_node(&mut builder, KindBits(0), Offset(0));
        leave_node(&mut builder, Offset(1));
        ryo_language::domain::completion::service::name_kind(
            &mut ryo_language::domain::completion::service::completion(
                table,
                symbols,
                request.bytes,
                request.file,
            ),
            KindBits(0),
            ryo_relations::domain::terms::types::NameText("root"),
        );
        Ok(())
    }
    fn baseline_file(
        &self,
        _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
    ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
        Ok(())
    }
    fn read_file(
        &self,
        request: ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        table: &mut ryo_language::domain::elements::types::Elements,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        wanted: &[ryo_relations::domain::relations::types::Relation],
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
        let source = request.bytes;
        let file = request.file;
        self.build_file(request, table, symbols)?;
        self.walk_file(table, symbols, source, file, wanted, out);
        Ok(())
    }
    fn read_file_explicit(
        &self,
        _: ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        _: &mut ryo_language::domain::elements::types::Elements,
        _: &mut ryo_language::domain::symbols::types::Symbols,
        _: &[ryo_relations::domain::relations::types::Relation],
        _: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
        Err(ryo_language::inbound::use_cases::parse::errors::ParseError::Unsupported)
    }
    fn walk_file(
        &self,
        _: &mut ryo_language::domain::elements::types::Elements,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        source: ryo_language::domain::symbols::types::ByteSlice<'_>,
        file: ryo_relations::domain::relations::types::FileId,
        wanted: &[ryo_relations::domain::relations::types::Relation],
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) {
        use ryo_relations::domain::catalog::types::OptionalCapability;
        use ryo_relations::domain::relations::collections::relations::identifier::item::IdentifierRow;
        use ryo_relations::domain::relations::types::{
            Missing, Offset, Position, Relation, ReportText, Rows, Span,
        };
        use ryo_relations::domain::terms::types::NameText;
        if wanted.contains(&Relation::Identifier) {
            if self.0.contains(&OptionalCapability::Identifier) {
                out.write(IdentifierRow {
                    id: ryo_relations::domain::relations::service::node_id(file, Position(0)),
                    name: ryo_language::domain::symbols::service::intern_derived(
                        symbols,
                        source,
                        NameText(self.2),
                    ),
                    file,
                    span: Span {
                        start: Offset(0),
                        end: Offset(1),
                    },
                });
            } else {
                <dyn ryo_relations::domain::relations::types::RowWriter as Rows<
                    IdentifierRow,
                >>::partial_at(
                    out,
                    file,
                    ReportText("unadvertised fixture capability"),
                    Missing::Any,
                );
            }
        }
        if wanted.contains(&Relation::LiteralExpressionParse) {
            literal_parse_capability_row(
                self.0.contains(&OptionalCapability::LiteralExpressionParse),
                source,
                file,
                symbols,
                out,
            );
        }
        if wanted.contains(&Relation::RawFormatSegment) {
            use ryo_relations::domain::relations::collections::relations::raw_format_segment::item::RawFormatSegmentRow;
            if self.0.contains(&OptionalCapability::RawFormatSegment) {
                let intern =
                    |interned_symbols: &mut ryo_language::domain::symbols::types::Symbols,
                     text: &str| {
                        ryo_language::domain::symbols::service::intern_derived(
                            interned_symbols,
                            source,
                            NameText(text),
                        )
                    };
                let id = ryo_relations::domain::relations::service::node_id(file, Position(0));
                out.write(RawFormatSegmentRow {
                    id,
                    segment: id,
                    segment_ordinal: ryo_relations::domain::relations::types::Integer(0),
                    dialect: intern(symbols, "percent-string"),
                    outcome: intern(symbols, "parsed"),
                    error_category: intern(symbols, "none"),
                    error_character: intern(symbols, ""),
                    error_index: ryo_relations::domain::relations::types::Integer(0),
                    positional_consumption: ryo_relations::domain::relations::types::Integer(0),
                    mapping_present: ryo_relations::domain::relations::types::Flag::Lowered,
                    dynamic_width: ryo_relations::domain::relations::types::Flag::Lowered,
                    dynamic_precision: ryo_relations::domain::relations::types::Flag::Lowered,
                    file,
                    span: Span {
                        start: Offset(0),
                        end: Offset(1),
                    },
                });
            } else {
                <dyn ryo_relations::domain::relations::types::RowWriter as Rows<
                    RawFormatSegmentRow,
                >>::partial_at(
                    out,
                    file,
                    ReportText("unadvertised raw format capability"),
                    Missing::Any,
                );
            }
        }
    }
    fn absence_of(
        &self,
        relation: ryo_relations::domain::relations::types::Relation,
    ) -> Option<ryo_relations::domain::answers::types::AbsenceCause> {
        use ryo_relations::domain::answers::types::AbsenceCause;
        use ryo_relations::domain::catalog::types::OptionalCapability;
        use ryo_relations::domain::relations::types::Relation;
        ((relation == Relation::Identifier && !self.0.contains(&OptionalCapability::Identifier))
            || (relation == Relation::RawFormatSegment
                && !self.0.contains(&OptionalCapability::RawFormatSegment))
            || (relation == Relation::LiteralExpressionParse
                && !self.0.contains(&OptionalCapability::LiteralExpressionParse)))
        .then_some(AbsenceCause::NotYetRead)
    }
}

/// Emit or explicitly refuse the optional literal fact in the persistent-identity fixture.
fn literal_parse_capability_row(
    available: bool,
    source: ryo_language::domain::symbols::types::ByteSlice<'_>,
    file: ryo_relations::domain::relations::types::FileId,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
    out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
) {
    use ryo_relations::domain::relations::{
        collections::relations::literal_expression_parse::item::LiteralExpressionParseRow,
        types::{Missing, Offset, Position, ReportText, Rows, Span},
    };
    use ryo_relations::domain::terms::types::NameText;
    if available {
        let dialect = ryo_language::domain::symbols::service::intern_derived(
            symbols,
            source,
            NameText("python-string-annotation"),
        );
        let outcome = ryo_language::domain::symbols::service::intern_derived(
            symbols,
            source,
            NameText("parsed"),
        );
        out.write(LiteralExpressionParseRow {
            id: ryo_relations::domain::relations::service::node_id(file, Position(0)),
            dialect,
            outcome,
            file,
            span: Span {
                start: Offset(0),
                end: Offset(1),
            },
        });
    } else {
        <dyn ryo_relations::domain::relations::types::RowWriter as Rows<
            LiteralExpressionParseRow,
        >>::partial_at(
            out,
            file,
            ReportText("unadvertised literal parse capability"),
            Missing::Any,
        );
    }
}

/// A linter over one fixture provider, retaining into `cache`: `build` and `identity` are what its cache identity is made of, `present`
/// the name its identifier answers.
pub(crate) fn provider_linter(
    build: &str,
    identity: &'static str,
    capabilities: &'static [OptionalCapability],
    present: &'static str,
    cache: Cache,
) -> Linter {
    Linter::new(
        Languages(vec![Language::from_parts(
            LanguageName("fixture".into()),
            LanguageBuild(build.into()),
            std::sync::Arc::new(CapabilityProvider(capabilities, identity, present)),
            std::sync::Arc::new(
                ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
            ),
        )]),
        WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        cache,
    )
}
