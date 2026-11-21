//! The `facts` service: the words of a file, the one way fillers, the engine and the host turn texts into words and back, the
//! word of a language's fixed name by its handle, and the `syntax-error` rows every language writes from its node table.

use super::types::{BaseSet, BaseWords, CellWord, FileWords, InternedText, WordSet, WordsEngine};
use crate::domain::diagnostics::service::{impact_name, listed, phase_name};
use crate::domain::fixed_names::types::{CheckedNames, FixedWord};
use crate::domain::node_table::types::NodeTable;
use ryo_relations::domain::relations::collections::relations::syntax_error::item::SyntaxErrorRow;
use ryo_relations::domain::relations::service::diagnostic_id;
use ryo_relations::domain::relations::types::Rows;
use ryo_relations::domain::relations::types::{Count, FileId, Span};
use ryo_relations::domain::terms::types::{ChildOrdinal, NameText, Symbol};
use std::sync::Arc;

/// The code of `text` in `set`, adding it (coded after `from`'s words) when the set does not hold it.
fn added(set: &mut WordSet, from: CellWord, text: NameText<'_>) -> CellWord {
    if let Some(code) = set.codes.get(text.0) {
        return *code;
    }
    let code = CellWord(
        from.0
            .saturating_add(u64::try_from(set.texts.len()).unwrap_or(u64::MAX)),
    );
    set.texts.push(InternedText(text.0.into()));
    let previous = set.codes.insert(InternedText(text.0.into()), code);
    debug_assert!(previous.is_none());
    code
}

/// The word of the fixed name at a place: the place after the empty text, which is word 0. The one rule `base_words` holds the
/// fixed names to and `fixed` answers by.
fn place_word(place: FixedWord) -> CellWord {
    CellWord(u64::from(place.0).saturating_add(1))
}

/// The run's base words, each text once, coded from zero: word 0 is the empty text, then the language's fixed names in the order
/// of their places, then `texts` in the order given. A fixed name's word is its place after the empty text, by construction:
/// the names are checked (`fixed_names::service::checked`), each stated once and none empty.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, file_words, text};
/// use ryo_language::domain::facts::types::CellWord;
/// use ryo_language::domain::fixed_names::service::checked;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let names = [NameText("next-token")];
/// let fixed = checked(&names).expect("one name, stated once");
/// let words = file_words(&base_words(fixed, &[NameText("print"), NameText("eval")]));
/// assert_eq!(text(&words, CellWord(0)), NameText(""));
/// assert_eq!(text(&words, CellWord(1)), NameText("next-token"));
/// assert_eq!(text(&words, CellWord(3)), NameText("eval"));
/// ```
#[must_use]
pub fn base_words(fixed: CheckedNames<'_>, texts: &[NameText<'_>]) -> BaseWords {
    let mut words = WordSet::default();
    let empty = added(
        &mut words,
        CellWord::default(),
        NameText(Default::default()),
    );
    debug_assert_eq!(empty, CellWord::default());
    for (number, each) in fixed.0.iter().enumerate() {
        let place = FixedWord(u32::try_from(number).unwrap_or(u32::MAX));
        let word = added(&mut words, CellWord::default(), *each);
        debug_assert_eq!(
            word,
            place_word(place),
            "a checked name is new to the words: its word is its place after the empty text"
        );
    }
    let codes: Vec<CellWord> = texts
        .iter()
        .map(|each| added(&mut words, CellWord::default(), *each))
        .collect();
    debug_assert_eq!(codes.len(), texts.len());
    BaseWords(Arc::new(BaseSet {
        words,
        fixed: Count(fixed.0.len()),
    }))
}

