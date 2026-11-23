//! The cardinality of the entry that the existing driver comparison uses.
//! Distinct values of one fact column have disjoint postings.
use super::{E, File, Native, Run, Ty, ViewSpec, lit_pick_for};
use ryo::languages::{KindBits, node_table};

pub(super) enum Entry {
    Unknown,
    Kinds(Box<[u16]>),
    Literal {
        fact: u16,
        column: usize,
        words: Box<[u32]>,
    },
}

impl Entry {
    pub(super) fn bind(
        run: &mut Run,
        view: &ViewSpec,
        kinds: Option<&[u32]>,
        conjuncts: &[E],
        side: usize,
    ) -> Self {
        if let Some(kinds) = kinds {
            return Self::Kinds(
                run.kind_sym
                    .iter()
                    .enumerate()
                    .filter_map(|(code, sym)| kinds.contains(sym).then_some(code as u16))
                    .collect(),
            );
        }
        let Native::Fact(fact) = view.base else {
            return Self::Unknown;
        };
        let name = run.fact_rels[usize::from(fact)].0.clone();
        let Some((_, _, types)) = run.catalog(&name) else {
            return Self::Unknown;
        };
        let symbol = |column: usize| types.get(column) == Some(&Ty::Symbol);
        let pick = lit_pick_for(&view.filter, 0, &[], symbol)
            .or_else(|| lit_pick_for(conjuncts, side, &view.map, symbol));
        if let Some((column, mut words)) = pick {
            words.sort_unstable();
            words.dedup();
            Self::Literal {
                fact,
                column,
                words: words.into_boxed_slice(),
            }
        } else {
            Self::Unknown
        }
    }

    pub(super) fn len(&self, f: &File<'_>) -> Option<usize> {
        match self {
            Self::Unknown => None,
            Self::Kinds(codes) => Some(
                codes
                    .iter()
                    .map(|c| node_table::ids_of_kind(f.table, KindBits(*c)).len())
                    .sum(),
            ),
            Self::Literal {
                fact,
                column,
                words,
            } => {
                f.facts.get(usize::from(*fact))?.as_ref()?;
                let index = f.fact_list(*fact, *column);
                Some(
                    words
                        .iter()
                        .filter_map(|w| index.get(&u64::from(*w)))
                        .map(<[u32]>::len)
                        .sum(),
                )
            }
        }
    }
}
