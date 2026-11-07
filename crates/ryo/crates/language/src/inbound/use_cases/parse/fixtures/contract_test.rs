//! Clearing collected partial observations retains storage without retaining stale causes.
use super::Collect;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::{
    collections::relations::indent_region::item::IndentRegionRow,
    types::{FileId, Missing, Relation, ReportText, Rows},
};
#[derive(Clone, Copy, Debug)]
enum Operation {
    /// Leading trivia is not an expression root.
    EmbeddedRoot(bool),
    /// Classes must be backed by same-file representative rows.
    Shapes(u32, bool),

    /// The semantic projection must preserve native identities and values.
    Projection(u64),
    /// A field predicate must preserve its native selected edges.
    FieldProjection(u64),
    /// Physical span bounds do not follow the length of decoded logical text.
    SemanticSpan(u64),
    /// Explicit parser-implied nodes have no fabricated physical coordinates.
    ImpliedSpan(u64),
    /// Canonical spelling and original source interval.
    Identifier(Option<&'static str>, u64),
    Partial,
    PartialAt,
    Qualified(&'static str, Option<&'static str>, u64, u64),
}
type Input = Operation;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("embedded root is an actual node under its carrier"),
        word: Word::Succeeds,
        input: || Operation::EmbeddedRoot(false),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("leading trivia cannot become an embedded expression root"),
        word: Word::Succeeds,
        input: || Operation::EmbeddedRoot(true),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("known class is backed by its own source occurrence"),
        word: Word::Succeeds,
        input: || Operation::Shapes(3, true),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown sentinel is not a structural class"),
        word: Word::Succeeds,
        input: || Operation::Shapes(3, false),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("class from another file is refused"),
        word: Word::Succeeds,
        input: || Operation::Shapes(4, true),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("parser implied semantic node retains absent span"),
        word: Word::Succeeds,
        input: || Operation::ImpliedSpan(u64::from(u32::MAX) << 32_u32),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("ordinary semantic node cannot invent absent span"),
        word: Word::Succeeds,
        input: || Operation::SemanticSpan(u64::from(u32::MAX) << 32_u32),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("semantic field predicate selects native edge"),
        word: Word::Succeeds,
        input: || Operation::FieldProjection(7),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("uncomputed semantic field cannot silently lose native match"),
        word: Word::Succeeds,
        input: || Operation::FieldProjection(0),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("semantic projection preserves native rows"),
        word: Word::Succeeds,
        input: || Operation::Projection(7),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("semantic projection cannot replace native token text"),
        word: Word::Succeeds,
        input: || Operation::Projection(11),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("semantic span may cover a complete physical carrier"),
        word: Word::Succeeds,
        input: || Operation::SemanticSpan(6),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("semantic span beyond physical file is refused"),
        word: Word::Succeeds,
        input: || Operation::SemanticSpan(7),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("normalized identifier can differ in byte length"),
        word: Word::Succeeds,
        input: || Operation::Identifier(Some("exec"), 16),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown identifier cannot masquerade as canonical"),
        word: Word::Succeeds,
        input: || Operation::Identifier(None, 16),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("empty identifier is invalid"),
        word: Word::Succeeds,
        input: || Operation::Identifier(Some(""), 16),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("source span cannot use normalized byte extent beyond file"),
        word: Word::Succeeds,
        input: || Operation::Identifier(Some("exec"), 17),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("partial causes reuse buffers and preserve fresh content"),
        word: Word::Succeeds,
        input: || Operation::Partial,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("file partials clear old identity and reuse buffers"),
        word: Word::Succeeds,
        input: || Operation::PartialAt,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("proven builtin requires explicit availability"),
        word: Word::Succeeds,
        input: || Operation::Qualified("builtin", Some("builtins.value"), 3, 11),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("missing builtin availability is refused"),
        word: Word::Succeeds,
        input: || Operation::Qualified("builtin", Some("builtins.value"), 0, 0),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("known import is independent of builtin availability"),
        word: Word::Succeeds,
        input: || Operation::Qualified("import", Some("module.value"), 0, 0),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("local value must not invent a canonical path"),
        word: Word::Succeeds,
        input: || Operation::Qualified("local", Some("module.value"), 0, 0),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("dynamic unknown identity remains a complete valid row"),
        word: Word::Succeeds,
        input: || Operation::Qualified("local", None, 0, 0),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown provenance category is refused"),
        word: Word::Succeeds,
        input: || Operation::Qualified("guessed", None, 0, 0),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("import cannot masquerade as versioned builtin"),
        word: Word::Succeeds,
        input: || Operation::Qualified("import", Some("module.value"), 3, 11),
        expect: Expect::Value(false),
    },
];
fn qualified(origin: &str, path: Option<&str>, major: u64, minor: u64) -> Result<Value, Failure> {
    use crate::domain::symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    };
    use ryo_relations::domain::terms::{
        service::unknown_symbol,
        types::{NameText, UnknownReason},
    };
    let source = ByteSlice(b"");
    let mut symbols = Symbols::default();
    let path = path.map_or_else(
        || unknown_symbol(UnknownReason(23)),
        |text| intern_derived(&mut symbols, source, NameText(text)),
    );
    let provenance = intern_derived(&mut symbols, source, NameText(origin));
    Ok(super::qualified_name_valid(
        &[
            0,
            u64::from(path.0),
            u64::from(provenance.0),
            major,
            minor,
            0,
            0,
        ],
        &symbols,
        source,
    ))
}
#[expect(
    clippy::too_many_lines,
    reason = "one contract dispatch keeps every parser-diagnostic transport case aligned with its table"
)]
fn subject(operation: Input) -> Result<Value, Failure> {
    let mut output = Collect::default();
    match operation {
        Operation::EmbeddedRoot(leading_token) => {
            use crate::domain::builders::service::{enter_node, leave_node, table_builder, token};
            use crate::domain::{
                elements::types::Elements,
                expansions::service::write_embedded,
                symbols::types::{ByteSlice, Symbols},
            };
            use ryo_relations::domain::elements::types::EmbeddedMapping;
            use ryo_relations::domain::relations::types::{KindBits, Offset, Position};
            let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
            let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"x"));
            enter_node(&mut builder, KindBits(1), Offset(0));
            leave_node(&mut builder, Offset(1));
            write_embedded(
                &mut builder,
                Position(0),
                ByteSlice(b"x"),
                EmbeddedMapping::Exact(Offset(0)),
                |logical| {
                    if leading_token {
                        token(logical, KindBits(2), Offset(0), Offset(0));
                    }
                    enter_node(logical, KindBits(3), Offset(0));
                    token(logical, KindBits(2), Offset(0), Offset(1));
                    leave_node(logical, Offset(1));
                },
            );
            Ok(super::embedded_roots_valid(&elements))
        }

        Operation::Shapes(class_file, representative) => {
            use ryo_relations::domain::relations::{
                collections::relations::expression_shape::item::ExpressionShapeRow,
                service::node_id,
                types::{Offset, Position, Span},
            };
            let class = node_id(FileId(class_file), Position(6));
            let row = ExpressionShapeRow {
                id: node_id(FileId(3), Position(2)),
                shape: class,
                file: FileId(3),
                span: Span {
                    start: Offset(0),
                    end: Offset(1),
                },
            };
            <Collect as Rows<ExpressionShapeRow>>::write(&mut output, row);
            if representative {
                <Collect as Rows<ExpressionShapeRow>>::write(
                    &mut output,
                    ExpressionShapeRow { id: class, ..row },
                );
            }
            Ok(super::shape_representatives_valid(&output))
        }

        Operation::FieldProjection(field) => {
            output
                .relations
                .extend([Relation::SyntaxChild, Relation::SemanticChild]);
            output.words.extend([3, 5, 0, 7, 0, 3, 5, 0, field, 0]);
            Ok(super::filtered_projection_valid(&output, 7))
        }
        Operation::Projection(text) => {
            output
                .relations
                .extend([Relation::SyntaxTree, Relation::SemanticSyntax]);
            output.words.extend([3, 5, 7, 0, 1, 3, 5, text, 0, 1]);
            Ok(super::projection_preserves_native(&output))
        }
        Operation::SemanticSpan(end) => {
            output.relations.push(Relation::SemanticSyntax);
            output.words.extend([3, 5, 7, 0, end]);
            Ok(super::semantic_spans_valid(&output, 6, false, |_| false))
        }
        Operation::ImpliedSpan(span) => {
            output.relations.push(Relation::SemanticSyntax);
            output.words.extend([3, 5, 7, 0, span]);
            Ok(super::semantic_spans_valid(&output, 6, false, |id| id == 3))
        }
        Operation::Identifier(name, span) => {
            use crate::domain::symbols::{
                service::intern_derived,
                types::{ByteSlice, Symbols},
            };
            use ryo_relations::domain::terms::{
                service::unknown_symbol,
                types::{NameText, UnknownReason},
            };
            let source = ByteSlice("𝗲𝘅𝗲𝗰".as_bytes());
            let mut symbols = Symbols::default();
            let name = name.map_or_else(
                || unknown_symbol(UnknownReason(23)),
                |name| intern_derived(&mut symbols, source, NameText(name)),
            );
            Ok(super::identifier_valid(
                &[0, u64::from(name.0), 0, span],
                &symbols,
                source,
            ))
        }

        Operation::Qualified(origin, path, major, minor) => qualified(origin, path, major, minor),
        Operation::Partial => {
            <Collect as Rows<IndentRegionRow>>::partial(
                &mut output,
                ReportText("old long cause"),
                Missing::Any,
            );
            let pointer = output.partials[0].1.as_ptr();
            output.clear();
            assert!(output.partials.is_empty());
            assert_eq!(
                output.cause_buffers.first().map(String::as_str),
                Some("old long cause")
            );
            <Collect as Rows<IndentRegionRow>>::partial(
                &mut output,
                ReportText("new cause"),
                Missing::Any,
            );
            <Collect as Rows<IndentRegionRow>>::partial(
                &mut output,
                ReportText("new cause"),
                Missing::Any,
            );
            Ok(
                output.partials == [(Relation::IndentRegion, "new cause".into(), Missing::Any)]
                    && output.partials[0].1.as_ptr() == pointer,
            )
        }
        Operation::PartialAt => {
            <Collect as Rows<IndentRegionRow>>::partial_at(
                &mut output,
                FileId(3),
                ReportText("old long cause"),
                Missing::Any,
            );
            let pointer = output.partials_at[0].2.as_ptr();
            output.clear();
            assert!(output.partials_at.is_empty());
            <Collect as Rows<IndentRegionRow>>::partial_at(
                &mut output,
                FileId(5),
                ReportText("new cause"),
                Missing::Any,
            );
            Ok(
                output.partials_at == [(Relation::IndentRegion, FileId(5), "new cause".into())]
                    && output.partials_at[0].2.as_ptr() == pointer,
            )
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
