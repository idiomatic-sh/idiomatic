//! The routes row's walk side: what the walk's fillers write for a file, rendered the way the frozen rows hold it, the canonical
//! numbering of the nodes of a row, and the coverage of the spelled kinds by the fixtures. The old reader these were once compared with is
//! gone: the rows it wrote are frozen (`frozen.rs`), and the walk is held to them.

use crate::inbound::use_cases::parse::collect::Collect;
use super::reading::written;
use super::samples::routes_samples;
use crate::domain::facts::service::text;
use crate::domain::facts::types::{CellWord, FactAnswer, FactSource, FileWords, FilledColumns, Wanted};
use crate::domain::files::types::SourcePath;
use crate::domain::node_table::service::{kind_codes, span_of};
use crate::domain::node_table::types::NodeTable;
use crate::domain::symbols::service::intern_derived;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest, ParseEngine, ParseRequest};
use crate::inbound::use_cases::walk::types::WalkEngine;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::service::{all, kinds};
use ryo_relations::domain::relations::types::{CellKind, FileId, Position, Relation};
use ryo_relations::domain::terms::service::{term_text, unknown_reason};
use ryo_relations::domain::terms::types::{NameText, Symbol};
use std::path::Path;

/// A node as both routes can name it: where it is and what it is. The two routes number a file's nodes in their own ways, so a node
/// cell is compared as this key, numbered by first sight across both routes of one source. An embedded segment's nodes are keyed by
/// the file offsets they cover (both routes give the same), never by the order the segment was embedded in.
///
/// A column that holds a CLASS REPRESENTATIVE ("an opaque real syntax representative in the same file": `ExpressionShape.shape`,
/// `ConstantKey.key`) names whichever member of its class a route met first, and the routes meet embedded segments in different
/// orders (the old writer embeds some while the table is written and some in a later pass). Such a cell is compared as the least key
/// among the members of its class (the rows that carry it), which does not depend on any numbering.
#[derive(Default)]
pub(super) struct Canon;

/// The column that holds a class representative in the rows of `relation`, the first column holding the member.
const fn representative_column(relation: Relation) -> Option<usize> {
    match relation {
        Relation::ExpressionShape | Relation::ConstantKey => Some(1),
        _ => None,
    }
}

impl Canon {
    /// A numbering that names a node by a hash of its key, so that the name does not depend on the order the rows are met in:
    /// what a frozen record needs.
    pub(super) fn hashed() -> Self {
        Self
    }

    pub(super) fn id(&mut self, key: (u32, u32, u16)) -> u64 {
        u64::from(super::frozen::key_hash(key))
    }

    /// Replaces each node cell of `collected`'s rows of `relation` by the id of the key `key_of` gives its place; a class
    /// representative by the id of the least key among its class.
    pub(super) fn apply(&mut self, collected: &mut Collect, relation: Relation, key_of: &dyn Fn(u32) -> (u32, u32, u16)) {
        let kinds = kinds(relation);
        let node = |bits: u64| u32::try_from(bits & 0xffff_ffff).unwrap_or(u32::MAX);
        let representative = representative_column(relation).filter(|at| *at < kinds.len() && kinds[*at] == CellKind::Node);
        let mut least: std::collections::HashMap<u32, (u32, u32, u16)> = std::collections::HashMap::new();
        if let Some(at) = representative {
            for row in collected.words.chunks(kinds.len().max(1)) {
                let key = key_of(node(row[0]));
                least.entry(node(row[at])).and_modify(|held| *held = (*held).min(key)).or_insert(key);
            }
        }
        for row in collected.words.chunks_mut(kinds.len().max(1)) {
            for (column, (kind, bits)) in kinds.iter().zip(row).enumerate() {
                if *kind == CellKind::Node {
                    let key = if representative == Some(column) { least.get(&node(*bits)).copied().unwrap_or_else(|| key_of(node(*bits))) } else { key_of(node(*bits)) };
                    *bits = self.id(key);
                }
            }
        }
    }
}
/// What the walk's filler of a relation gave for a file.
pub(super) enum Walked {
    /// A relation the node table holds, which no filler writes.
    Skipped,
    /// The language does not have the relation, or the node table holds it.
    Refused,
    /// The rows, each in full, and the causes for which they are read in part.
    Rows(super::reading::Reading),
}