/// The word of a language's fixed name, by its handle: its place after the empty text, with no lookup and no table, and the same
/// cell in every file of the run. A release run does not check the handle, which is sound: a handle is a constant of a
/// language's own filler code, every filler runs under the contract's harness in test builds, where the assertion below
/// holds it, and the names cannot leave their places because `base_words` takes checked names alone: none stated twice, none empty.
///
/// # Panics
///
/// In a build with debug assertions, when the handle is past the fixed names the base words were made with: the words were
/// made without the language's vocabulary, or the language's handle is not a place in the names it states.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, clear_words, file_words, fixed, intern};
/// use ryo_language::domain::fixed_names::service::checked;
/// use ryo_language::domain::fixed_names::types::FixedWord;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let names = [NameText("next-token")];
/// let mut words = file_words(&base_words(checked(&names).expect("one name, stated once"), &[]));
/// let first = fixed(&words, FixedWord(0));
/// clear_words(&mut words);
/// assert_eq!(fixed(&words, FixedWord(0)), first);
/// assert_eq!(intern(&mut words, NameText("next-token")), first);
/// ```
#[must_use]
pub fn fixed(words: &FileWords, word: FixedWord) -> CellWord {
    debug_assert!(
        usize::try_from(word.0).is_ok_and(|place| place < words.0.base.0.fixed.0),
        "the handle {word:?} is past the {} fixed names the base words were made with",
        words.0.base.0.fixed.0
    );
    place_word(word)
}

/// A worker's words over the run's base words, with no word of a file yet.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, file_words, intern};
/// use ryo_language::domain::facts::types::CellWord;
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut words = file_words(&base_words(CheckedNames::default(), &[NameText("print")]));
/// assert_eq!(intern(&mut words, NameText("print")), CellWord(1));
/// ```
#[must_use]
pub fn file_words(base: &BaseWords) -> FileWords {
    FileWords(WordsEngine {
        base: base.clone(),
        own: WordSet::default(),
    })
}

/// Forgets the words of the last file, keeping their room: what a worker calls before the next file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, clear_words, file_words, intern};
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut words = file_words(&base_words(CheckedNames::default(), &[]));
/// let first = intern(&mut words, NameText("a"));
/// clear_words(&mut words);
/// assert_eq!(intern(&mut words, NameText("b")), first);
/// ```
pub fn clear_words(words: &mut FileWords) {
    words.0.own.texts.clear();
    words.0.own.codes.clear();
}

/// The code of a file's first own word: after the run's base words. A word below it is the same text in every file of
/// the run; a word from it on is this file's own, and the next file numbers its own words from here again.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_count, base_words, file_words, intern};
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut words = file_words(&base_words(CheckedNames::default(), &[NameText("kind")]));
/// let first = base_count(&words);
/// assert!(intern(&mut words, NameText("kind")) < first);
/// assert_eq!(intern(&mut words, NameText("local_name")), first);
/// ```
#[must_use]
pub fn base_count(words: &FileWords) -> CellWord {
    CellWord(u64::try_from(words.0.base.0.words.texts.len()).unwrap_or(u64::MAX))
}

/// The code after the file's last word: a cell below it is a word (of the run, or one this file interned so far), and a
/// cell from it on is no word at all, whatever else it is. It grows as the file interns texts.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_count, base_words, file_words, intern, words_end};
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut words = file_words(&base_words(CheckedNames::default(), &[NameText("kind")]));
/// assert_eq!(words_end(&words), base_count(&words));
/// let own = intern(&mut words, NameText("local_name"));
/// assert!(own < words_end(&words));
/// ```
#[must_use]
pub fn words_end(words: &FileWords) -> CellWord {
    CellWord(
        base_count(words)
            .0
            .saturating_add(u64::try_from(words.0.own.texts.len()).unwrap_or(u64::MAX)),
    )
}

