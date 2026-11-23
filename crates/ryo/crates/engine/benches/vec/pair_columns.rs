//! Compile one union of demanded columns per relation for every possible path
//! in the retained program. Each provider writes at most one table per file.
use super::{CellTest, Domain, Language, Program, Source};
pub(super) fn compile(p: &mut Program, language: &Language) {
    let mut columns = vec![1u64; p.facts.len()]; // Column zero also tracks the row count.
    let mut read = |slot: usize, col: usize| {
        if col < 64 {
            columns[slot] |= 1 << col;
        } else {
            columns[slot] = u64::MAX;
        }
    };
    for (slot, f) in p.facts.iter().enumerate() {
        if let Some(k) = f.key {
            read(slot, k);
        }
    }
    for d in &p.domains {
        match d {
            Domain::Fact(s, Some((c, _)))
            | Domain::OtherValues(s, c, _)
            | Domain::Number(s, c, ..)
            | Domain::Flag(s, c, _)
            | Domain::Matching(s, c, _)
            | Domain::Unmatched(s, c, _)
            | Domain::CapturedSuffix(s, c, ..) => read(*s, *c),
            Domain::RelatedIds(s, c, filter) => {
                read(*s, *c);
                if let Some((c, _)) = filter {
                    read(*s, *c);
                }
            }
            Domain::SameRow(s, tests) => {
                for test in tests {
                    let c = match test {
                        CellTest::Numeric(c, ..) | CellTest::Symbols(c, _) => *c,
                    };
                    read(*s, c);
                }
            }
            Domain::Member(s, c, p, f) => {
                read(*s, *c);
                read(*p, *f);
            }
            Domain::Any
            | Domain::Empty
            | Domain::HasParent
            | Domain::CountUnder(..)
            | Domain::Kind(_)
            | Domain::Field(_)
            | Domain::Text(_)
            | Domain::TextMatching(..)
            | Domain::SpanMatching(..)
            | Domain::Fact(_, None)
            | Domain::When(..)
            | Domain::Except(..)
            | Domain::ProvenExcept(..)
            | Domain::Both(..)
            | Domain::Either(..)
            | Domain::Parents(_)
            | Domain::Children(_)
            | Domain::Ancestors(_)
            | Domain::Descendants(_) => {}
        }
    }
    for (fact, mask) in p.facts.iter_mut().zip(columns) {
        if let Source::Catalog(rel) = fact.source
            && let Some(key) = fact.key
            && fact.columns.len() <= 64
            && (language.projection)(fact.name)
        {
            fact.source = Source::Columns(rel, key, mask);
        }
    }
}