/// What the filler of `relation` writes for the walked file, its node cells named by `canon`.
pub(super) fn walked(
    relation: Relation,
    table: &NodeTable,
    facts: &mut dyn FactSource,
    words: &mut FileWords,
    (path, source): (SourcePath<'_>, ByteSlice<'_>),
    unknowns: &'static [ryo_relations::domain::terms::types::UnknownName],
    canon: &mut Canon,
) -> Walked {
    let mut columns = FilledColumns::default();
    let answer = facts.fill(relation, Wanted::Whole, words, &mut columns);
    let (mut symbols, mut collected) = (Symbols::default(), Collect::default());
    match answer {
        FactAnswer::Refused(AbsenceCause::NoSuchConstruct) => return Walked::Skipped,
        FactAnswer::Refused(_) => return Walked::Refused,
        FactAnswer::Partial(partial) => collected.partials.push((relation, partial.cause.0.to_string(), partial.missing)),
        FactAnswer::Answered(_) => {}
    }
    let rows = columns.0.first().map_or(0, |column| column.0.len());
    for row in 0..rows {
        collected.relations.push(relation);
        for (kind, column) in kinds(relation).iter().zip(&columns.0) {
            let bits = column.0[row].0;
            collected.words.push(if *kind == CellKind::Symbol { symbol_bits(words, &mut symbols, source, bits) } else { bits });
        }
    }
    let (kind_codes, spans) = (kind_codes(table), table);
    canon.apply(&mut collected, relation, &|at| {
        if at as usize >= kind_codes.len() {
            return (u32::MAX, u32::MAX, u16::MAX);
        }
        let span = span_of(spans, Position(at));
        (span.start.0, span.end.0, kind_codes.get(at as usize).copied().unwrap_or(u16::MAX))
    });
    Walked::Rows(written(collected, symbols, (&[], source.0), (path, relation), unknowns))
}
/// A text cell of a fill as the old route's symbols hold it: the word's text interned; a marker of the catalog (an unknown
/// reason, no symbol) is kept as it is.
pub(super) fn symbol_bits(words: &FileWords, symbols: &mut Symbols, source: ByteSlice<'_>, bits: u64) -> u64 {
    let symbol = Symbol(u32::try_from(bits).unwrap_or(u32::MAX));
    if bits >= u64::from(u32::MAX) || term_text(symbol).is_some() || unknown_reason(symbol).is_some() {
        return bits;
    }
    let NameText(spelled) = text(words, CellWord(bits));
    u64::from(intern_derived(symbols, source, NameText(spelled)).0)
}
/// Names the node cells of a row by the key of their place.
pub(super) fn canon_nodes(canon: &mut Canon, relation: Relation, row: &mut [u64], key: &dyn Fn(u32) -> (u32, u32, u16)) {
    for (kind, bits) in kinds(relation).iter().zip(row) {
        if *kind == CellKind::Node {
            *bits = canon.id(key(u32::try_from(*bits & 0xffff_ffff).unwrap_or(u32::MAX)));
        }
    }
}

/// Which kinds a language spells and which of them its fixtures show with text. A kind a provider declares spelled (`spells`) but that
/// no fixture ever reads with a non-empty word on a route is a kind whose text rule nothing checks: the leaf rule once starved every
/// Rust token because no fixture held a spelled identifier where it mattered, and the comparison read zero.
#[derive(Debug, Default)]
pub struct Coverage {
    /// The kinds `spells` says yes to, by name.
    pub spelled: std::collections::BTreeSet<String>,
    /// The kinds that occur at all in the fixtures (any position, text or none).
    pub occurring: std::collections::BTreeSet<String>,
    /// The kinds seen with a non-empty word through the numbered visit; empty, and `numbered` false, for a language with none.
    pub text_walk: std::collections::BTreeSet<String>,
    /// Whether the language has a numbered visit (the walk route's words).
    pub numbered: bool,
}

