//! Clearing collected partial observations retains storage without retaining stale causes.
use super::Collect;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::{
    collections::relations::indent_region::item::IndentRegionRow,
    types::{FileId, Missing, Relation, ReportText, Rows},
};
#[derive(Clone, Copy, Debug)]
enum Operation {
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
    use crate::domain::terms::{
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
fn subject(operation: Input) -> Result<Value, Failure> {
    let mut output = Collect::default();
    match operation {
        Operation::Identifier(name, span) => {
            use crate::domain::{
                symbols::{
                    service::intern_derived,
                    types::{ByteSlice, Symbols},
                },
                terms::{
                    service::unknown_symbol,
                    types::{NameText, UnknownReason},
                },
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
