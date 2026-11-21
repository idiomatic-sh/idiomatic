//! The cases of the words a load gives the names of a language's kinds and fields: each kind and each field has the word of
//! its name, read by its code; code 0 of the fields, no field, is the empty text's word; and a name that is a word already,
//! a constant text of the pack or a fixed name of the language, keeps that one word.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowered::types::LoweredPack;
use crate::domain::lowering::types::{LoweredRules, SelectedRules};
use crate::domain::programs::service::{field_word, kind_word};
use crate::domain::steps::types::ConstantText;
use crate::inbound::use_cases::load::errors::LoadError;
use crate::inbound::use_cases::load::fixtures::kinds;
use crate::inbound::use_cases::load::service::{named_words, sequenced, worded};
use ryo_language::domain::facts::service::{file_words, fixed, text};
use ryo_language::domain::facts::types::{CellWord, FileWords};
use ryo_language::domain::fixed_names::types::FixedWord;
use ryo_language::domain::node_table::types::FieldCode;
use ryo_relations::domain::relations::types::KindBits;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::descriptions::types::{FieldWord, KindVocabulary, KindWord};
use std::borrow::Cow;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The word of each kind code of a loaded rule set whose pack holds one constant text, a kind's name.
    Kinds,
    /// The word of each field code of the same rule set, and of a code past the fields.
    Fields,
    /// The words of the kinds of a language one of whose kinds is named as its fixed name.
    KindNamedAsFixed,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = LoadError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kinds_cell_is_the_word_of_its_name_by_its_code"),
        word: Word::Succeeds,
        input: || Scenario::Kinds,
        expect: Expect::Value(Cow::Borrowed("2 `blank`; 1 `sentence`; 3 `word`; none")),
    },
    Case {
        name: CaseName(
            "a_fields_cell_is_the_word_of_its_name_and_no_field_is_the_empty_texts_word",
        ),
        word: Word::Succeeds,
        input: || Scenario::Fields,
        expect: Expect::Value(Cow::Borrowed("0 ``; 4 `label`; none")),
    },
    Case {
        name: CaseName("a_kind_named_as_a_fixed_name_has_that_names_one_word"),
        word: Word::Succeeds,
        input: || Scenario::KindNamedAsFixed,
        expect: Expect::Value(Cow::Borrowed(
            "the fixed name is word 1; the kinds are words [2, 3, 1]",
        )),
    },
];

/// A language of three kinds and, after no field, one field, that states no fixed name.
fn names() -> KindVocabulary {
    KindVocabulary {
        kinds: ["blank", "sentence", "word"]
            .map(|kind| KindWord(kind.into()))
            .into(),
        fields: ["", "label"].map(|field| FieldWord(field.into())).into(),
        fixed: Vec::new(),
        ..kinds()
    }
}

/// A cell said: its word and the text the run's words hold for it; `none` for a code the vocabulary states nothing at.
fn said(words: &FileWords, cell: Option<CellWord>) -> String {
    cell.map_or_else(
        || "none".to_owned(),
        |cell| format!("{} `{}`", cell.0, text(words, cell).0),
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    // A pack of no rule whose one constant text is the name of the second kind.
    let lower = |_: SelectedRules<'_>| LoweredRules {
        pack: LoweredPack {
            texts: vec![ConstantText("sentence".into())],
            ..LoweredPack::default()
        },
        refused: Vec::new(),
    };
    let loaded = sequenced(SelectedRules(&[]), None, &names(), &lower)?;
    let words = file_words(&loaded.programs.0.words);
    Ok(Cow::Owned(match scenario {
        Scenario::Kinds => {
            let each = (0..4).map(|code| said(&words, kind_word(&loaded.programs, KindBits(code))));
            each.collect::<Vec<_>>().join("; ")
        }
        Scenario::Fields => {
            let each =
                (0..3).map(|code| said(&words, field_word(&loaded.programs, FieldCode(code))));
            each.collect::<Vec<_>>().join("; ")
        }
        Scenario::KindNamedAsFixed => {
            let stated = [NameText("word")];
            let kinds = ["blank", "sentence", "word"].map(NameText);
            let base = worded(&stated, &kinds)?;
            let named = named_words(&base, &names());
            let held = file_words(&base);
            format!(
                "the fixed name is word {}; the kinds are words {:?}",
                fixed(&held, FixedWord(0)).0,
                named.kinds.iter().map(|word| word.0).collect::<Vec<_>>()
            )
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