impl Coverage {
    /// The spelled kinds not seen with text on every route the language has: the uncovered ones.
    #[must_use]
    pub fn uncovered(&self) -> Vec<&str> {
        self.spelled
            .iter()
            .filter(|kind| !self.text_walk.contains(*kind))
            .map(String::as_str)
            .collect()
    }

    /// The spelled kinds that occur in no fixture at all.
    #[must_use]
    pub fn never_occurring(&self) -> Vec<&str> {
        self.spelled.iter().filter(|kind| !self.occurring.contains(*kind)).map(String::as_str).collect()
    }
}

/// The coverage of a language's spelled kinds over the samples of `fixtures`, on the numbered visit.
///
/// # Panics
///
/// When a folder of the fixtures cannot be read.
#[must_use]
pub fn coverage(old: &dyn ParseEngine, walk: &dyn WalkEngine, fixtures: &Path) -> Coverage {
    let names = walk.vocabulary().kinds;
    let name_of = |code: u16| names.0.get(usize::from(code)).map_or_else(|| format!("#{code}"), |name| name.0.to_owned());
    let mut found = Coverage::default();
    for code in 0..names.0.len() {
        let code = u16::try_from(code).unwrap_or(u16::MAX);
        if walk.spells(ryo_relations::domain::relations::types::KindBits(code)) {
            found.spelled.insert(name_of(code));
        }
    }
    let numbered = Some(old.file_relations().numbered());
    found.numbered = true;
    for sample in routes_samples(fixtures) {
        let request =
            ParseRequest { path: sample.path, file: FileId(0), bytes: sample.bytes, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
        if let Some(numbered) = &numbered {
            let mut symbols = Symbols::default();
            let mut reader = numbered.open();
            let _ = reader.visit(request, all(), &[], None, &mut symbols, &mut |file, _| {
                for (at, code) in kind_codes(file.tree).iter().enumerate() {
                    let name = name_of(*code);
                    if file.words.get(at).is_some_and(|word| *word != ryo_relations::domain::relations::constants::NO_SYMBOL) {
                        found.text_walk.insert(name.clone());
                    }
                    found.occurring.insert(name);
                }
            });
        }
    }
    found
}

/// The reviewed list of spelled kinds no source can show with text, from the language's `unreachable_kinds.tsv` (one line each:
/// the kind, a tab, the reason). The check fails on a spelled kind that is neither covered nor listed here, on a listed kind that is
/// covered after all or not spelled at all (a stale entry), and on an entry without a reason.
pub const UNREACHABLE_KINDS: &str = "unreachable_kinds.tsv";

/// What is wrong with the coverage of a language given its exemption list: empty when every spelled kind is covered or listed.
#[must_use]
pub fn coverage_failures(covered: &Coverage, exemptions: &str) -> Vec<String> {
    let mut listed = std::collections::BTreeMap::new();
    let mut found = Vec::new();
    for line in exemptions.lines().filter(|line| !line.trim().is_empty() && !line.starts_with('#')) {
        match line.split_once('\t') {
            Some((kind, reason)) if !reason.trim().is_empty() => {
                listed.insert(kind.trim().to_owned(), reason.trim().to_owned());
            }
            _ => found.push(format!("unreachable kinds: `{line}` has no reason")),
        }
    }
    let uncovered = covered.uncovered();
    for kind in &uncovered {
        if !listed.contains_key(*kind) {
            found.push(format!("spelled kind `{kind}` is never seen with text in the fixtures and is not listed in {UNREACHABLE_KINDS}"));
        }
    }
    for kind in listed.keys() {
        if !covered.spelled.contains(kind) {
            found.push(format!("{UNREACHABLE_KINDS}: `{kind}` is not a spelled kind of the language"));
        } else if !uncovered.contains(&kind.as_str()) {
            found.push(format!("{UNREACHABLE_KINDS}: `{kind}` is covered by the fixtures: remove its entry"));
        }
    }
    found
}

#[cfg(test)]
mod coverage_tests {
    use super::{Coverage, coverage_failures};

    fn covered(spelled: &[&str], text: &[&str]) -> Coverage {
        let set = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect();
        Coverage { spelled: set(spelled), occurring: set(text), text_walk: set(text), numbered: true }
    }

    #[test]
    fn a_spelled_kind_without_text_must_be_listed_with_a_reason() {
        let found = covered(&["a", "b"], &["a"]);
        assert_eq!(coverage_failures(&found, "").len(), 1, "b is uncovered and not listed");
        assert!(coverage_failures(&found, "b\tno source writes it").is_empty());
        assert_eq!(coverage_failures(&found, "b\t").len(), 2, "no reason: the entry is refused and b is still unlisted");
    }

    #[test]
    fn a_stale_entry_is_refused() {
        let found = covered(&["a", "b"], &["a", "b"]);
        assert_eq!(coverage_failures(&found, "b\tonce unreachable").len(), 1, "b is covered");
        assert_eq!(coverage_failures(&found, "c\tnot a kind").len(), 1, "c is not spelled");
    }
}

#[cfg(test)]
mod canonical_tests {
    use super::{Canon, Collect, Relation};

    /// The rows of an `ExpressionShape`: expression, representative, file, span (the last two as they come).
    fn rows(of: &[(u64, u64)]) -> Collect {
        let mut collected = Collect::default();
        for (expression, representative) in of {
            collected.relations.push(Relation::ExpressionShape);
            collected.words.extend([*expression, *representative, 0, 0]);
        }
        collected
    }

    /// Two routes number the same three segments (keys by file offsets) in different orders and pick their own member as the
    /// representative of the one class: after canonicalising, the rows are the same.
    #[test]
    fn two_numberings_of_the_same_embedded_segments_give_the_same_rows() {
        // Positions 10, 11, 12 are the segments at offsets 100, 200, 300 on one route; 12, 10, 11 on the other.
        let key_a = |at: u32| match at { 10 => (100, 110, 7), 11 => (200, 210, 7), _ => (300, 310, 7) };
        let key_b = |at: u32| match at { 12 => (100, 110, 7), 10 => (200, 210, 7), _ => (300, 310, 7) };
        let mut canon = Canon::default();
        let mut a = rows(&[(10, 11), (11, 11), (12, 11)]);
        canon.apply(&mut a, Relation::ExpressionShape, &key_a);
        let mut b = rows(&[(12, 10), (10, 10), (11, 10)]);
        canon.apply(&mut b, Relation::ExpressionShape, &key_b);
        let mut sorted = |collected: &Collect| {
            let mut rows: Vec<Vec<u64>> = collected.words.chunks(4).map(<[u64]>::to_vec).collect();
            rows.sort();
            rows
        };
        assert_eq!(sorted(&a), sorted(&b), "the representative is the least member whichever route met it first");
    }

    /// Other relations' node cells are still named by their own key, and two classes stay two classes.
    #[test]
    fn two_classes_stay_apart() {
        let key = |at: u32| (at * 10, at * 10 + 5, 7);
        let mut canon = Canon::default();
        let mut collected = rows(&[(1, 1), (2, 1), (3, 3)]);
        canon.apply(&mut collected, Relation::ExpressionShape, &key);
        let shapes: Vec<u64> = collected.words.chunks(4).map(|row| row[1]).collect();
        assert_eq!(shapes[0], shapes[1]);
        assert_ne!(shapes[0], shapes[2]);
    }
}

