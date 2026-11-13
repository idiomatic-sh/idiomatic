//! A file's semantic session renders from the tree its read parsed: one read and its session parse the file once.

use super::adapter::SyntaxRuffPythonParserAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::{self, ByteSlice, Elements, Symbols};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Whether the session is begun from what the read kept in the table or from an empty slot.
#[derive(Clone, Copy)]
enum Begun {
    /// From the parsed tree the read kept.
    FromTheRead,
    /// From nothing: the session parses the file itself.
    FromNothing,
}

/// What a row asks: how the session of `x = y` is begun.
type Input = Begun;
/// How many whole-file parses the read and the session made, and whether `y` rendered as itself.
type Value = (usize, bool);
type Failure = ();

/// The rows: the positive control parses once, the negative control (nothing kept) parses twice, and both render `y`.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_read_and_its_semantic_session_parse_the_file_once"),
        word: Word::Succeeds,
        input: || Begun::FromTheRead,
        expect: Expect::Value((1, true)),
    },
    Case {
        name: CaseName("a_session_begun_from_nothing_parses_the_file_again"),
        word: Word::Succeeds,
        input: || Begun::FromNothing,
        expect: Expect::Value((2, true)),
    },
];

/// Reads `x = y`, begins its session as `begun` says, and renders `y`.
fn subject(begun: Input) -> Result<Value, Failure> {
    let source = b"x = y\n";
    let parses = Arc::new(AtomicUsize::new(0));
    let adapter = SyntaxRuffPythonParserAdapter {
        settings: PythonParserSettings::new(PythonTargetVersion::Py310),
        file_parses: Arc::clone(&parses),
        ..Default::default()
    };
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    adapter.read(
        SourceText(source),
        PythonSourceKind::Python,
        &mut languages::table_builder(&mut elements, &mut symbols, ByteSlice(source)),
    );
    let retained = match begun {
        Begun::FromTheRead => languages::take_semantics(&mut elements),
        Begun::FromNothing => languages::SemanticSlot::Empty,
    };
    let languages::SemanticSlot::Begun(mut session) =
        adapter.begin_semantics(&elements, PythonSourceKind::Python, retained)
    else {
        return Err(());
    };
    let at = languages::positions(&elements)
        .find(|&at| {
            languages::start_at(&elements, at) == languages::Offset(4)
                && languages::end_at(&elements, at) == languages::Offset(5)
        })
        .ok_or(())?;
    let rendered = session
        .render_batch(
            ByteSlice(source),
            &[languages::RenderRequest {
                node: languages::node_id(languages::FileId(0), at),
                context: languages::RenderContext::PositionalArgument,
            }],
        )
        .map_err(|_| ())?;
    let known = matches!(
        rendered.as_slice(),
        [languages::RenderOutcome::Known(text)] if &*text.0 == "y"
    );
    Ok((parses.load(Ordering::Relaxed), known))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