/// The text a word stands for: a base word of the run or a word this file interned. A code no table holds answers the empty
/// text.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, file_words, intern, text};
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut words = file_words(&base_words(CheckedNames::default(), &[NameText("print")]));
/// let own = intern(&mut words, NameText("local_name"));
/// assert_eq!(text(&words, own), NameText("local_name"));
/// ```
#[must_use]
pub fn text(words: &FileWords, cell: CellWord) -> NameText<'_> {
    let base = base_count(words);
    let found = if cell < base {
        words
            .0
            .base
            .0
            .words
            .texts
            .get(usize::try_from(cell.0).unwrap_or(usize::MAX))
    } else {
        words
            .0
            .own
            .texts
            .get(usize::try_from(cell.0.saturating_sub(base.0)).unwrap_or(usize::MAX))
    };
    NameText(found.map_or_else(Default::default, |each| &*each.0))
}

/// The word for `text`: its base word when the run has one, else a word of this file, interned once.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, file_words, intern};
/// use ryo_language::domain::facts::types::CellWord;
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut words = file_words(&base_words(CheckedNames::default(), &[NameText("print")]));
/// assert_eq!(intern(&mut words, NameText("x")), CellWord(2));
/// assert_eq!(intern(&mut words, NameText("x")), CellWord(2));
/// ```
pub fn intern(words: &mut FileWords, text: NameText<'_>) -> CellWord {
    if let Some(code) = words.0.base.0.words.codes.get(text.0) {
        return *code;
    }
    let base = base_count(words);
    added(&mut words.0.own, base, text)
}

/// The word for `text` as a row's text cell holds it.
pub(crate) fn symbol(words: &mut FileWords, text: NameText<'_>) -> Symbol {
    Symbol(u32::try_from(intern(words, text).0).unwrap_or(u32::MAX))
}

/// Writes the `syntax-error` rows of the file, one for each diagnostic the parser reported, in the order it reported them: the
/// body every language's `syntax-error` filler calls.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, file_words, syntax_error_rows};
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_language::domain::node_table::service::{builder, diagnose, enter_node, finished, leave_node};
/// use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
/// use ryo_relations::domain::relations::collections::relations::syntax_error::item::SyntaxErrorRow;
/// use ryo_relations::domain::relations::types::{Count, FileId, KindBits, Missing, Offset, ReportText, Rows};
/// use ryo_relations::domain::terms::types::NameText;
///
/// struct Kept(Vec<SyntaxErrorRow>);
/// impl Rows<SyntaxErrorRow> for Kept {
///     fn write(&mut self, row: SyntaxErrorRow) { self.0.push(row); }
///     fn partial(&mut self, _: ReportText<'_>, _: Missing) {}
///     fn partial_at(&mut self, _: FileId, _: ReportText<'_>, _: Missing) {}
/// }
/// let mut table = NodeTable::default();
/// let mut writer = builder(&mut table, Count(2));
/// enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
/// diagnose(&mut writer, Offset(0), Offset(1), NameText("unexpected token"));
/// leave_node(&mut writer, Offset(1));
/// let mut words = file_words(&base_words(CheckedNames::default(), &[]));
/// let mut kept = Kept(Vec::new());
/// syntax_error_rows(finished(&mut writer), FileId(3), &mut words, &mut kept);
/// assert_eq!(kept.0.len(), 1);
/// ```
pub fn syntax_error_rows(
    table: &NodeTable,
    file: FileId,
    words: &mut FileWords,
    out: &mut dyn Rows<SyntaxErrorRow>,
) {
    for (number, diagnostic) in listed(&table.0.diagnostics).enumerate() {
        let message = std::str::from_utf8(diagnostic.message.0).unwrap_or_default();
        out.write(SyntaxErrorRow {
            id: diagnostic_id(
                file,
                ChildOrdinal(u32::try_from(number).unwrap_or(u32::MAX)),
            ),
            message: symbol(words, NameText(message)),
            phase: symbol(words, phase_name(diagnostic.phase)),
            impact: symbol(words, impact_name(diagnostic.impact)),
            code: symbol(words, NameText(diagnostic.code.0)),
            file,
            span: Span {
                start: diagnostic.start,
                end: diagnostic.end,
            },
        });
    }
}
