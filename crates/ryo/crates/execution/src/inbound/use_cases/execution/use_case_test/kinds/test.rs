//! The `kind_sets` service's cases: the elements of a named kind are found among that kind, and a file nothing read teaches nothing.
use super::fixtures::{
    Found, Kinds, Observed, a_file_with_no_names_teaches_nothing_about_kinds,
    another_language_does_not_inherit_the_sets_of_the_first, finds_the_elements_of_a_named_kind,
    masked, names_a_kind_by_a_test_on_its_name,
};
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_domain::domain::operating::types::KindNaming;
use ryo_execution_domain::domain::table_model::types::Verdict;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::Test;
use std::borrow::Cow;

type Input = Kinds;
type Value = Observed;
type Failure = OperatingError;
const FOUND: Found = Cow::Borrowed(&[
    ("nodes", Cow::Borrowed(&[0])),
    ("tokens", Cow::Borrowed(&[1, 2, 3])),
    ("one_of", Cow::Borrowed(&[1, 3])),
    ("in_a_list", Cow::Borrowed(&[0, 2])),
    ("not_in_a_list", Cow::Borrowed(&[1, 3])),
    ("a_prefix", Cow::Borrowed(&[1, 3])),
    ("words", Cow::Borrowed(&[1, 3])),
    ("a_word", Cow::Borrowed(&[3])),
    ("none", Cow::Borrowed(&[])),
    ("others", Cow::Borrowed(&[0, 2])),
    ("neither", Cow::Borrowed(&[2])),
    ("a_word_that_is_not_the_file", Cow::Borrowed(&[1, 3])),
]);
const LANGUAGES: Found = Cow::Borrowed(&[
    ("first", Cow::Borrowed(&[1])),
    ("second", Cow::Borrowed(&[1])),
    ("wrong", Cow::Borrowed(&[])),
    ("again", Cow::Borrowed(&[1])),
]);
const MASKED: Observed = Observed::Masked {
    kinds: Cow::Borrowed(&[CellKind::Node, CellKind::Symbol, CellKind::Node]),
    rows: 1,
    nodes: Cow::Borrowed(&[1, 1]),
    names: Cow::Borrowed(&[Cow::Borrowed("word")]),
};
/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("prepared_masks_keep_exact_projected_rows"),
        word: Word::Succeeds,
        input: || Kinds::Mask(Test::Equal, "word"),
        expect: Expect::Value(MASKED),
    },
    Case {
        name: CaseName("a_notequal_mask_of_file_keeps_exact_projected_rows"),
        word: Word::Succeeds,
        input: || Kinds::Mask(Test::NotEqual, "file"),
        expect: Expect::Value(MASKED),
    },
    Case {
        name: CaseName("a_globany_mask_of_wor_keeps_exact_projected_rows"),
        word: Word::Succeeds,
        input: || Kinds::Mask(Test::GlobAny, "wor*"),
        expect: Expect::Value(MASKED),
    },
    Case {
        name: CaseName("a_notglobany_mask_of_fi_keeps_exact_projected_rows"),
        word: Word::Succeeds,
        input: || Kinds::Mask(Test::NotGlobAny, "fi*"),
        expect: Expect::Value(MASKED),
    },
    Case {
        name: CaseName("a_startswith_mask_of_wo_keeps_exact_projected_rows"),
        word: Word::Succeeds,
        input: || Kinds::Mask(Test::StartsWith, "wo"),
        expect: Expect::Value(MASKED),
    },
    Case {
        name: CaseName("a_endswith_mask_of_rd_keeps_exact_projected_rows"),
        word: Word::Succeeds,
        input: || Kinds::Mask(Test::EndsWith, "rd"),
        expect: Expect::Value(MASKED),
    },
    Case {
        name: CaseName("a_matches_mask_of_word_keeps_exact_projected_rows"),
        word: Word::Succeeds,
        input: || Kinds::Mask(Test::Matches, "^word$"),
        expect: Expect::Value(MASKED),
    },
    Case {
        name: CaseName("a_equal_mask_of_absent_keeps_exact_projected_rows"),
        word: Word::Succeeds,
        input: || Kinds::Mask(Test::Equal, "absent"),
        expect: Expect::Value(Observed::Masked {
            kinds: Cow::Borrowed(&[CellKind::Node, CellKind::Symbol, CellKind::Node]),
            rows: 0,
            nodes: Cow::Borrowed(&[]),
            names: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("a_test_on_the_name_of_a_kind_names_a_kind"),
        word: Word::Succeeds,
        input: || Kinds::Naming,
        expect: Expect::Value(Observed::Naming {
            naming: [KindNaming::Names, KindNaming::Silent, KindNaming::Silent],
            verdicts: [
                Verdict::Holds,
                Verdict::Fails,
                Verdict::Holds,
                Verdict::Holds,
            ],
        }),
    },
    Case {
        name: CaseName("the_elements_of_a_named_kind_are_found_among_that_kind"),
        word: Word::Succeeds,
        input: || Kinds::Named,
        expect: Expect::Value(Observed::Found(FOUND)),
    },
    Case {
        name: CaseName("another_language_does_not_inherit_the_sets_of_the_first"),
        word: Word::Succeeds,
        input: || Kinds::Languages,
        expect: Expect::Value(Observed::Found(LANGUAGES)),
    },
    Case {
        name: CaseName("a_file_with_no_names_teaches_nothing_about_kinds"),
        word: Word::Succeeds,
        input: || Kinds::NoNames,
        expect: Expect::Value(Observed::Counts([0, 1])),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Kinds::Named => finds_the_elements_of_a_named_kind(),
        Kinds::NoNames => a_file_with_no_names_teaches_nothing_about_kinds(),
        Kinds::Languages => another_language_does_not_inherit_the_sets_of_the_first(),
        Kinds::Naming => Ok(names_a_kind_by_a_test_on_its_name()),
        Kinds::Mask(test, text) => masked(test, text),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
