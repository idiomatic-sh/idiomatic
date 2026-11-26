//! The refusals the executor makes before it reads a file: a parameter nothing is bound to, and a word for a kind no linked language has.

use super::refusal::Refusal;
use crate::lower::{SharedPlanData, query_batch};
use ryo_values::domain::finish_plans::types::FinishPlan;
use std::collections::BTreeMap;
use std::sync::Arc;

fn lowered(text: &str) -> Result<Arc<FinishPlan>, Refusal> {
    let parameters = BTreeMap::new();
    query_batch(&[(text, &parameters)], "/", Default::default(), &mut SharedPlanData::default())
        .pop()
        .expect("one answer")
        .map(|(_, typed)| typed)
}

fn toml() -> Vec<(String, Vec<String>)> {
    vec![("toml".to_owned(), vec!["entry".to_owned(), "comment".to_owned()])]
}

fn trees(condition: &str) -> String {
    format!(
        "let rows = select * from \"syntax-tree\" where {condition};\nlet named = extend(from=rows, values={{\"problem\": \"a node\"}});\nemit named subject \"file\" key(\"span\");"
    )
}

#[test]
fn a_parameter_nothing_is_bound_to_is_refused_by_name() {
    let text = "param word: string;\nlet rows = select * from \"comment\" where text = $word;\nlet named = extend(from=rows, values={\"problem\": \"a comment\"});\nemit named subject \"file\" key(\"span\");";
    assert_eq!(lowered(text).err(), Some(Refusal::MissingParameter { name: "word".to_owned() }));
}

#[test]
fn a_word_for_a_kind_no_language_has_is_refused_with_the_languages_looked_in() {
    let plan = lowered(&trees("kind = \"no-such-kind\"")).expect("lowers");
    assert_eq!(
        super::kinds::unknown(&plan, toml),
        Some(Refusal::UnknownKind { word: "no-such-kind".to_owned(), languages: "toml".to_owned() })
    );
}

#[test]
fn a_word_that_names_a_kind_is_not_refused() {
    let plan = lowered(&trees("kind = \"entry\"")).expect("lowers");
    assert_eq!(super::kinds::unknown(&plan, toml), None);
}

#[test]
fn a_pattern_is_checked_against_the_kinds_and_not_as_a_kind() {
    let known = lowered(&trees("glob_any(kind, \"ent*\")")).expect("lowers");
    assert_eq!(super::kinds::unknown(&known, toml), None);
    let unknown = lowered(&trees("glob_any(kind, \"zzz*\")")).expect("lowers");
    assert!(matches!(super::kinds::unknown(&unknown, toml), Some(Refusal::UnknownKind { .. })));
}

#[test]
fn a_query_that_writes_no_kind_asks_no_language() {
    let plan = lowered("let rows = select * from \"comment\";\nlet named = extend(from=rows, values={\"problem\": \"a comment\"});\nemit named subject \"file\" key(\"span\");").expect("lowers");
    assert_eq!(super::kinds::unknown(&plan, || panic!("no language is asked")), None);
}
