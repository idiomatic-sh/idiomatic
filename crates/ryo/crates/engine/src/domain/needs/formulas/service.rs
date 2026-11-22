//! The services of the `formulas` part: a site's formula compiled from the code's records into nodes, once a worker at
//! the site's first decision. A record met again (a part many formulas share) is compiled once; another site is named,
//! never copied in.

use super::types::{AskRank, FormulaNode};
use crate::domain::code::records::service::need_at;
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::types::{NeedSite, ProgramCode, RelationNumber, WordSlot};
use ryo_relations::domain::catalog::service::spelled;
use ryo_relations::domain::catalog::types::Spelling;
use ryo_relations::domain::relations::service::columns;
use crate::domain::needs::masks::service::numbered;
use crate::domain::needs::masks::types::{
    MustWord, NeedMasks, NeedTerm, RecordCost, Stretch, TermNumber,
};
use crate::domain::steps::types::MadeTableId;
use ryo_relations::domain::relations::types::Count;

/// A root being compiled.
const COMPILING: u32 = u32::MAX;

/// The slot `ahead` slots after a place among the needs.
fn slot(code: &ProgramCode, at: WordSlot, ahead: WordSlot) -> Option<WordSlot> {
    let place = at.0.checked_add(ahead.0)?;
    code.needs
        .get(usize::try_from(place).unwrap_or(usize::MAX))
        .copied()
}

/// The most words of a node's musts: past it the words of the lowest places are kept, a part of what it needs.
const MOST_WORDS: usize = 8;

