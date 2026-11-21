//! The `keyed_rows` service's cases: for each relation it answers by key, the rows it writes for every key of a source, taken
//! together, are the rows the relation's whole filler writes for the source, and a key no row holds writes none. Every element
//! of the file is asked as a node key, and a word no name of the file has as a name key.

use super::service::{
    binding_read_rows_of, binding_shadow_rows_of, reference_rows_of, resolves_rows_of,
    scope_rows_of,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::binding_facts::service::{binding_read_rows, binding_shadow_rows};
use crate::domain::lexical_facts::fixtures::CONSTRUCTS;
use crate::domain::lexical_facts::service::{reference_rows, resolves_rows, scope_rows};
use crate::fixtures::model_rows::Gathered;
use crate::outbound::syntax::fixtures::model_view::{FixtureView, ModelFixture, viewed};
use ryo::languages::{
    self, CatalogRelation, CellWord, FileId, FileWords, Position, Rows, facts, node_id, node_table,
};

/// The relation asked by key.
#[derive(Clone, Copy, Debug)]
enum Keyed {
    /// `scope` by its own node.
    Scope,
    /// `reference` by the name or attribute expression.
    Reference,
    /// `resolves` by the name expression.
    Resolves,
    /// `binding-read` by the name read.
    Read,
    /// `binding-shadow` by the later binding.
    Shadow,
}

/// What the fills by key of a source wrote against its whole fill.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Held {
    /// The rows the whole fill wrote.
    whole: usize,
    /// The keys asked that wrote a row.
    keys: usize,
    /// The keys asked that wrote none.
    empty: usize,
    /// The rows the whole fill wrote and no key did, then the rows a key wrote and the whole fill did not, as cells.
    differing: Vec<String>,
    /// The rows a key wrote that do not hold the key.
    strays: usize,
}

type Input = Keyed;
type Value = Held;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_rows_of_every_scope_are_the_scopes_of_the_file"),
        word: Word::Succeeds,
        input: || Keyed::Scope,
        expect: Expect::Value(Held {
            whole: 17,
            keys: 17,
            empty: 442,
            differing: Vec::new(),
            strays: 0,
        }),
    },
    Case {
        name: CaseName("the_rows_of_every_expression_are_the_references_of_the_file"),
        word: Word::Succeeds,
        input: || Keyed::Reference,
        expect: Expect::Value(Held {
            whole: 106,
            keys: 106,
            empty: 353,
            differing: Vec::new(),
            strays: 0,
        }),
    },
    Case {
        name: CaseName("the_rows_of_every_name_expression_are_the_resolutions_of_the_file"),
        word: Word::Succeeds,
        input: || Keyed::Resolves,
        expect: Expect::Value(Held {
            whole: 78,
            keys: 78,
            empty: 381,
            differing: Vec::new(),
            strays: 0,
        }),
    },
    Case {
        name: CaseName("the_rows_of_every_name_are_the_reads_of_the_file"),
        word: Word::Succeeds,
        input: || Keyed::Read,
        expect: Expect::Value(Held {
            whole: 109,
            keys: 71,
            empty: 1,
            differing: Vec::new(),
            strays: 0,
        }),
    },
    Case {
        name: CaseName("the_rows_of_every_binding_are_the_shadows_of_the_file"),
        word: Word::Succeeds,
        input: || Keyed::Shadow,
        expect: Expect::Value(Held {
            whole: 11,
            keys: 11,
            empty: 448,
            differing: Vec::new(),
            strays: 0,
        }),
    },
];

/// The rows a fill wrote, as cells, sorted.
fn cells<R: CatalogRelation>(gathered: &Gathered<R>) -> Vec<Vec<u64>> {
    let mut rows: Vec<Vec<u64>> = gathered
        .rows
        .iter()
        .map(|row| row.as_ref().iter().map(|cell| cell.0).collect())
        .collect();
    rows.sort_unstable();
    rows
}