/// The musts of a node.
fn musts_of(masks: &NeedMasks, node: u32) -> &[MustWord] {
    let formulas = &masks.formulas;
    let stretch = formulas
        .node_musts
        .get(usize::try_from(node).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or_default();
    formulas
        .musts
        .get(stretch.start.0..stretch.start.0.saturating_add(stretch.count.0))
        .unwrap_or_default()
}

/// The words of one bit.
fn bit_words(bit: u32) -> Vec<MustWord> {
    let place = usize::try_from(bit).unwrap_or(usize::MAX);
    vec![MustWord {
        word: Count(place / 64),
        bits: 1_u64 << (place % 64),
    }]
}

/// What every one of several nodes needs: the words of all, joined.
fn union(masks: &NeedMasks, parts: &[u32]) -> Vec<MustWord> {
    let mut all: Vec<MustWord> = Vec::new();
    for part in parts {
        for word in musts_of(masks, *part) {
            match all.iter_mut().find(|held| held.word == word.word) {
                Some(held) => held.bits |= word.bits,
                None => all.push(*word),
            }
        }
    }
    all.sort_unstable_by_key(|word| word.word);
    all.truncate(MOST_WORDS);
    all
}

/// What some one of several nodes needs, whichever it is: the bits all of them need.
fn intersection(masks: &NeedMasks, parts: &[u32]) -> Vec<MustWord> {
    let Some((first, rest)) = parts.split_first() else {
        return Vec::new();
    };
    let mut common: Vec<MustWord> = musts_of(masks, *first).to_vec();
    for part in rest {
        let theirs = musts_of(masks, *part);
        for word in &mut common {
            word.bits &= theirs
                .iter()
                .find(|held| held.word == word.word)
                .map_or(0, |held| held.bits);
        }
        common.retain(|word| word.bits != 0);
    }
    common
}

/// A node added, with what asking it may cost and the free terms it cannot hold without; answers its number.
fn pushed(masks: &mut NeedMasks, node: FormulaNode, rank: AskRank, musts: &[MustWord]) -> u32 {
    let formulas = &mut masks.formulas;
    formulas.node_musts.push(Stretch {
        start: Count(formulas.musts.len()),
        count: Count(musts.len()),
    });
    formulas.musts.extend_from_slice(musts);
    formulas.node_clauses.push(Stretch::default());
    let informs = match node {
        // A term that costs a fill is read by the pass over the free terms once the file holds its relation whole
        // (`executing::sources`): what a reader already filled costs nothing to ask.
        FormulaNode::Term(_) => true,
        FormulaNode::Table(_) => false,
        FormulaNode::All(parts) | FormulaNode::Any(parts) => formulas
            .parts
            .get(parts.start.0..parts.start.0.saturating_add(parts.count.0))
            .unwrap_or_default()
            .iter()
            .any(|part| {
                formulas
                    .informs
                    .get(usize::try_from(*part).unwrap_or(usize::MAX))
                    .copied()
                    .unwrap_or(true)
            }),
        FormulaNode::Always
        | FormulaNode::Never
        | FormulaNode::Free(_)
        | FormulaNode::Site(_)
        | FormulaNode::TableNeed(_)
        | FormulaNode::AllBits(_)
        | FormulaNode::AnyBits(_) => true,
    };
    formulas.informs.push(informs);
    let of = |part: &u32| {
        formulas
            .deciding
            .get(usize::try_from(*part).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or((rank, rank))
    };
    let deciding = match node {
        FormulaNode::All(parts) | FormulaNode::Any(parts) => {
            let each = formulas
                .parts
                .get(parts.start.0..parts.start.0.saturating_add(parts.count.0))
                .unwrap_or_default();
            let empty = each.iter().map(of).map(|(empty, _)| empty);
            let holds = each.iter().map(of).map(|(_, holds)| holds);
            if matches!(node, FormulaNode::All(_)) {
                (empty.min().unwrap_or(rank), holds.max().unwrap_or(rank))
            } else {
                (empty.max().unwrap_or(rank), holds.min().unwrap_or(rank))
            }
        }
        FormulaNode::Always
        | FormulaNode::Never
        | FormulaNode::Free(_)
        | FormulaNode::AllBits(_)
        | FormulaNode::AnyBits(_) => (AskRank::Known, AskRank::Known),
        FormulaNode::Term(_)
        | FormulaNode::Table(_)
        | FormulaNode::Site(_)
        | FormulaNode::TableNeed(_) => (rank, rank),
    };
    formulas.deciding.push(deciding);
    formulas.nodes.push(node);
    formulas.ranks.push(rank);
    u32::try_from(formulas.nodes.len().saturating_sub(1)).unwrap_or(u32::MAX)
}

/// The most words of a node's clause: an Any whose alternatives' terms spread wider keeps none.
const MOST_CLAUSE_WORDS: usize = 16;

/// The clause of a node.
fn clause_of(masks: &NeedMasks, node: u32) -> &[MustWord] {
    let formulas = &masks.formulas;
    let stretch = formulas
        .node_clauses
        .get(usize::try_from(node).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or_default();
    formulas
        .musts
        .get(stretch.start.0..stretch.start.0.saturating_add(stretch.count.0))
        .unwrap_or_default()
}

/// A clause a node cannot hold without, the narrowest at hand: its own clause, else one of its musts. None: nothing
/// is known it needs.
fn best_clause(masks: &NeedMasks, node: u32) -> Option<Vec<MustWord>> {
    let own = clause_of(masks, node);
    if !own.is_empty() {
        return Some(own.to_vec());
    }
    let first = musts_of(masks, node).first()?;
    Some(vec![MustWord {
        word: first.word,
        bits: first.bits & first.bits.wrapping_neg(),
    }])
}

/// Sets a node's clause.
fn clause_set(masks: &mut NeedMasks, node: u32, clause: &[MustWord]) {
    if clause.is_empty() {
        return;
    }
    let formulas = &mut masks.formulas;
    let stretch = Stretch {
        start: Count(formulas.musts.len()),
        count: Count(clause.len()),
    };
    formulas.musts.extend_from_slice(clause);
    if let Some(held) = formulas
        .node_clauses
        .get_mut(usize::try_from(node).unwrap_or(usize::MAX))
    {
        *held = stretch;
    }
}

/// The clause of an Any: some part must hold, so the file must hold some term of the narrowest clause of each part.
/// None when a part has none, or when they spread past the width.
fn any_clause(masks: &NeedMasks, parts: &[u32]) -> Vec<MustWord> {
    let mut all: Vec<MustWord> = Vec::new();
    for part in parts {
        let Some(clause) = best_clause(masks, *part) else {
            return Vec::new();
        };
        for word in clause {
            match all.iter_mut().find(|held| held.word == word.word) {
                Some(held) => held.bits |= word.bits,
                None => all.push(word),
            }
        }
        if all.len() > MOST_CLAUSE_WORDS {
            return Vec::new();
        }
    }
    all.sort_unstable_by_key(|word| word.word);
    all
}

/// The clause of an All: the narrowest clause of a part.
fn all_clause(masks: &NeedMasks, parts: &[u32]) -> Vec<MustWord> {
    let fewest = parts
        .iter()
        .map(|part| clause_of(masks, *part))
        .filter(|clause| !clause.is_empty())
        .min_by_key(|clause| clause.iter().map(|word| word.bits.count_ones()).sum::<u32>());
    fewest.map_or_else(Vec::new, <[MustWord]>::to_vec)
}

/// The musts of a node, by number, as the file's free bits must hold them: a stretch of the formulas' musts.
pub(crate) fn node_musts(masks: &NeedMasks, node: u32) -> (Stretch, Stretch) {
    let clause = masks
        .formulas
        .node_clauses
        .get(usize::try_from(node).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or_default();
    let musts = masks
        .formulas
        .node_musts
        .get(usize::try_from(node).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or_default();
    (musts, clause)
}

/// The least it may cost to find a node empty, and to find it holds.
fn deciding_of(masks: &NeedMasks, node: u32) -> (AskRank, AskRank) {
    masks
        .formulas
        .deciding
        .get(usize::try_from(node).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or((AskRank::Presence, AskRank::Presence))
}

/// Sets what it may cost to decide a node that stands for another formula: the formula's.
fn deciding_set(masks: &mut NeedMasks, node: u32, root: Option<u32>) {
    let Some(root) = root else {
        return;
    };
    let held = deciding_of(masks, root);
    if let Some(slot) = masks
        .formulas
        .deciding
        .get_mut(usize::try_from(node).unwrap_or(usize::MAX))
    {
        *slot = held;
    }
}

/// What asking a node may cost.
pub(crate) fn rank_of(masks: &NeedMasks, node: u32) -> AskRank {
    masks
        .formulas
        .ranks
        .get(usize::try_from(node).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or(AskRank::Presence)
}

/// The node of a term: a bit of the file's free answers when it costs nothing, else the term by number.
fn term_node(code: &ProgramCode, masks: &mut NeedMasks, term: NeedTerm, at: WordSlot) -> u32 {
    let number = numbered(code, masks, term, at);
    if let Some(node) = masks.formulas.leaves.get(&(0, number.0)).copied() {
        return node;
    }
    let mut node = term_pushed(masks, number);
    // A row of a filled relation held to a constant in a column its item declares spelled cannot exist in a file that
    // spells no token so: the term that asks the file for that token, which costs nothing, goes before it.
    if let NeedTerm::Row {
        relation,
        column,
        constant,
    } = term
        && column != NO_SLOT
        && constant != NO_SLOT
        && let Some(spelling) = spelling_of(code, relation, column)
        && !spelled_off()
    {
        let spelled = numbered(code, masks, NeedTerm::Spelled { constant, spelling }, NO_SLOT);
        let bit = term_pushed(masks, spelled);
        node = joined(masks, vec![bit, node], true);
    }
    let _was = masks.formulas.leaves.insert((0, number.0), node);
    node
}

/// SPIKE: whether the spelled terms are left out (`RYO_X_NO_SPELLED`, read once), to count what they decide.
fn spelled_off() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("RYO_X_NO_SPELLED").is_some())
}

/// How the column at a place of a relation the code reads is spelled, when its item declares it.
fn spelling_of(code: &ProgramCode, relation: RelationNumber, column: WordSlot) -> Option<Spelling> {
    let read = code
        .relations
        .get(usize::try_from(relation.0).unwrap_or(usize::MAX))
        .copied()?;
    let named = columns(read)
        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
        .copied()?;
    spelled(read)
        .iter()
        .find(|(held, _)| *held == named)
        .map(|(_, spelling)| *spelling)
}

/// The node of a term met the first time.
fn term_pushed(masks: &mut NeedMasks, number: TermNumber) -> u32 {
    let place = usize::try_from(number.0).unwrap_or(usize::MAX);
    let cost = masks.term_costs.get(place).copied().unwrap_or_default();
    let free = masks.free_places.get(place).copied().unwrap_or(0);
    match (cost, free.checked_sub(1)) {
        (RecordCost::Free, Some(bit)) => {
            pushed(masks, FormulaNode::Free(bit), AskRank::Free, &bit_words(bit))
        }
        (RecordCost::Presence, _) => pushed(masks, FormulaNode::Term(number), AskRank::Presence, &[]),
        (RecordCost::Unknown | RecordCost::Free | RecordCost::Dear, _) => {
            pushed(masks, FormulaNode::Term(number), AskRank::Row, &[])
        }
    }
}

/// The node of an `All` or an `Any` of the parts at `offsets`: its parts compiled and ordered by what asking them may cost
/// (the record's order among parts of one rank). A part of an `All` that cannot be read is passed over; one of an `Any`
/// holds.
fn parts_node(
    code: &ProgramCode,
    masks: &mut NeedMasks,
    count: WordSlot,
    offsets: WordSlot,
    every: bool,
) -> u32 {
    let mut parts: Vec<u32> = Vec::with_capacity(usize::try_from(count.0).unwrap_or(0));
    for number in 0..count.0 {
        match slot(code, offsets, WordSlot(number)) {
            Some(place) => parts.push(compiled(code, place, masks)),
            None if every => {}
            None => parts.push(pushed(masks, FormulaNode::Always, AskRank::Free, &[])),
        }
    }
    if every {
        joined(masks, parts, true)
    } else {
        factored(masks, parts)
    }
}

/// A part's own parts as an All sees them: its free bits, and its other parts by number.
fn conjuncts(masks: &NeedMasks, part: u32) -> (Vec<MustWord>, Vec<u32>) {
    let (mut words, mut others) = (Vec::new(), Vec::new());
    let mut add = |masks: &NeedMasks, node: u32, words: &mut Vec<MustWord>| {
        let held = masks.formulas.nodes.get(usize::try_from(node).unwrap_or(usize::MAX)).copied();
        let more: Vec<MustWord> = match held {
            Some(FormulaNode::Free(bit)) => bit_words(bit),
            Some(FormulaNode::AllBits(stretch)) => masks
                .formulas
                .bits
                .get(stretch.start.0..stretch.start.0.saturating_add(stretch.count.0))
                .unwrap_or_default()
                .to_vec(),
            Some(_) | None => {
                others.push(node);
                return;
            }
        };
        for word in more {
            match words.iter_mut().find(|held: &&mut MustWord| held.word == word.word) {
                Some(held) => held.bits |= word.bits,
                None => words.push(word),
            }
        }
    };
    match masks.formulas.nodes.get(usize::try_from(part).unwrap_or(usize::MAX)).copied() {
        Some(FormulaNode::All(stretch)) => {
            let inner = masks
                .formulas
                .parts
                .get(stretch.start.0..stretch.start.0.saturating_add(stretch.count.0))
                .unwrap_or_default()
                .to_vec();
            for node in inner {
                add(masks, node, &mut words);
            }
        }
        Some(_) | None => add(masks, part, &mut words),
    }
    (words, others)
}

/// The node of an Any, with what all its alternatives need taken out in front: an Any of (C and R1), (C and R2) is C
/// and an Any of R1, R2, so C is tested once and the alternatives keep what tells them apart. An alternative left with
/// nothing holds whenever C does.
fn factored(masks: &mut NeedMasks, parts: Vec<u32>) -> u32 {
    if parts.len() < 2 {
        return joined(masks, parts, false);
    }
    let each: Vec<(Vec<MustWord>, Vec<u32>)> = parts.iter().map(|part| conjuncts(masks, *part)).collect();
    let Some((first_words, first_others)) = each.first() else {
        return joined(masks, parts, false);
    };
    let mut common_words = first_words.clone();
    let mut common_others = first_others.clone();
    for (words, others) in each.iter().skip(1) {
        for word in &mut common_words {
            word.bits &= words.iter().find(|held| held.word == word.word).map_or(0, |held| held.bits);
        }
        common_words.retain(|word| word.bits != 0);
        common_others.retain(|node| others.contains(node));
    }
    if common_words.is_empty() && common_others.is_empty() {
        return joined(masks, parts, false);
    }
    let mut rests: Vec<u32> = Vec::with_capacity(each.len());
    let mut always = false;
    for (words, others) in each {
        let left_words: Vec<MustWord> = words
            .iter()
            .map(|word| MustWord {
                word: word.word,
                bits: word.bits
                    & !common_words.iter().find(|held| held.word == word.word).map_or(0, |held| held.bits),
            })
            .filter(|word| word.bits != 0)
            .collect();
        let left_others: Vec<u32> = others.into_iter().filter(|node| !common_others.contains(node)).collect();
        if left_words.is_empty() && left_others.is_empty() {
            always = true;
            break;
        }
        rests.push(all_of(masks, &left_words, left_others));
    }
    let mut all = common_others;
    if !common_words.is_empty() {
        all.push(all_of(masks, &common_words, Vec::new()));
    }
    if !always {
        all.push(joined(masks, rests, false));
    }
    joined(masks, all, true)
}

/// The node of an All of free bits and other parts.
fn all_of(masks: &mut NeedMasks, words: &[MustWord], mut others: Vec<u32>) -> u32 {
    if !words.is_empty() {
        let stretch = Stretch {
            start: Count(masks.formulas.bits.len()),
            count: Count(words.len()),
        };
        masks.formulas.bits.extend_from_slice(words);
        let bits = pushed(masks, FormulaNode::AllBits(stretch), AskRank::Free, words);
        others.insert(0, bits);
    }
    joined(masks, others, true)
}

/// The node of an All (`every`) or an Any of compiled parts: the parts that are free terms joined into one node of bit
/// words, the others ordered by what asking them may cost.
fn joined(masks: &mut NeedMasks, parts: Vec<u32>, every: bool) -> u32 {
    // An All of Alls is one All, an Any of Anys one Any: their parts are ordered together, so a dear part of one is
    // not asked before a cheaper part of another.
    let mut flat: Vec<u32> = Vec::with_capacity(parts.len());
    for part in parts {
        match masks.formulas.nodes.get(usize::try_from(part).unwrap_or(usize::MAX)).copied() {
            Some(FormulaNode::All(inner)) if every => flat.extend_from_slice(
                masks
                    .formulas
                    .parts
                    .get(inner.start.0..inner.start.0.saturating_add(inner.count.0))
                    .unwrap_or_default(),
            ),
            Some(FormulaNode::Any(inner)) if !every => flat.extend_from_slice(
                masks
                    .formulas
                    .parts
                    .get(inner.start.0..inner.start.0.saturating_add(inner.count.0))
                    .unwrap_or_default(),
            ),
            Some(_) | None => flat.push(part),
        }
    }
    flat.dedup();
    let mut parts = flat;
    if parts.len() == 1
        && let Some(only) = parts.first()
    {
        return *only;
    }
    // The parts that are free terms become one node of bit words: a word's and-compare in place of a node a term.
    let mut words: Vec<MustWord> = Vec::new();
    parts.retain(|part| {
        let Some(FormulaNode::Free(bit)) = masks
            .formulas
            .nodes
            .get(usize::try_from(*part).unwrap_or(usize::MAX))
            .copied()
        else {
            return true;
        };
        let place = usize::try_from(bit).unwrap_or(usize::MAX);
        let (word, held) = (Count(place / 64), 1_u64 << (place % 64));
        match words.iter_mut().find(|each| each.word == word) {
            Some(each) => each.bits |= held,
            None => words.push(MustWord { word, bits: held }),
        }
        false
    });
    if !words.is_empty() {
        let stretch = Stretch {
            start: Count(masks.formulas.bits.len()),
            count: Count(words.len()),
        };
        masks.formulas.bits.extend_from_slice(&words);
        let one = words.len() == 1 && words.iter().all(|word| word.bits.count_ones() == 1);
        let (node, musts) = if every {
            (FormulaNode::AllBits(stretch), words.clone())
        } else if one {
            (FormulaNode::AnyBits(stretch), words.clone())
        } else {
            (FormulaNode::AnyBits(stretch), Vec::new())
        };
        let bits = pushed(masks, node, AskRank::Free, &musts);
        if !every && !one {
            clause_set(masks, bits, &words);
        }
        if parts.is_empty() {
            return bits;
        }
        parts.insert(0, bits);
    }
    if parts.is_empty() {
        let node = if every {
            FormulaNode::Always
        } else {
            FormulaNode::Never
        };
        return pushed(masks, node, AskRank::Free, &[]);
    }
    // An All looks for a part that is empty, an Any for one that holds: each asks first the parts that may say so
    // for least once the free terms left them open (a part they answer is read by the free pass, and comes last).
    parts.sort_by_key(|part| {
        let (empty, holds) = deciding_of(masks, *part);
        if every { empty } else { holds }
    });
    let rank = parts
        .iter()
        .map(|part| rank_of(masks, *part))
        .max()
        .unwrap_or_default();
    let stretch = Stretch {
        start: Count(masks.formulas.parts.len()),
        count: Count(parts.len()),
    };
    masks.formulas.parts.extend_from_slice(&parts);
    let (node, musts, clause) = if every {
        (FormulaNode::All(stretch), union(masks, &parts), all_clause(masks, &parts))
    } else {
        (FormulaNode::Any(stretch), intersection(masks, &parts), any_clause(masks, &parts))
    };
    let number = pushed(masks, node, rank, &musts);
    clause_set(masks, number, &clause);
    number
}

/// The node of a table's row: its own need and then its row, so that a need the free terms decide is decided with no
/// table asked.
fn table_node(code: &ProgramCode, masks: &mut NeedMasks, table: MadeTableId) -> u32 {
    let row = pushed(masks, FormulaNode::Table(table), AskRank::Table, &[]);
    let Some(need) = code
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
        .map(|held| held.need)
        .filter(|need| usize::try_from(need.0).is_ok_and(|place| place < code.needs.len()))
    else {
        return row;
    };
    let root = site_root(code, need, masks);
    let rank = root.map_or(AskRank::Table, |root| rank_of(masks, root));
    let musts = root.map_or_else(Vec::new, |root| musts_of(masks, root).to_vec());
    let clause = root.map_or_else(Vec::new, |root| clause_of(masks, root).to_vec());
    let own = pushed(masks, FormulaNode::TableNeed(table), rank, &musts);
    clause_set(masks, own, &clause);
    deciding_set(masks, own, root);
    let stretch = Stretch {
        start: Count(masks.formulas.parts.len()),
        count: Count(2),
    };
    masks.formulas.parts.extend_from_slice(&[own, row]);
    let number = pushed(masks, FormulaNode::All(stretch), AskRank::Table, &musts);
    clause_set(masks, number, &clause);
    number
}

/// The node compiled from the record at a place among the needs: compiled once, read again after.
fn compiled(code: &ProgramCode, at: WordSlot, masks: &mut NeedMasks) -> u32 {
    let place = usize::try_from(at.0).unwrap_or(usize::MAX);
    if let Some(node) = masks
        .formulas
        .at_place
        .get(place)
        .copied()
        .and_then(|held| held.checked_sub(1))
    {
        return node;
    }
    let node = match need_at(&code.needs, at) {
        None | Some(NeedRecord::Unknown) => pushed(masks, FormulaNode::Always, AskRank::Free, &[]),
        Some(NeedRecord::Kinds(list)) => term_node(code, masks, NeedTerm::Kinds(list), at),
        Some(NeedRecord::Path {
            comparison,
            regex,
            constant,
        }) => term_node(
            code,
            masks,
            NeedTerm::Path {
                comparison,
                regex,
                constant,
            },
            at,
        ),
        Some(NeedRecord::Bytes { regex, constant }) => {
            term_node(code, masks, NeedTerm::Bytes { regex, constant }, at)
        }
        Some(NeedRecord::RowTest {
            relation,
            column,
            comparison,
            regex,
            constant,
        }) => term_node(
            code,
            masks,
            NeedTerm::RowTest {
                relation,
                column,
                comparison,
                regex,
                constant,
            },
            at,
        ),
        Some(NeedRecord::Row {
            relation,
            column,
            constant,
        }) => term_node(
            code,
            masks,
            NeedTerm::Row {
                relation,
                column,
                constant,
            },
            at,
        ),
        Some(NeedRecord::Site(site)) if masks.formulas.leaves.contains_key(&(1, site.0)) => {
            masks.formulas.leaves.get(&(1, site.0)).copied().unwrap_or(0)
        }
        Some(NeedRecord::Table(table)) if masks.formulas.leaves.contains_key(&(2, table.0)) => {
            masks.formulas.leaves.get(&(2, table.0)).copied().unwrap_or(0)
        }
        Some(NeedRecord::Site(site)) => {
            // A site met while its own formula is compiled says nothing of itself: it is taken to cost a table.
            let root = site_root(code, site, masks);
            let rank = root.map_or(AskRank::Table, |root| rank_of(masks, root));
            let musts = root.map_or_else(Vec::new, |root| musts_of(masks, root).to_vec());
            let clause = root.map_or_else(Vec::new, |root| clause_of(masks, root).to_vec());
            let number = pushed(masks, FormulaNode::Site(site), rank, &musts);
            clause_set(masks, number, &clause);
            deciding_set(masks, number, root);
            // Only a site whose formula was compiled is shared: one met inside itself is not.
            if root.is_some() {
                let _was = masks.formulas.leaves.insert((1, site.0), number);
            }
            number
        }
        Some(NeedRecord::Table(table)) => {
            let number = table_node(code, masks, table);
            let _was = masks.formulas.leaves.insert((2, table.0), number);
            number
        }
        Some(NeedRecord::All {
            parts: count,
            offsets,
        }) => parts_node(code, masks, count, offsets, true),
        Some(NeedRecord::Any {
            parts: count,
            offsets,
        }) => parts_node(code, masks, count, offsets, false),
    };
    let at_place = &mut masks.formulas.at_place;
    if at_place.len() <= place {
        at_place.resize(place.saturating_add(1), 0);
    }
    if let Some(held) = at_place.get_mut(place) {
        *held = node.saturating_add(1);
    }
    node
}

/// The node of a site's formula, compiled at the site's first ask. None while it is being compiled: a formula that
/// names itself.
pub(crate) fn site_root(code: &ProgramCode, site: NeedSite, masks: &mut NeedMasks) -> Option<u32> {
    let place = usize::try_from(site.0).unwrap_or(usize::MAX);
    match masks.formulas.roots.get(place).copied() {
        Some(COMPILING) => return None,
        Some(held) if held > 0 => return Some(held.saturating_sub(1)),
        Some(_) | None => {}
    }
    let roots = &mut masks.formulas.roots;
    if roots.len() <= place {
        roots.resize(place.saturating_add(1), 0);
    }
    if let Some(held) = roots.get_mut(place) {
        *held = COMPILING;
    }
    let node = match slot(code, WordSlot(site.0), WordSlot(0)) {
        Some(formula) => compiled(code, formula, masks),
        None => pushed(masks, FormulaNode::Always, AskRank::Free, &[]),
    };
    if let Some(held) = masks.formulas.roots.get_mut(place) {
        *held = node.saturating_add(1);
    }
    Some(node)
}