/// The whole fill of `CONSTRUCTS` against its fills by key on the column at `column`: by every element of the file when
/// `names` is no word, by every name the whole fill wrote and by `names`, a word no name has, when it is one.
fn held<R: CatalogRelation>(
    column: usize,
    name: Option<&'static str>,
    whole: impl Fn(&mut FixtureView<'_>, &mut FileWords, &mut dyn Rows<R::Row>),
    of: impl Fn(&mut FixtureView<'_>, &mut FileWords, CellWord, &mut dyn Rows<R::Row>),
) -> Held {
    let mut found = Held::default();
    viewed(
        &mut ModelFixture::default(),
        CONSTRUCTS,
        FileId(0),
        &mut |view| {
            let mut words =
                facts::file_words(&facts::base_words(languages::CheckedNames::default(), &[]));
            let mut all = Gathered::<R> {
                rows: Vec::new(),
                causes: Vec::new(),
            };
            whole(view, &mut words, &mut all);
            let all = cells(&all);
            let mut keys: Vec<u64> = match name {
                Some(absent) => {
                    let mut asked: Vec<u64> = all
                        .iter()
                        .filter_map(|row| row.get(column).copied())
                        .collect();
                    asked.push(facts::intern(&mut words, languages::NameText(absent)).0);
                    asked
                }
                None => (0..u32::try_from(node_table::element_count(view.table).0)
                    .unwrap_or(u32::MAX))
                    .map(|id| node_id(view.file, Position(id)).0)
                    .collect(),
            };
            keys.sort_unstable();
            keys.dedup();
            let mut by_key: Vec<Vec<u64>> = Vec::new();
            for key in keys {
                let mut one = Gathered::<R> {
                    rows: Vec::new(),
                    causes: Vec::new(),
                };
                of(view, &mut words, CellWord(key), &mut one);
                let one = cells(&one);
                found.strays += one
                    .iter()
                    .filter(|row| row.get(column) != Some(&key))
                    .count();
                if one.is_empty() {
                    found.empty += 1;
                } else {
                    found.keys += 1;
                }
                by_key.extend(one);
            }
            by_key.sort_unstable();
            found.whole = all.len();
            found.differing = all
                .iter()
                .filter(|row| !by_key.contains(row))
                .map(|row| format!("- {row:?}"))
                .chain(
                    by_key
                        .iter()
                        .filter(|row| !all.contains(row))
                        .map(|row| format!("+ {row:?}")),
                )
                .collect();
            if found.differing.is_empty() && by_key.len() != all.len() {
                found.differing.push(format!(
                    "{} rows by key against {}",
                    by_key.len(),
                    all.len()
                ));
            }
        },
    );
    found
}

fn subject(keyed: Input) -> Value {
    match keyed {
        Keyed::Scope => held::<languages::Scope>(
            1,
            None,
            |view, words, rows| scope_rows(view, words, rows),
            |view, _words, key, rows| scope_rows_of(view, key, rows),
        ),
        Keyed::Reference => held::<languages::Reference>(
            0,
            None,
            |view, words, rows| reference_rows(view, words, rows),
            |view, words, key, rows| reference_rows_of(view, words, key, rows),
        ),
        Keyed::Resolves => held::<languages::Resolves>(
            0,
            None,
            |view, words, rows| resolves_rows(view, words, rows),
            |view, words, key, rows| resolves_rows_of(view, words, key, rows),
        ),
        Keyed::Read => held::<languages::BindingRead>(
            1,
            Some("a_name_no_read_has"),
            |view, words, rows| binding_read_rows(view, words, rows),
            |view, words, key, rows| binding_read_rows_of(view, words, key, rows),
        ),
        Keyed::Shadow => held::<languages::BindingShadow>(
            0,
            None,
            |view, words, rows| binding_shadow_rows(view, words, rows),
            |view, words, key, rows| binding_shadow_rows_of(view, words, key, rows),
        ),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
